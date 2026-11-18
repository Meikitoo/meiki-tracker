//! Point d'entrée de l'application Meiki Tracker (côté Rust).

pub mod commands;
pub mod db;
pub mod detail;
pub mod exclusions;
pub mod exeinfo;
pub mod library;
pub mod service;
pub mod settings;
pub mod system;
pub mod tracker;

use service::TrackerControl;
use std::sync::atomic::Ordering;
use std::sync::Mutex;
use tauri::{Emitter, Manager};

/// Événements envoyés à l'interface.
pub const EVENT_CHANGED: &str = "tracker-changed"; // une session a démarré/fini → recharger
pub const EVENT_TICK: &str = "tracker-tick"; // à chaque vérification → chrono en direct

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(commands::ExeCache::default())
        .setup(|app| {
            let path = db::default_db_path();
            // Ouvrir la base ici d'abord garantit que le schéma existe avant le thread de suivi.
            let conn = db::open(&path)?;
            app.manage(commands::AppDb(Mutex::new(conn)));

            let control = TrackerControl::default();
            app.manage(control.clone());

            let handle = app.handle().clone();
            let paused = control.paused.clone();
            // Le suivi tourne dans son propre thread, indépendamment de la fenêtre.
            service::spawn(path, control, move |conn, events| {
                for e in events {
                    println!("[tracker] {}", service::describe(conn, e));
                }
                if !events.is_empty() {
                    let _ = handle.emit(EVENT_CHANGED, ());
                }
                let now = db::now();
                if let Ok(live) = library::live_infos(conn, now) {
                    let tick = library::LiveTick { live, paused: paused.load(Ordering::Relaxed), at: now };
                    let _ = handle.emit(EVENT_TICK, tick);
                }
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_library,
            commands::list_running_programs,
            commands::describe_exe,
            commands::add_game,
            commands::get_game_icon,
            commands::get_game_detail,
            commands::rename_game,
            commands::set_baseline,
            commands::set_weekly_limit,
            commands::archive_game,
            commands::update_session,
            commands::delete_session,
        ])
        .run(tauri::generate_context!())
        .expect("Erreur au lancement de Meiki Tracker");
}
