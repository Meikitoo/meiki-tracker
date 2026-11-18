//! Sauvegardes : export manuel, import, et sauvegarde automatique hebdomadaire.
//!
//! Une sauvegarde est une copie complète et cohérente de la base SQLite
//! (`VACUUM INTO`), avec l'extension `.meikibackup`.

use crate::db;
use rusqlite::{Connection, OpenFlags, OptionalExtension};
use std::path::{Path, PathBuf};

pub const EXTENSION: &str = "meikibackup";
const AUTO_PREFIX: &str = "meiki-auto-";
/// Nombre de sauvegardes automatiques gardées dans le dossier choisi.
const AUTO_KEEP: usize = 8;
const WEEK_SECS: i64 = 7 * 24 * 3600;

/// Réglages propres à ce PC, jamais écrasés par un import.
const MACHINE_KEYS: &[&str] = &["autostart", "backup_dir", "last_auto_backup_at", "tray_hint_shown"];

/// Écrit une copie complète de la base dans `dest` (remplacée si elle existe).
pub fn backup_to(conn: &Connection, dest: &Path) -> Result<(), String> {
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("Dossier inaccessible : {e}"))?;
    }
    if dest.exists() {
        std::fs::remove_file(dest).map_err(|e| format!("Impossible de remplacer le fichier : {e}"))?;
    }
    conn.execute("VACUUM INTO ?1", [dest.to_string_lossy()])
        .map_err(|e| format!("Sauvegarde impossible : {e}"))?;
    Ok(())
}

fn stamp(ts: i64, with_time: bool) -> String {
    use chrono::{Local, TimeZone};
    let d = Local.timestamp_opt(ts, 0).single().unwrap_or_else(Local::now);
    d.format(if with_time { "%Y-%m-%d_%H-%M-%S" } else { "%Y-%m-%d" }).to_string()
}

/// Nom proposé pour un export manuel.
pub fn suggested_name(now: i64) -> String {
    format!("meiki-tracker-{}.{EXTENSION}", stamp(now, false))
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportSummary {
    pub games: i64,
    pub sessions: i64,
    /// Copie des données d'avant l'import (au cas où).
    pub safety_backup: String,
}

/// Vérifie qu'un fichier est bien une sauvegarde Meiki Tracker lisible.
fn check_backup(path: &Path) -> Result<(i64, i64), String> {
    let bad = || "Ce fichier n'est pas une sauvegarde Meiki Tracker valide.".to_string();
    let src = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY).map_err(|_| bad())?;
    let version: i32 = src.pragma_query_value(None, "user_version", |r| r.get(0)).map_err(|_| bad())?;
    if version > db::SCHEMA_VERSION {
        return Err("Cette sauvegarde vient d'une version plus récente de Meiki Tracker : mets l'appli à jour.".into());
    }
    if version < 1 {
        return Err(bad());
    }
    let count = |t: &str| -> Result<i64, String> {
        src.query_row(&format!("SELECT COUNT(*) FROM {t}"), [], |r| r.get(0)).map_err(|_| bad())
    };
    Ok((count("games")?, count("sessions")?))
}

/// Remplace toutes les données par celles de la sauvegarde.
/// Refusé si un jeu est en cours (sa session serait perdue).
/// `safety_dir` reçoit une copie des données actuelles avant remplacement.
pub fn import_from(conn: &mut Connection, src: &Path, now: i64, safety_dir: &Path) -> Result<ImportSummary, String> {
    let (games, sessions) = check_backup(src)?;
    let running: Option<i64> = conn
        .query_row("SELECT game_id FROM live_sessions WHERE ended_at IS NULL LIMIT 1", [], |r| r.get(0))
        .optional()
        .map_err(|e| e.to_string())?;
    if running.is_some() {
        return Err("Un jeu est en cours : ferme-le avant d'importer une sauvegarde.".into());
    }

    // Filet de sécurité : copie des données actuelles avant de les remplacer.
    let safety = safety_dir.join(format!("avant-import-{}.{EXTENSION}", stamp(now, true)));
    backup_to(conn, &safety)?;

    conn.execute("ATTACH DATABASE ?1 AS imp", [src.to_string_lossy()]).map_err(|e| e.to_string())?;
    let result = (|| -> rusqlite::Result<()> {
        let keys = MACHINE_KEYS.iter().map(|k| format!("'{k}'")).collect::<Vec<_>>().join(",");
        let tx = conn.transaction()?;
        tx.execute_batch(&format!(
            "DELETE FROM live_sessions;
             DELETE FROM sessions;
             DELETE FROM games;
             DELETE FROM settings WHERE key NOT IN ({keys});
             INSERT INTO games (id, name, process_name, category, baseline_secs, baseline_set_at,
                                weekly_limit_secs, icon_png, added_at, archived)
               SELECT id, name, process_name, category, baseline_secs, baseline_set_at,
                      weekly_limit_secs, icon_png, added_at, archived FROM imp.games;
             INSERT INTO sessions (id, game_id, started_at, ended_at, duration_secs, afk_secs, gap_secs, tag, note)
               SELECT id, game_id, started_at, ended_at, duration_secs, afk_secs, gap_secs, tag, note FROM imp.sessions;
             INSERT OR REPLACE INTO settings (key, value)
               SELECT key, value FROM imp.settings WHERE key NOT IN ({keys});"
        ))?;
        tx.commit()
    })();
    let _ = conn.execute("DETACH DATABASE imp", []);
    result.map_err(|e| format!("Import impossible (tes données n'ont pas été modifiées) : {e}"))?;

    Ok(ImportSummary { games, sessions, safety_backup: safety.to_string_lossy().to_string() })
}

/// Sauvegarde automatique si un dossier est choisi et que la dernière date de plus d'une semaine.
/// Renvoie le fichier créé, s'il y en a un.
pub fn maybe_auto_backup(conn: &Connection, now: i64) -> Result<Option<PathBuf>, String> {
    let Some(dir) = db::get_setting(conn, "backup_dir").map_err(|e| e.to_string())?.filter(|d| !d.is_empty()) else {
        return Ok(None);
    };
    let last: i64 = db::get_setting(conn, "last_auto_backup_at")
        .map_err(|e| e.to_string())?
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
    if now - last < WEEK_SECS {
        return Ok(None);
    }
    run_auto_backup(conn, Path::new(&dir), now).map(Some)
}

/// Crée tout de suite une sauvegarde dans le dossier automatique et nettoie les plus anciennes.
pub fn run_auto_backup(conn: &Connection, dir: &Path, now: i64) -> Result<PathBuf, String> {
    let dest = dir.join(format!("{AUTO_PREFIX}{}.{EXTENSION}", stamp(now, false)));
    backup_to(conn, &dest)?;
    db::set_setting(conn, "last_auto_backup_at", &now.to_string()).map_err(|e| e.to_string())?;
    prune(dir);
    Ok(dest)
}

/// Garde les `AUTO_KEEP` sauvegardes automatiques les plus récentes (on ne touche
/// qu'aux fichiers créés par l'appli : « meiki-auto-AAAA-MM-JJ.meikibackup »).
fn prune(dir: &Path) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    let mut autos: Vec<PathBuf> = entries
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with(AUTO_PREFIX) && n.ends_with(&format!(".{EXTENSION}")))
        })
        .collect();
    autos.sort(); // la date dans le nom rend l'ordre alphabétique chronologique
    let excess = autos.len().saturating_sub(AUTO_KEEP);
    for old in autos.into_iter().take(excess) {
        let _ = std::fs::remove_file(old);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("meiki-test-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn sample(conn: &Connection, name: &str, secs: i64) {
        let id = db::add_game(conn, name, &format!("{name}.exe"), "game", 0).unwrap();
        db::insert_session(conn, &db::Session {
            id: 0, game_id: id, started_at: 0, ended_at: secs, duration_secs: secs,
            afk_secs: 0, gap_secs: 0, tag: Some("Fun".into()), note: None,
        }).unwrap();
    }

    #[test]
    fn export_puis_import_restaure_les_donnees() {
        let dir = tmp("roundtrip");
        let src = db::open(&dir.join("a.db")).unwrap();
        sample(&src, "cod", 3600);
        db::set_setting(&src, "min_session_secs", "300").unwrap();
        db::set_setting(&src, "backup_dir", "C:/autre-pc").unwrap();
        let file = dir.join("save.meikibackup");
        backup_to(&src, &file).unwrap();

        let mut dst = db::open(&dir.join("b.db")).unwrap();
        sample(&dst, "valorant", 60);
        db::set_setting(&dst, "backup_dir", "D:/mes-sauvegardes").unwrap();
        let s = import_from(&mut dst, &file, 1_000_000, &dir.join("securite")).unwrap();
        assert_eq!((s.games, s.sessions), (1, 1));
        assert!(Path::new(&s.safety_backup).exists(), "copie de sécurité créée");

        let games = db::active_games(&dst).unwrap();
        assert_eq!(games.len(), 1);
        assert_eq!(games[0].process_name, "cod.exe");
        assert_eq!(db::tracked_secs(&dst, games[0].id).unwrap(), 3600);
        assert_eq!(db::get_setting(&dst, "min_session_secs").unwrap().as_deref(), Some("300"));
        assert_eq!(db::get_setting(&dst, "backup_dir").unwrap().as_deref(), Some("D:/mes-sauvegardes"), "réglage du PC conservé");
    }

    #[test]
    fn import_refuse_fichier_invalide_et_jeu_en_cours() {
        let dir = tmp("invalid");
        let junk = dir.join("pas-une-base.meikibackup");
        std::fs::write(&junk, b"bonjour").unwrap();
        let mut conn = db::open(&dir.join("c.db")).unwrap();
        let safety = dir.join("securite");
        assert!(import_from(&mut conn, &junk, 0, &safety).unwrap_err().contains("pas une sauvegarde"));

        let file = dir.join("ok.meikibackup");
        backup_to(&conn, &file).unwrap();
        let g = db::add_game(&conn, "cod", "cod.exe", "game", 0).unwrap();
        db::save_live(&conn, &db::LiveSession::new(g, 0)).unwrap();
        assert!(import_from(&mut conn, &file, 0, &safety).unwrap_err().contains("en cours"));
        assert_eq!(db::active_games(&conn).unwrap().len(), 1, "rien n'a été modifié");
    }

    #[test]
    fn sauvegarde_auto_hebdo_et_nettoyage() {
        let dir = tmp("auto");
        let backups = dir.join("sauvegardes");
        let conn = db::open(&dir.join("d.db")).unwrap();
        assert_eq!(maybe_auto_backup(&conn, 0).unwrap(), None, "pas de dossier choisi");
        db::set_setting(&conn, "backup_dir", &backups.to_string_lossy()).unwrap();

        let week = WEEK_SECS;
        let first = 1_800_000_000;
        assert!(maybe_auto_backup(&conn, first).unwrap().is_some());
        assert!(maybe_auto_backup(&conn, first + 3600).unwrap().is_none(), "moins d'une semaine");
        for i in 1..=10 {
            assert!(maybe_auto_backup(&conn, first + i * week).unwrap().is_some());
        }
        std::fs::write(backups.join("ma-note.txt"), b"perso").unwrap();
        prune(&backups);
        let names: Vec<_> = std::fs::read_dir(&backups).unwrap().map(|e| e.unwrap().file_name()).collect();
        assert_eq!(names.iter().filter(|n| n.to_string_lossy().starts_with(AUTO_PREFIX)).count(), AUTO_KEEP);
        assert!(backups.join("ma-note.txt").exists(), "les autres fichiers ne sont jamais touchés");
    }
}
