//! Point d'entrée de l'application Meiki Tracker (côté Rust).

pub mod backup;
pub mod commands;
pub mod db;
pub mod desktop;
pub mod detail;
pub mod exclusions;
pub mod exeinfo;
pub mod i18n;
pub mod library;
pub mod recap;
pub mod service;
pub mod settings;
pub mod steam;
pub mod system;
pub mod tracker;

use service::TrackerControl;
use std::sync::atomic::Ordering;
use std::sync::Mutex;
use tauri::{Emitter, Manager, WindowEvent};
use tauri_plugin_autostart::MacosLauncher;

/// Événements envoyés à l'interface.
pub const EVENT_CHANGED: &str = "tracker-changed"; // une session a démarré/fini → recharger
pub const EVENT_TICK: &str = "tracker-tick"; // à chaque vérification → chrono en direct

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let mut builder = tauri::Builder::default();
    // En développement avec une base de test (MEIKI_DB), on autorise une 2e instance :
    // l'appli installée peut tourner à côté sans être dérangée.
    let test_instance = cfg!(debug_assertions) && std::env::var_os("MEIKI_DB").is_some_and(|v| !v.is_empty());
    if !test_instance {
        // Doit être le premier plugin : si l'appli tourne déjà, on rouvre sa fenêtre
        // au lieu de lancer un deuxième suivi.
        builder = builder.plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            desktop::show_main_window(app);
        }));
    }
    builder
        .plugin(tauri_plugin_autostart::init(MacosLauncher::LaunchAgent, Some(vec![desktop::ARG_MINIMIZED])))
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .manage(commands::ExeCache::default())
        .manage(desktop::TrayTooltip::default())
        .setup(|app| {
            let path = db::default_db_path();
            // Ouvrir la base ici d'abord garantit que le schéma existe avant le thread de suivi.
            let conn = db::open(&path)?;
            i18n::load(&conn);
            app.manage(commands::AppDb(Mutex::new(conn)));

            let control = TrackerControl::default();
            app.manage(control.clone());

            desktop::create_tray(app.handle())?;

            // Lancé par Windows au démarrage : on reste discret dans la zone de notification.
            let minimized = std::env::args().any(|a| a == desktop::ARG_MINIMIZED);
            if !minimized {
                desktop::show_main_window(app.handle());
            }

            desktop::spawn_steam_startup_sync(app.handle().clone(), path.clone());

            let handle = app.handle().clone();
            let paused = control.paused.clone();
            // Le suivi tourne dans son propre thread, indépendamment de la fenêtre.
            service::spawn(path, control, move |conn, events| {
                for e in events {
                    println!("[tracker] {}", service::describe(conn, e));
                }
                if !events.is_empty() {
                    desktop::on_tracker_events(&handle, conn, events);
                    let _ = handle.emit(EVENT_CHANGED, ());
                }
                let now = db::now();
                let is_paused = paused.load(Ordering::Relaxed);
                if let Ok(live) = library::live_infos(conn, now) {
                    let names: Vec<String> = live.iter().map(|l| l.name.clone()).collect();
                    desktop::update_tooltip(&handle, &names, is_paused);
                    let tick = library::LiveTick { live, paused: is_paused, at: now };
                    let _ = handle.emit(EVENT_TICK, tick);
                }
            });
            Ok(())
        })
        .on_window_event(|window, event| {
            // Fermer la fenêtre ne quitte pas l'appli : le suivi continue en fond.
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                desktop::on_close_requested(window.app_handle());
            }
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
            commands::set_tracking_paused,
            commands::set_autostart,
            commands::set_notifications,
            commands::get_settings,
            commands::update_tracking_settings,
            commands::set_backup_dir,
            commands::export_backup,
            commands::import_backup,
            commands::get_overview,
            commands::set_language,
            commands::set_update_check,
            commands::get_recap,
            commands::save_png,
            commands::get_steam_settings,
            commands::set_steam_enabled,
            commands::save_steam_account,
            commands::forget_steam_account,
            commands::steam_sync_all,
            commands::steam_sync_game,
            commands::get_achievements,
            commands::steam_link_candidates,
            commands::link_steam_game,
        ])
        .run(tauri::generate_context!())
        .expect("Erreur au lancement de Meiki Tracker");
}
