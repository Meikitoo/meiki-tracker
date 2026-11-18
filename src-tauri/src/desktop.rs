//! Intégration au bureau Windows : icône dans la zone de notification (près de
//! l'horloge), fenêtre qui se cache au lieu de se fermer, pause du tracking,
//! notifications de fin de session.

use crate::db;
use crate::service::{self, TrackerControl};
use crate::settings::Settings;
use crate::tracker::TrackerEvent;
use rusqlite::Connection;
use std::sync::atomic::Ordering;
use std::sync::Mutex;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, Wry};
use tauri_plugin_notification::NotificationExt;

pub const TRAY_ID: &str = "main";
/// Argument passé par le lancement automatique avec Windows : démarrer caché.
pub const ARG_MINIMIZED: &str = "--minimized";

/// Élément « Mettre en pause / Reprendre » du menu, gardé pour changer son texte.
struct PauseItem(MenuItem<Wry>);

/// Dernière infobulle affichée (évite de la redéfinir toutes les 5 s).
#[derive(Default)]
pub struct TrayTooltip(Mutex<String>);

fn pause_label(paused: bool) -> &'static str {
    if paused { "Reprendre le tracking" } else { "Mettre en pause le tracking" }
}

pub fn show_main_window(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.unminimize();
        let _ = w.show();
        let _ = w.set_focus();
    }
}

pub fn set_paused(app: &AppHandle, paused: bool) {
    let control = app.state::<TrackerControl>();
    control.paused.store(paused, Ordering::Relaxed);
    if let Some(item) = app.try_state::<PauseItem>() {
        let _ = item.0.set_text(pause_label(paused));
    }
    // L'interface recharge et affiche le nouvel état.
    let _ = app.emit(crate::EVENT_CHANGED, ());
}

pub fn create_tray(app: &AppHandle) -> tauri::Result<()> {
    let open = MenuItem::with_id(app, "open", "Ouvrir Meiki Tracker", true, None::<&str>)?;
    let pause = MenuItem::with_id(app, "pause", pause_label(false), true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quitter", true, None::<&str>)?;
    let sep = PredefinedMenuItem::separator(app)?;
    let menu = Menu::with_items(app, &[&open, &pause, &sep, &quit])?;
    app.manage(PauseItem(pause));

    let mut builder = TrayIconBuilder::with_id(TRAY_ID)
        .tooltip("Meiki Tracker")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "open" => show_main_window(app),
            "pause" => {
                let paused = app.state::<TrackerControl>().paused.load(Ordering::Relaxed);
                set_paused(app, !paused);
            }
            // Les sessions en cours seront closes au dernier instant vu, au prochain lancement.
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            // Clic gauche sur l'icône : ouvrir la fenêtre (clic droit : menu).
            if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
                show_main_window(tray.app_handle());
            }
        });
    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }
    builder.build(app)?;
    Ok(())
}

/// Fermer la fenêtre = la cacher. La première fois, on explique où est passée l'appli.
pub fn on_close_requested(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.hide();
    }
    let db = app.state::<crate::commands::AppDb>();
    let Ok(conn) = db.0.lock() else { return };
    if db::get_setting(&conn, "tray_hint_shown").ok().flatten().is_none() {
        let _ = db::set_setting(&conn, "tray_hint_shown", "true");
        notify(
            app,
            "Meiki Tracker continue en arrière-plan",
            "Le suivi reste actif. Retrouve l'appli dans la zone de notification, près de l'horloge.",
        );
    }
}

pub(crate) fn notify(app: &AppHandle, title: &str, body: &str) {
    let _ = app.notification().builder().title(title).body(body).show();
}

/// « ce soir », « cet après-midi »… selon l'heure locale.
pub fn moment_of_day(ts: i64) -> &'static str {
    use chrono::{Local, TimeZone, Timelike};
    let hour = Local.timestamp_opt(ts, 0).single().map(|d| d.hour()).unwrap_or(20);
    match hour {
        5..=11 => "ce matin",
        12..=17 => "cet après-midi",
        18..=23 => "ce soir",
        _ => "cette nuit",
    }
}

/// Appelé par le thread de suivi à chaque vérification.
pub fn on_tracker_events(app: &AppHandle, conn: &Connection, events: &[TrackerEvent]) {
    let settings = Settings::load(conn).unwrap_or_default();
    for e in events {
        // Notification discrète à la fermeture du jeu : « Call of Duty : 2h14 ce soir ».
        if let TrackerEvent::Ended { game_id, net_secs } = e {
            if settings.notifications && *net_secs >= settings.min_session_secs {
                if let Ok(Some(g)) = db::get_game(conn, *game_id) {
                    let body = format!("{} : {} {}", g.name, service::fmt_duration(*net_secs), moment_of_day(db::now()));
                    notify(app, "Session terminée", &body);
                }
            }
            // Jeu Steam fermé : on récupère les succès gagnés pendant la partie.
            spawn_steam_game_sync(app, conn, *game_id);
        }
    }
}

/// Synchro des succès d'un jeu juste fermé (si l'option est activée et le jeu lié).
fn spawn_steam_game_sync(app: &AppHandle, conn: &Connection, game_id: i64) {
    let enabled = crate::steam::config(conn).map(|c| c.enabled).unwrap_or(false);
    let linked = crate::steam::game_appid(conn, game_id).ok().flatten().is_some();
    if !enabled || !linked {
        return;
    }
    let app = app.clone();
    std::thread::spawn(move || {
        // Laisse à Steam le temps d'enregistrer les derniers succès.
        std::thread::sleep(std::time::Duration::from_secs(30));
        match crate::steam::sync_game(&db::default_db_path(), game_id) {
            Ok(r) => {
                let _ = app.emit(crate::EVENT_CHANGED, ());
                if r.new_unlocked > 0 {
                    let conn = db::open(&db::default_db_path());
                    let notifications = conn.as_ref().ok().and_then(|c| Settings::load(c).ok()).map(|s| s.notifications).unwrap_or(true);
                    let name = conn.ok().and_then(|c| db::get_game(&c, game_id).ok().flatten()).map(|g| g.name).unwrap_or_default();
                    if notifications {
                        let body = if r.new_unlocked == 1 {
                            format!("{name} : 1 nouveau succès débloqué ({}/{})", r.unlocked, r.total)
                        } else {
                            format!("{name} : {} nouveaux succès débloqués ({}/{})", r.new_unlocked, r.unlocked, r.total)
                        };
                        notify(&app, "Succès Steam", &body);
                    }
                }
            }
            Err(e) => eprintln!("[steam] synchro après la partie impossible : {e}"),
        }
    });
}

/// Au lancement de l'appli : synchro de tous les jeux liés, au plus une fois toutes les 12 h.
pub fn spawn_steam_startup_sync(app: AppHandle, db_path: std::path::PathBuf) {
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_secs(20));
        let Ok(conn) = db::open(&db_path) else { return };
        let enabled = crate::steam::config(&conn).map(|c| c.enabled && c.api_key.is_some() && c.steam_id.is_some()).unwrap_or(false);
        let last: i64 = db::get_setting(&conn, crate::steam::KEY_LAST_SYNC).ok().flatten().and_then(|v| v.parse().ok()).unwrap_or(0);
        drop(conn);
        if !enabled || db::now() - last < 12 * 3600 {
            return;
        }
        match crate::steam::sync_all(&db_path) {
            Ok(r) => {
                for e in &r.errors {
                    eprintln!("[steam] {e}");
                }
                let _ = app.emit(crate::EVENT_CHANGED, ());
            }
            Err(e) => eprintln!("[steam] synchro au démarrage impossible : {e}"),
        }
    });
}

/// Infobulle de l'icône : « Meiki Tracker — En jeu : Call of Duty ».
pub fn update_tooltip(app: &AppHandle, live_names: &[String], paused: bool) {
    let text = if paused {
        "Meiki Tracker — tracking en pause".to_string()
    } else if live_names.is_empty() {
        "Meiki Tracker — tracking actif".to_string()
    } else {
        format!("Meiki Tracker — En jeu : {}", live_names.join(", "))
    };
    let state = app.state::<TrayTooltip>();
    let Ok(mut last) = state.0.lock() else { return };
    if *last != text {
        if let Some(tray) = app.tray_by_id(TRAY_ID) {
            let _ = tray.set_tooltip(Some(&text));
        }
        *last = text;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Local, TimeZone};

    #[test]
    fn moments_de_la_journee() {
        let at = |h| Local.with_ymd_and_hms(2026, 11, 18, h, 30, 0).unwrap().timestamp();
        assert_eq!(moment_of_day(at(9)), "ce matin");
        assert_eq!(moment_of_day(at(15)), "cet après-midi");
        assert_eq!(moment_of_day(at(22)), "ce soir");
        assert_eq!(moment_of_day(at(2)), "cette nuit");
    }
}
