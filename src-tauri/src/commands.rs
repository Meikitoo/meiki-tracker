//! Commandes appelables depuis l'interface (via `invoke` côté TypeScript).

use crate::db;
use crate::library::{self, Library};
use crate::service::TrackerControl;
use crate::settings::Settings;
use rusqlite::Connection;
use std::sync::atomic::Ordering;
use std::sync::Mutex;
use tauri::State;

/// Connexion réservée à l'interface (le tracking a la sienne, dans son thread).
pub struct AppDb(pub Mutex<Connection>);

fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}

#[tauri::command]
pub fn get_library(db: State<'_, AppDb>, control: State<'_, TrackerControl>) -> Result<Library, String> {
    let conn = db.0.lock().map_err(err)?;
    let settings = Settings::load(&conn).map_err(err)?;
    library::build(
        &conn,
        db::now(),
        control.paused.load(Ordering::Relaxed),
        settings.check_interval_secs,
    )
    .map_err(err)
}
