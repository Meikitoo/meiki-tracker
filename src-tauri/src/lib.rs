//! Point d'entrée de l'application Meiki Tracker (côté Rust).

pub mod db;
pub mod exclusions;
pub mod service;
pub mod settings;
pub mod system;
pub mod tracker;

use service::TrackerControl;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let control = TrackerControl::default();

    // Le suivi tourne dans son propre thread, indépendamment de la fenêtre.
    service::spawn(db::default_db_path(), control.clone(), |conn, events| {
        for e in events {
            println!("[tracker] {}", service::describe(conn, e));
        }
    });

    tauri::Builder::default()
        .manage(control)
        .run(tauri::generate_context!())
        .expect("Erreur au lancement de Meiki Tracker");
}
