//! Boucle de fond : toutes les quelques secondes, regarde les processus ouverts
//! et fait avancer le moteur de sessions.

use crate::db;
use crate::settings::Settings;
use crate::system::{InputMonitor, ProcessScanner};
use crate::tracker::{self, TickInput, TrackerEvent};
use rusqlite::Connection;
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

/// Contrôle partagé avec le reste de l'appli (bouton « pause » de la zone de notification…).
#[derive(Clone, Default)]
pub struct TrackerControl {
    pub paused: Arc<AtomicBool>,
    pub stop: Arc<AtomicBool>,
}

/// Lance la boucle dans un thread dédié. `on_events` est appelé à chaque tour
/// (avec la liste des changements, souvent vide).
pub fn spawn<F>(db_path: PathBuf, control: TrackerControl, on_events: F) -> std::thread::JoinHandle<()>
where
    F: Fn(&Connection, &[TrackerEvent]) + Send + 'static,
{
    std::thread::Builder::new()
        .name("meiki-tracker".into())
        .spawn(move || {
            if let Err(e) = run(&db_path, &control, on_events) {
                eprintln!("[tracker] arrêt sur erreur : {e}");
            }
        })
        .expect("impossible de démarrer le thread de tracking")
}

pub fn run<F>(db_path: &std::path::Path, control: &TrackerControl, on_events: F) -> rusqlite::Result<()>
where
    F: Fn(&Connection, &[TrackerEvent]),
{
    let mut conn = db::open(db_path)?;
    let recovered = tracker::recover(&mut conn)?;
    if !recovered.is_empty() {
        on_events(&conn, &recovered);
    }

    let mut scanner = ProcessScanner::new();
    let mut input = InputMonitor::new();
    let empty = HashSet::new();

    while !control.stop.load(Ordering::Relaxed) {
        // Réglages relus à chaque tour : un changement dans Paramètres s'applique tout de suite.
        let settings = Settings::load(&conn)?;
        let now = db::now();
        let paused = control.paused.load(Ordering::Relaxed);
        let idle_secs = input.idle_secs(now);
        let names = if paused { Some(empty.clone()) } else { scanner.running_names() };
        let Some(names) = names else {
            // Lecture des processus impossible ce tour-ci : on ne touche à rien.
            sleep_interruptible(Duration::from_secs(settings.check_interval_secs as u64), &control.stop);
            continue;
        };
        let tick_input = TickInput { now, running: &names, idle_secs };
        match tracker::tick(&mut conn, &settings, &tick_input) {
            Ok(events) => on_events(&conn, &events),
            // Une erreur ponctuelle (disque occupé…) ne doit pas arrêter le suivi.
            Err(e) => eprintln!("[tracker] erreur pendant la vérification : {e}"),
        }
        sleep_interruptible(Duration::from_secs(settings.check_interval_secs as u64), &control.stop);
    }
    Ok(())
}

fn sleep_interruptible(total: Duration, stop: &AtomicBool) {
    let step = Duration::from_millis(250);
    let mut slept = Duration::ZERO;
    while slept < total && !stop.load(Ordering::Relaxed) {
        std::thread::sleep(step);
        slept += step;
    }
}

/// Texte lisible d'un événement (console de debug, logs).
pub fn describe(conn: &Connection, e: &TrackerEvent) -> String {
    let name = |id: i64| {
        db::get_game(conn, id)
            .ok()
            .flatten()
            .map(|g| g.name)
            .unwrap_or_else(|| format!("jeu #{id}"))
    };
    match e {
        TrackerEvent::Started { game_id } => format!("▶ {} : session démarrée", name(*game_id)),
        TrackerEvent::Resumed { game_id, gap_secs } => {
            format!("↻ {} : relancé, fusionné avec la session précédente (pause de {} retirée)", name(*game_id), fmt_duration(*gap_secs))
        }
        TrackerEvent::Ended { game_id, net_secs } => {
            format!("■ {} : fermé ({} comptées, en attente de fusion éventuelle)", name(*game_id), fmt_duration(*net_secs))
        }
        TrackerEvent::Saved { game_id, duration_secs, .. } => {
            format!("✔ {} : session enregistrée ({})", name(*game_id), fmt_duration(*duration_secs))
        }
        TrackerEvent::Discarded { game_id, duration_secs } => {
            format!("✖ {} : session trop courte ignorée ({})", name(*game_id), fmt_duration(*duration_secs))
        }
        TrackerEvent::Recovered { game_id, last_seen } => {
            format!("⚠ {} : session récupérée après coupure (dernier passage {})", name(*game_id), fmt_time(*last_seen))
        }
        TrackerEvent::Interrupted { game_id, last_seen } => {
            format!("⏸ {} : PC en veille détecté, session coupée à {}", name(*game_id), fmt_time(*last_seen))
        }
    }
}

/// 2h14, 12 min, 45 s
pub fn fmt_duration(secs: i64) -> String {
    let secs = secs.max(0);
    let (h, m) = (secs / 3600, (secs % 3600) / 60);
    if h > 0 {
        format!("{h}h{m:02}")
    } else if m > 0 {
        format!("{m} min")
    } else {
        format!("{secs} s")
    }
}

/// Heure locale lisible.
pub fn fmt_time(ts: i64) -> String {
    use chrono::{Local, TimeZone};
    Local
        .timestamp_opt(ts, 0)
        .single()
        .map(|d| d.format("%d/%m/%Y %H:%M:%S").to_string())
        .unwrap_or_else(|| ts.to_string())
}
