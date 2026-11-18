//! Réglages de l'application, stockés dans la table `settings`.

use crate::db;
use rusqlite::{Connection, Result};

#[derive(Debug, Clone, PartialEq)]
pub struct Settings {
    /// Intervalle entre deux vérifications des processus.
    pub check_interval_secs: i64,
    /// Une session plus courte que ça est ignorée.
    pub min_session_secs: i64,
    /// Sans clavier/souris/manette pendant ce temps, on considère le joueur AFK.
    pub afk_threshold_secs: i64,
    /// Relance du même jeu dans ce délai = même session. 0 = pas de fusion.
    pub merge_window_secs: i64,
    pub autostart: bool,
    pub notifications: bool,
    pub theme: String,
    pub backup_dir: Option<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            check_interval_secs: 5,
            min_session_secs: 120,
            afk_threshold_secs: 600,
            merge_window_secs: 300,
            autostart: false,
            notifications: true,
            theme: "dark".into(),
            backup_dir: None,
        }
    }
}

/// Noms des clés, utilisés aussi par l'outil de debug.
pub const KEYS: &[&str] = &[
    "check_interval_secs",
    "min_session_secs",
    "afk_threshold_secs",
    "merge_window_secs",
    "autostart",
    "notifications",
    "theme",
    "backup_dir",
];

impl Settings {
    pub fn load(conn: &Connection) -> Result<Settings> {
        let d = Settings::default();
        let int = |k: &str, def: i64| -> Result<i64> {
            Ok(db::get_setting(conn, k)?
                .and_then(|v| v.parse().ok())
                .unwrap_or(def))
        };
        let boolean = |k: &str, def: bool| -> Result<bool> {
            Ok(db::get_setting(conn, k)?.map(|v| v == "true").unwrap_or(def))
        };
        Ok(Settings {
            // Bornes de sécurité : jamais moins de 2 s entre deux vérifications.
            check_interval_secs: int("check_interval_secs", d.check_interval_secs)?.clamp(2, 60),
            min_session_secs: int("min_session_secs", d.min_session_secs)?.max(0),
            afk_threshold_secs: int("afk_threshold_secs", d.afk_threshold_secs)?.max(0),
            merge_window_secs: int("merge_window_secs", d.merge_window_secs)?.max(0),
            autostart: boolean("autostart", d.autostart)?,
            notifications: boolean("notifications", d.notifications)?,
            theme: db::get_setting(conn, "theme")?.unwrap_or(d.theme),
            backup_dir: db::get_setting(conn, "backup_dir")?.filter(|s| !s.is_empty()),
        })
    }
}
