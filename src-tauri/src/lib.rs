// Point d'entrée de l'application Meiki Tracker (côté Rust).
// Les modules de tracking, base de données, etc. arriveront à l'étape 2.

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("Erreur au lancement de Meiki Tracker");
}
