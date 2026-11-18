//! Outil console pour tester le tracking sans interface.
//! Usage : cargo run --bin meiki-debug -- <commande>

use meiki_tracker_lib::{db, exclusions, service, settings::{self, Settings}, system::ProcessScanner};
use std::sync::atomic::Ordering;

const AIDE: &str = "\
Meiki Tracker — outil de debug

Commandes :
  jeux                         Liste les jeux suivis
  ajouter <exe> [nom]          Suit un programme (ex. ajouter notepad.exe Bloc-notes)
  processus                    Programmes ouverts qu'on pourrait suivre
  suivre                       Lance le tracking ici, en direct (Ctrl+C pour arrêter)
  sessions [nombre]            Dernières sessions enregistrées (20 par défaut)
  en-cours                     Sessions en cours / en attente de fusion
  reglages                     Affiche les réglages
  regler <clé> <valeur>        Modifie un réglage (ex. regler min_session_secs 10)
  chemin                       Emplacement de la base de données";

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let cmd = args.first().map(String::as_str).unwrap_or("aide");
    let path = db::default_db_path();
    let conn = match db::open(&path) {
        Ok(c) => c,
        Err(e) => return eprintln!("Impossible d'ouvrir la base {} : {e}", path.display()),
    };

    let result: rusqlite::Result<()> = (|| {
        match cmd {
            "jeux" => {
                let games = db::active_games(&conn)?;
                if games.is_empty() {
                    println!("Aucun jeu suivi. Ajoute-en un : ajouter <exe> [nom]");
                }
                for g in games {
                    let tracked = db::tracked_secs(&conn, g.id)?;
                    println!(
                        "#{:<3} {:<25} {:<30} total {} (dont {} trackées · départ {})",
                        g.id, g.name, g.process_name,
                        service::fmt_duration(g.baseline_secs + tracked),
                        service::fmt_duration(tracked),
                        service::fmt_duration(g.baseline_secs),
                    );
                }
            }
            "ajouter" => {
                let Some(exe) = args.get(1) else {
                    println!("Précise le nom du .exe, ex. : ajouter notepad.exe Bloc-notes");
                    return Ok(());
                };
                if exclusions::is_excluded(exe) {
                    println!("Attention : {exe} est dans la liste d'exclusion (launcher/système). Ajouté quand même pour le test.");
                }
                let name = if args.len() > 2 { args[2..].join(" ") } else { exe.trim_end_matches(".exe").to_string() };
                match db::add_game(&conn, &name, exe, "game", 0) {
                    Ok(id) => println!("Ajouté : #{id} {name} ({exe})"),
                    Err(e) => println!("Impossible d'ajouter (déjà suivi ?) : {e}"),
                }
            }
            "processus" => {
                for p in ProcessScanner::new().candidate_programs() {
                    println!("{:<40} {}", p.process_name, p.exe_path.unwrap_or_default());
                }
            }
            "sessions" => {
                let n = args.get(1).and_then(|v| v.parse().ok()).unwrap_or(20);
                let list = db::sessions(&conn, None, n)?;
                if list.is_empty() {
                    println!("Aucune session enregistrée pour l'instant.");
                }
                for s in list {
                    let game = db::get_game(&conn, s.game_id)?.map(|g| g.name).unwrap_or_default();
                    println!(
                        "{:<20} {} → {}  {:>8}  (AFK retiré : {}, pauses retirées : {})",
                        game, service::fmt_time(s.started_at), service::fmt_time(s.ended_at),
                        service::fmt_duration(s.duration_secs),
                        service::fmt_duration(s.afk_secs), service::fmt_duration(s.gap_secs),
                    );
                }
            }
            "en-cours" => {
                let now = db::now();
                let lives = db::live_sessions(&conn)?;
                if lives.is_empty() {
                    println!("Aucune session en cours.");
                }
                for l in lives {
                    let game = db::get_game(&conn, l.game_id)?.map(|g| g.name).unwrap_or_default();
                    let state = match l.ended_at {
                        None => "EN JEU".to_string(),
                        Some(t) => format!("fermé à {}, en attente de fusion", service::fmt_time(t)),
                    };
                    println!(
                        "{game} : {state} — début {}, dernier passage {}, compté {}",
                        service::fmt_time(l.started_at), service::fmt_time(l.last_seen),
                        service::fmt_duration(l.net_secs(now)),
                    );
                }
            }
            "reglages" => println!("{:#?}", Settings::load(&conn)?),
            "regler" => match (args.get(1), args.get(2)) {
                (Some(k), Some(v)) if settings::KEYS.contains(&k.as_str()) => {
                    db::set_setting(&conn, k, v)?;
                    println!("{k} = {v}");
                }
                _ => println!("Usage : regler <clé> <valeur>. Clés : {}", settings::KEYS.join(", ")),
            },
            "chemin" => println!("{}", path.display()),
            "suivre" => {
                let others = ProcessScanner::new().running_names().unwrap_or_default();
                if others.contains("meiki-tracker.exe") {
                    println!("L'appli Meiki Tracker tourne déjà et suit les jeux : ferme-la d'abord pour éviter un double suivi.");
                    return Ok(());
                }
                drop(conn);
                println!("Tracking en direct… (Ctrl+C pour arrêter — ça simule aussi un crash)");
                let control = service::TrackerControl::default();
                control.stop.store(false, Ordering::Relaxed);
                service::run(&path, &control, |conn, events| {
                    for e in events {
                        println!("{}  {}", service::fmt_time(db::now()), service::describe(conn, e));
                    }
                })?;
            }
            _ => println!("{AIDE}"),
        }
        Ok(())
    })();

    if let Err(e) = result {
        eprintln!("Erreur : {e}");
    }
}
