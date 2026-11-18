//! Commandes appelables depuis l'interface (via `invoke` côté TypeScript).
//!
//! Les commandes un peu lourdes sont `async` : Tauri les exécute alors hors du
//! fil principal, et la fenêtre ne se fige jamais.

use crate::db::{self, AddOutcome};
use crate::exeinfo;
use crate::library::{self, Library};
use crate::service::TrackerControl;
use crate::settings::Settings;
use crate::system::ProcessScanner;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::atomic::Ordering;
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, State};

/// Connexion réservée à l'interface (le tracking a la sienne, dans son thread).
pub struct AppDb(pub Mutex<Connection>);

/// Cache des infos lues dans les .exe (icône, nom), par chemin.
#[derive(Default)]
pub struct ExeCache(Mutex<HashMap<String, ExeInfo>>);

#[derive(Clone)]
struct ExeInfo {
    description: Option<String>,
    icon: Option<String>,
}

fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}

#[tauri::command]
pub async fn get_library(db: State<'_, AppDb>, control: State<'_, TrackerControl>) -> Result<Library, String> {
    let conn = db.0.lock().map_err(err)?;
    let settings = Settings::load(&conn).map_err(err)?;
    library::build(&conn, db::now(), control.paused.load(Ordering::Relaxed), settings.check_interval_secs)
        .map_err(err)
}

/// Un programme proposé dans « Ajouter un jeu ».
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProgramCandidate {
    pub process_name: String,
    pub exe_path: Option<String>,
    /// Nom proposé par défaut (description du fichier, sinon nom du .exe).
    pub display_name: String,
    /// Icône en « data URL » PNG.
    pub icon: Option<String>,
    pub windowed: bool,
    pub already_tracked: bool,
}

fn exe_info(cache: &ExeCache, path: &str) -> ExeInfo {
    if let Some(i) = cache.0.lock().ok().and_then(|c| c.get(path).cloned()) {
        return i;
    }
    let p = Path::new(path);
    let info = ExeInfo {
        description: exeinfo::file_description(p),
        icon: exeinfo::icon_png(p).map(|png| exeinfo::png_data_url(&png)),
    };
    if let Ok(mut c) = cache.0.lock() {
        c.insert(path.to_string(), info.clone());
    }
    info
}

fn candidate(cache: &ExeCache, process_name: String, exe_path: Option<String>, windowed: bool, tracked: &HashSet<String>) -> ProgramCandidate {
    let info = exe_path.as_deref().map(|p| exe_info(cache, p));
    let display_name = info
        .as_ref()
        .and_then(|i| i.description.clone())
        .unwrap_or_else(|| exeinfo::name_from_file(&process_name));
    ProgramCandidate {
        already_tracked: tracked.contains(&process_name.to_lowercase()),
        icon: info.and_then(|i| i.icon),
        process_name,
        exe_path,
        display_name,
        windowed,
    }
}

fn tracked_names(db: &AppDb) -> Result<HashSet<String>, String> {
    let conn = db.0.lock().map_err(err)?;
    Ok(db::active_games(&conn).map_err(err)?.into_iter().map(|g| g.process_name.to_lowercase()).collect())
}

/// Programmes ouverts en ce moment, hors launchers/système.
/// Par défaut, seulement ceux qui ont une fenêtre (les jeux en ont toujours une).
#[tauri::command]
pub async fn list_running_programs(
    include_background: bool,
    db: State<'_, AppDb>,
    cache: State<'_, ExeCache>,
) -> Result<Vec<ProgramCandidate>, String> {
    let tracked = tracked_names(&db)?;
    let mut list: Vec<ProgramCandidate> = ProcessScanner::new()
        .candidate_programs()
        .into_iter()
        .filter(|p| include_background || p.windowed)
        .map(|p| candidate(&cache, p.process_name, p.exe_path, p.windowed, &tracked))
        .collect();
    list.sort_by(|a, b| {
        b.windowed
            .cmp(&a.windowed)
            .then_with(|| a.display_name.to_lowercase().cmp(&b.display_name.to_lowercase()))
    });
    Ok(list)
}

/// Infos d'un .exe choisi avec « Parcourir… ».
#[tauri::command]
pub async fn describe_exe(path: String, db: State<'_, AppDb>, cache: State<'_, ExeCache>) -> Result<ProgramCandidate, String> {
    let p = Path::new(&path);
    let file = p
        .file_name()
        .map(|f| f.to_string_lossy().to_string())
        .filter(|f| f.to_lowercase().ends_with(".exe"))
        .ok_or("Choisis un fichier .exe")?;
    let tracked = tracked_names(&db)?;
    Ok(candidate(&cache, file, Some(path), false, &tracked))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewGame {
    pub process_name: String,
    pub name: String,
    pub category: String,
    pub baseline_secs: i64,
    pub exe_path: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AddResult {
    pub id: i64,
    pub restored: bool,
}

/// Plafond du temps de départ : 100 000 h (au-delà, c'est forcément une faute de frappe).
const MAX_BASELINE_SECS: i64 = 100_000 * 3600;

#[tauri::command]
pub async fn add_game(game: NewGame, app: AppHandle, db: State<'_, AppDb>) -> Result<AddResult, String> {
    let name = game.name.trim();
    let process = game.process_name.trim();
    if name.is_empty() {
        return Err("Donne un nom au jeu.".into());
    }
    if name.chars().count() > 80 {
        return Err("Le nom est trop long (80 caractères max).".into());
    }
    if !process.to_lowercase().ends_with(".exe") {
        return Err("Programme invalide.".into());
    }
    if !matches!(game.category.as_str(), "game" | "creation") {
        return Err("Catégorie invalide.".into());
    }
    if !(0..=MAX_BASELINE_SECS).contains(&game.baseline_secs) {
        return Err("Temps de départ invalide.".into());
    }
    let icon = game.exe_path.as_deref().and_then(|p| exeinfo::icon_png(Path::new(p)));

    let outcome = {
        let conn = db.0.lock().map_err(err)?;
        db::add_or_restore_game(&conn, name, process, &game.category, game.baseline_secs, icon.as_deref())
            .map_err(err)?
    };
    let result = match outcome {
        AddOutcome::Added(id) => AddResult { id, restored: false },
        AddOutcome::Restored(id) => AddResult { id, restored: true },
        AddOutcome::AlreadyTracked(_) => return Err("Ce jeu est déjà dans ta bibliothèque.".into()),
    };
    let _ = app.emit(crate::EVENT_CHANGED, ());
    Ok(result)
}

/// Icône d'un jeu suivi, en « data URL » PNG.
#[tauri::command]
pub async fn get_game_icon(id: i64, db: State<'_, AppDb>) -> Result<Option<String>, String> {
    let conn = db.0.lock().map_err(err)?;
    Ok(db::game_icon(&conn, id).map_err(err)?.map(|png| exeinfo::png_data_url(&png)))
}
