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

// ------------------------------------------------------------------ Fiche d'un jeu

#[tauri::command]
pub async fn get_game_detail(id: i64, db: State<'_, AppDb>) -> Result<Option<crate::detail::GameDetail>, String> {
    let conn = db.0.lock().map_err(err)?;
    crate::detail::build(&conn, id, db::now()).map_err(err)
}

/// Exécute une modification, vérifie qu'elle a touché quelque chose, puis prévient l'interface.
fn modify(
    app: &AppHandle,
    db: &AppDb,
    f: impl FnOnce(&Connection) -> rusqlite::Result<bool>,
) -> Result<(), String> {
    let changed = {
        let conn = db.0.lock().map_err(err)?;
        f(&conn).map_err(err)?
    };
    if !changed {
        return Err("Élément introuvable (déjà supprimé ?).".into());
    }
    let _ = app.emit(crate::EVENT_CHANGED, ());
    Ok(())
}

fn clean_name(name: &str) -> Result<String, String> {
    let name = name.trim();
    if name.is_empty() {
        return Err("Donne un nom au jeu.".into());
    }
    if name.chars().count() > 80 {
        return Err("Le nom est trop long (80 caractères max).".into());
    }
    Ok(name.to_string())
}

/// Texte facultatif : vide → aucun ; tronqué à `max` caractères.
fn optional_text(s: Option<String>, max: usize) -> Option<String> {
    s.map(|t| t.trim().chars().take(max).collect::<String>()).filter(|t| !t.is_empty())
}

#[tauri::command]
pub async fn rename_game(id: i64, name: String, app: AppHandle, db: State<'_, AppDb>) -> Result<(), String> {
    let name = clean_name(&name)?;
    modify(&app, &db, |c| db::rename_game(c, id, &name))
}

#[tauri::command]
pub async fn set_baseline(id: i64, secs: i64, app: AppHandle, db: State<'_, AppDb>) -> Result<(), String> {
    if !(0..=MAX_BASELINE_SECS).contains(&secs) {
        return Err("Temps de départ invalide.".into());
    }
    modify(&app, &db, |c| db::set_baseline(c, id, secs))
}

#[tauri::command]
pub async fn set_weekly_limit(id: i64, secs: Option<i64>, app: AppHandle, db: State<'_, AppDb>) -> Result<(), String> {
    let secs = secs.filter(|s| *s > 0);
    if secs.is_some_and(|s| s > 7 * 24 * 3600) {
        return Err("Une semaine ne compte que 168 heures.".into());
    }
    modify(&app, &db, |c| db::set_weekly_limit(c, id, secs))
}

#[tauri::command]
pub async fn archive_game(id: i64, app: AppHandle, db: State<'_, AppDb>) -> Result<(), String> {
    modify(&app, &db, |c| db::archive_game(c, id))
}

#[tauri::command]
pub async fn update_session(
    id: i64,
    tag: Option<String>,
    note: Option<String>,
    app: AppHandle,
    db: State<'_, AppDb>,
) -> Result<(), String> {
    let tag = optional_text(tag, 30);
    let note = optional_text(note, 200);
    modify(&app, &db, |c| db::update_session(c, id, tag.as_deref(), note.as_deref()))
}

#[tauri::command]
pub async fn delete_session(id: i64, app: AppHandle, db: State<'_, AppDb>) -> Result<(), String> {
    modify(&app, &db, |c| db::delete_session(c, id))
}

// ------------------------------------------------------------------ Bureau

#[tauri::command]
pub async fn set_tracking_paused(paused: bool, app: AppHandle) -> Result<(), String> {
    crate::desktop::set_paused(&app, paused);
    Ok(())
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StartupSettings {
    pub autostart: bool,
    pub notifications: bool,
}

#[tauri::command]
pub async fn get_startup_settings(app: AppHandle, db: State<'_, AppDb>) -> Result<StartupSettings, String> {
    use tauri_plugin_autostart::ManagerExt;
    // L'état réel du lancement auto est lu dans Windows (source de vérité).
    let autostart = app.autolaunch().is_enabled().unwrap_or(false);
    let conn = db.0.lock().map_err(err)?;
    let notifications = Settings::load(&conn).map_err(err)?.notifications;
    Ok(StartupSettings { autostart, notifications })
}

#[tauri::command]
pub async fn set_autostart(enabled: bool, app: AppHandle, db: State<'_, AppDb>) -> Result<(), String> {
    use tauri_plugin_autostart::ManagerExt;
    let manager = app.autolaunch();
    let result = if enabled { manager.enable() } else { manager.disable() };
    result.map_err(|e| format!("Impossible de modifier le lancement au démarrage : {e}"))?;
    let conn = db.0.lock().map_err(err)?;
    db::set_setting(&conn, "autostart", if enabled { "true" } else { "false" }).map_err(err)
}

#[tauri::command]
pub async fn set_notifications(enabled: bool, db: State<'_, AppDb>) -> Result<(), String> {
    let conn = db.0.lock().map_err(err)?;
    db::set_setting(&conn, "notifications", if enabled { "true" } else { "false" }).map_err(err)
}

// ------------------------------------------------------------------ Paramètres et sauvegardes

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsView {
    pub settings: Settings,
    /// État réel du lancement avec Windows.
    pub autostart: bool,
    pub last_auto_backup_at: Option<i64>,
    pub suggested_backup_name: String,
}

#[tauri::command]
pub async fn get_settings(app: AppHandle, db: State<'_, AppDb>) -> Result<SettingsView, String> {
    use tauri_plugin_autostart::ManagerExt;
    let autostart = app.autolaunch().is_enabled().unwrap_or(false);
    let conn = db.0.lock().map_err(err)?;
    let settings = Settings::load(&conn).map_err(err)?;
    let last_auto_backup_at = db::get_setting(&conn, "last_auto_backup_at").map_err(err)?.and_then(|v| v.parse().ok());
    Ok(SettingsView {
        settings,
        autostart,
        last_auto_backup_at,
        suggested_backup_name: crate::backup::suggested_name(db::now()),
    })
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrackingSettings {
    pub min_session_secs: i64,
    pub afk_threshold_secs: i64,
    pub merge_window_secs: i64,
}

#[tauri::command]
pub async fn update_tracking_settings(s: TrackingSettings, db: State<'_, AppDb>) -> Result<(), String> {
    if !(0..=3600).contains(&s.min_session_secs) {
        return Err("Durée minimale : entre 0 et 60 minutes.".into());
    }
    if s.afk_threshold_secs != 0 && !(60..=7200).contains(&s.afk_threshold_secs) {
        return Err("Seuil AFK : entre 1 et 120 minutes (ou désactivé).".into());
    }
    if !(0..=3600).contains(&s.merge_window_secs) {
        return Err("Fusion : entre 0 et 60 minutes.".into());
    }
    let conn = db.0.lock().map_err(err)?;
    db::set_setting(&conn, "min_session_secs", &s.min_session_secs.to_string()).map_err(err)?;
    db::set_setting(&conn, "afk_threshold_secs", &s.afk_threshold_secs.to_string()).map_err(err)?;
    db::set_setting(&conn, "merge_window_secs", &s.merge_window_secs.to_string()).map_err(err)
}

/// Dossier de sauvegarde automatique (`None` : désactivée).
#[tauri::command]
pub async fn set_backup_dir(path: Option<String>, db: State<'_, AppDb>) -> Result<(), String> {
    let conn = db.0.lock().map_err(err)?;
    match path.filter(|p| !p.trim().is_empty()) {
        Some(p) => {
            if !Path::new(&p).is_dir() {
                return Err("Ce dossier n'existe pas.".into());
            }
            db::set_setting(&conn, "backup_dir", &p).map_err(err)?;
            // Première sauvegarde tout de suite, pour vérifier que le dossier est utilisable.
            crate::backup::run_auto_backup(&conn, Path::new(&p), db::now()).map(|_| ())
        }
        None => db::set_setting(&conn, "backup_dir", "").map_err(err),
    }
}

#[tauri::command]
pub async fn export_backup(path: String, db: State<'_, AppDb>) -> Result<(), String> {
    let conn = db.0.lock().map_err(err)?;
    crate::backup::backup_to(&conn, Path::new(&path))
}

#[tauri::command]
pub async fn import_backup(path: String, app: AppHandle, db: State<'_, AppDb>) -> Result<crate::backup::ImportSummary, String> {
    let summary = {
        let mut conn = db.0.lock().map_err(err)?;
        let db_path = db::default_db_path();
        let safety_dir = db_path.parent().map(|p| p.join("avant-import")).unwrap_or_else(|| db::data_dir().join("avant-import"));
        crate::backup::import_from(&mut conn, Path::new(&path), db::now(), &safety_dir)?
    };
    let _ = app.emit(crate::EVENT_CHANGED, ());
    Ok(summary)
}

#[tauri::command]
pub async fn get_overview(db: State<'_, AppDb>) -> Result<crate::detail::Overview, String> {
    let conn = db.0.lock().map_err(err)?;
    crate::detail::overview(&conn, db::now()).map_err(err)
}

/// Icône d'un jeu suivi, en « data URL » PNG.
#[tauri::command]
pub async fn get_game_icon(id: i64, db: State<'_, AppDb>) -> Result<Option<String>, String> {
    let conn = db.0.lock().map_err(err)?;
    Ok(db::game_icon(&conn, id).map_err(err)?.map(|png| exeinfo::png_data_url(&png)))
}
