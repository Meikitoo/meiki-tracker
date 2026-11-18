//! Moteur de sessions : à chaque vérification, on lui donne l'heure, la liste des
//! processus ouverts et le temps d'inactivité du joueur. Il crée, prolonge, clôt
//! et enregistre les sessions. Aucune dépendance au système : entièrement testable.
//!
//! Cycle de vie d'une session :
//!   1. le jeu apparaît          → session « en cours » (table live_sessions)
//!   2. à chaque vérification    → `last_seen` est mis à jour (sert après un crash)
//!   3. le jeu disparaît         → session « fermée », gardée en attente
//!   4a. relance avant la fin de la fenêtre de fusion → on reprend la même session
//!   4b. sinon                   → enregistrée dans `sessions` (si assez longue)

use crate::db::{self, LiveSession, Session};
use crate::settings::Settings;
use rusqlite::{Connection, Result};
use std::collections::{HashMap, HashSet};

/// Ce que le moteur a fait pendant une vérification (pour les logs, l'UI, les notifications).
#[derive(Debug, Clone, PartialEq)]
pub enum TrackerEvent {
    Started { game_id: i64 },
    /// Relance fusionnée avec la session précédente.
    Resumed { game_id: i64, gap_secs: i64 },
    /// Le jeu vient de se fermer (session pas encore définitive).
    Ended { game_id: i64, net_secs: i64 },
    /// Session enregistrée définitivement.
    Saved { game_id: i64, session_id: i64, duration_secs: i64 },
    /// Session trop courte, ignorée.
    Discarded { game_id: i64, duration_secs: i64 },
    /// Session coupée au démarrage de l'appli (crash, extinction…).
    Recovered { game_id: i64, last_seen: i64 },
    /// Trou anormal entre deux vérifications (mise en veille du PC).
    Interrupted { game_id: i64, last_seen: i64 },
}

/// Ce que voit le moteur à un instant donné.
pub struct TickInput<'a> {
    pub now: i64,
    /// Noms des processus ouverts, en minuscules (ex. "cod.exe").
    pub running: &'a HashSet<String>,
    /// Secondes depuis la dernière action clavier/souris/manette.
    pub idle_secs: i64,
}

/// Au-delà de ce trou entre deux vérifications, on considère que le PC était en veille.
fn stale_gap(s: &Settings) -> i64 {
    (s.check_interval_secs * 3).max(30)
}

/// À appeler une fois au démarrage : toute session restée « en cours » vient d'une
/// appli coupée brutalement. On la clôt au dernier instant où le jeu a été vu.
pub fn recover(conn: &mut Connection) -> Result<Vec<TrackerEvent>> {
    let tx = conn.transaction()?;
    let mut events = Vec::new();
    for mut l in db::live_sessions(&tx)? {
        if l.ended_at.is_none() {
            let at = l.last_seen;
            close(&mut l, at);
            db::save_live(&tx, &l)?;
            events.push(TrackerEvent::Recovered { game_id: l.game_id, last_seen: at });
        }
    }
    tx.commit()?;
    Ok(events)
}

/// Une vérification. Tout est fait dans une transaction : soit tout est écrit, soit rien.
pub fn tick(conn: &mut Connection, s: &Settings, input: &TickInput) -> Result<Vec<TrackerEvent>> {
    let tx = conn.transaction()?;
    let mut events = Vec::new();
    let mut lives: HashMap<i64, LiveSession> =
        db::live_sessions(&tx)?.into_iter().map(|l| (l.game_id, l)).collect();

    for game in db::active_games(&tx)? {
        let running = input.running.contains(&game.process_name.to_lowercase());
        let live = lives.remove(&game.id);
        step_game(&tx, s, input, game.id, running, live, &mut events)?;
    }
    // Sessions restantes = jeux archivés entre-temps : on les termine normalement.
    for (game_id, live) in lives {
        step_game(&tx, s, input, game_id, false, Some(live), &mut events)?;
    }
    tx.commit()?;
    Ok(events)
}

fn step_game(
    conn: &Connection,
    s: &Settings,
    input: &TickInput,
    game_id: i64,
    running: bool,
    mut live: Option<LiveSession>,
    events: &mut Vec<TrackerEvent>,
) -> Result<()> {
    let now = input.now;

    // 1. Session en cours : mise à jour, ou fermeture.
    if let Some(l) = live.as_mut().filter(|l| l.ended_at.is_none()) {
        if now - l.last_seen > stale_gap(s) {
            // Le PC dormait : on coupe au dernier instant vu. Si le jeu tourne
            // toujours, l'étape 2 décidera de fusionner ou non.
            let at = l.last_seen;
            close(l, at);
            events.push(TrackerEvent::Interrupted { game_id, last_seen: at });
        } else {
            update_afk(l, now, input.idle_secs, s.afk_threshold_secs);
            if running {
                l.last_seen = now;
            } else {
                close(l, now);
                events.push(TrackerEvent::Ended { game_id, net_secs: l.net_secs(now) });
            }
        }
    }

    // 2. Jeu ouvert sans session en cours : reprise (fusion) ou nouvelle session.
    if running && live.as_ref().is_none_or(|l| l.ended_at.is_some()) {
        if let Some(mut l) = live.take() {
            let ended = l.ended_at.unwrap_or(now);
            if s.merge_window_secs > 0 && now - ended <= s.merge_window_secs {
                let gap = (now - ended).max(0);
                l.gap_secs += gap;
                l.ended_at = None;
                l.afk_start = None;
                l.segment_start = now;
                l.last_seen = now;
                events.push(TrackerEvent::Resumed { game_id, gap_secs: gap });
                live = Some(l);
            } else {
                finalize(conn, s, &l, events)?;
            }
        }
        if live.is_none() {
            live = Some(LiveSession::new(game_id, now));
            events.push(TrackerEvent::Started { game_id });
        }
    }

    // 3. Session fermée depuis plus longtemps que la fenêtre de fusion : on l'enregistre.
    if let Some(l) = &live {
        if let Some(ended) = l.ended_at {
            if now - ended > s.merge_window_secs {
                finalize(conn, s, l, events)?;
                live = None;
            }
        }
    }

    match &live {
        Some(l) => db::save_live(conn, l),
        None => db::delete_live(conn, game_id),
    }
}

/// Gestion de l'inactivité. On ne retire le temps AFK qu'au-delà du seuil, mais on
/// retire alors toute la période depuis la dernière action (pas seulement le dépassement).
fn update_afk(l: &mut LiveSession, now: i64, idle_secs: i64, threshold: i64) {
    if threshold <= 0 {
        return; // AFK désactivé
    }
    let last_input = now - idle_secs.max(0);
    if idle_secs >= threshold {
        if l.afk_start.is_none() {
            l.afk_start = Some(last_input.max(l.segment_start));
        }
    } else if let Some(start) = l.afk_start.take() {
        l.afk_secs += (last_input - start).max(0);
    }
}

/// Ferme la session à l'instant `at` (le temps AFK en cours est comptabilisé).
fn close(l: &mut LiveSession, at: i64) {
    let at = at.max(l.segment_start);
    if let Some(start) = l.afk_start.take() {
        l.afk_secs += (at - start).max(0);
    }
    l.ended_at = Some(at);
    l.last_seen = at;
}

fn finalize(conn: &Connection, s: &Settings, l: &LiveSession, events: &mut Vec<TrackerEvent>) -> Result<()> {
    let ended_at = l.ended_at.unwrap_or(l.last_seen);
    let duration = l.net_secs(ended_at);
    if duration >= s.min_session_secs && duration > 0 {
        let session_id = db::insert_session(
            conn,
            &Session {
                id: 0,
                game_id: l.game_id,
                started_at: l.started_at,
                ended_at,
                duration_secs: duration,
                afk_secs: l.afk_secs,
                gap_secs: l.gap_secs,
                tag: None,
                note: None,
            },
        )?;
        events.push(TrackerEvent::Saved { game_id: l.game_id, session_id, duration_secs: duration });
    } else {
        events.push(TrackerEvent::Discarded { game_id: l.game_id, duration_secs: duration });
    }
    Ok(())
}

// ====================================================================== Tests

#[cfg(test)]
mod tests {
    use super::*;

    const STEP: i64 = 5;

    fn setup() -> (Connection, Settings, i64) {
        let conn = db::open_in_memory().unwrap();
        let game = db::add_game(&conn, "Call of Duty", "cod.exe", "game", 0).unwrap();
        (conn, Settings::default(), game)
    }

    fn running(on: bool) -> HashSet<String> {
        if on { HashSet::from(["cod.exe".to_string()]) } else { HashSet::new() }
    }

    /// Simule des vérifications toutes les 5 s de `from` à `to` inclus.
    /// `idle(t)` donne l'inactivité à l'instant t.
    fn play(
        conn: &mut Connection, s: &Settings, from: i64, to: i64, on: bool,
        idle: impl Fn(i64) -> i64,
    ) -> Vec<TrackerEvent> {
        let set = running(on);
        let mut all = Vec::new();
        let mut t = from;
        while t <= to {
            all.extend(tick(conn, s, &TickInput { now: t, running: &set, idle_secs: idle(t) }).unwrap());
            t += STEP;
        }
        all
    }

    fn once(conn: &mut Connection, s: &Settings, t: i64, on: bool) -> Vec<TrackerEvent> {
        play(conn, s, t, t, on, |_| 0)
    }

    fn saved(conn: &Connection) -> Vec<Session> {
        let mut v = db::sessions(conn, None, 100).unwrap();
        v.reverse();
        v
    }

    #[test]
    fn session_normale_enregistree() {
        let (mut c, s, g) = setup();
        let ev = play(&mut c, &s, 0, 3600, true, |_| 0);
        assert_eq!(ev[0], TrackerEvent::Started { game_id: g });
        once(&mut c, &s, 3605, false); // jeu fermé
        assert!(saved(&c).is_empty(), "pas encore définitive pendant la fenêtre de fusion");
        let ev = once(&mut c, &s, 3605 + 301, false);
        assert!(matches!(ev[0], TrackerEvent::Saved { duration_secs: 3605, .. }));
        let ss = saved(&c);
        assert_eq!(ss.len(), 1);
        assert_eq!((ss[0].started_at, ss[0].ended_at, ss[0].duration_secs), (0, 3605, 3605));
        assert!(db::live_sessions(&c).unwrap().is_empty());
    }

    #[test]
    fn session_trop_courte_ignoree() {
        let (mut c, s, _) = setup();
        play(&mut c, &s, 0, 60, true, |_| 0);
        once(&mut c, &s, 65, false);
        let ev = once(&mut c, &s, 400, false);
        assert!(matches!(ev[0], TrackerEvent::Discarded { duration_secs: 65, .. }));
        assert!(saved(&c).is_empty());
    }

    #[test]
    fn relance_rapide_fusionnee() {
        let (mut c, s, _) = setup();
        play(&mut c, &s, 0, 1000, true, |_| 0);
        once(&mut c, &s, 1005, false);
        let ev = play(&mut c, &s, 1200, 2000, true, |_| 0); // relance 195 s plus tard
        assert_eq!(ev[0], TrackerEvent::Resumed { game_id: 1, gap_secs: 195 });
        once(&mut c, &s, 2005, false);
        once(&mut c, &s, 2400, false);
        let ss = saved(&c);
        assert_eq!(ss.len(), 1);
        assert_eq!((ss[0].started_at, ss[0].ended_at), (0, 2005));
        assert_eq!(ss[0].gap_secs, 195);
        assert_eq!(ss[0].duration_secs, 2005 - 195);
    }

    #[test]
    fn relance_tardive_nouvelle_session() {
        let (mut c, s, _) = setup();
        play(&mut c, &s, 0, 1000, true, |_| 0);
        once(&mut c, &s, 1005, false);
        play(&mut c, &s, 1500, 2000, true, |_| 0); // 495 s > 300 s
        once(&mut c, &s, 2005, false);
        once(&mut c, &s, 2400, false);
        let ss = saved(&c);
        assert_eq!(ss.len(), 2);
        assert_eq!(ss[0].duration_secs, 1005);
        assert_eq!(ss[1].duration_secs, 505);
    }

    #[test]
    fn fusion_desactivee() {
        let (mut c, mut s, _) = setup();
        s.merge_window_secs = 0;
        play(&mut c, &s, 0, 1000, true, |_| 0);
        once(&mut c, &s, 1005, false);
        play(&mut c, &s, 1010, 2000, true, |_| 0);
        once(&mut c, &s, 2005, false);
        once(&mut c, &s, 2010, false);
        assert_eq!(saved(&c).len(), 2);
    }

    #[test]
    fn afk_retire() {
        let (mut c, s, _) = setup();
        // Aucune action entre 1000 et 2000, seuil 600 s : 1000 s retirées.
        let idle = |t: i64| if (1000..2000).contains(&t) { t - 1000 } else { 0 };
        play(&mut c, &s, 0, 3600, true, idle);
        once(&mut c, &s, 3605, false);
        once(&mut c, &s, 4000, false);
        let ss = saved(&c);
        assert_eq!(ss[0].afk_secs, 1000);
        assert_eq!(ss[0].duration_secs, 3605 - 1000);
    }

    #[test]
    fn courte_inactivite_non_retiree() {
        let (mut c, s, _) = setup();
        let idle = |t: i64| if (1000..1500).contains(&t) { t - 1000 } else { 0 };
        play(&mut c, &s, 0, 3600, true, idle);
        once(&mut c, &s, 3605, false);
        once(&mut c, &s, 4000, false);
        assert_eq!(saved(&c)[0].afk_secs, 0);
    }

    #[test]
    fn afk_jusqu_a_la_fermeture() {
        let (mut c, s, _) = setup();
        // Le joueur part à 1000 et ne revient pas ; le jeu est fermé à 3605.
        let idle = |t: i64| if t >= 1000 { t - 1000 } else { 0 };
        play(&mut c, &s, 0, 3600, true, idle);
        play(&mut c, &s, 3605, 3605, false, idle);
        once(&mut c, &s, 4000, false);
        let ss = saved(&c);
        assert_eq!(ss[0].afk_secs, 2605);
        assert_eq!(ss[0].duration_secs, 1000);
    }

    #[test]
    fn chrono_en_direct() {
        let (mut c, s, _) = setup();
        let idle = |t: i64| if t >= 1000 { t - 1000 } else { 0 };
        play(&mut c, &s, 0, 2000, true, idle);
        let l = &db::live_sessions(&c).unwrap()[0];
        // À 2000 : 1000 s jouées, AFK en cours depuis 1000 (non compté).
        assert_eq!(l.net_secs(2000), 1000);
    }

    #[test]
    fn recuperation_apres_crash() {
        let (mut c, s, _) = setup();
        play(&mut c, &s, 0, 1000, true, |_| 0);
        // Crash : plus aucune vérification. Redémarrage de l'appli bien plus tard.
        let ev = recover(&mut c).unwrap();
        assert_eq!(ev, vec![TrackerEvent::Recovered { game_id: 1, last_seen: 1000 }]);
        once(&mut c, &s, 9000, false);
        let ss = saved(&c);
        assert_eq!((ss[0].ended_at, ss[0].duration_secs), (1000, 1000));
    }

    #[test]
    fn recuperation_puis_relance_fusionnee() {
        let (mut c, s, _) = setup();
        play(&mut c, &s, 0, 1000, true, |_| 0);
        recover(&mut c).unwrap();
        // Le PC redémarre vite, le jeu est relancé 120 s après le dernier battement.
        let ev = play(&mut c, &s, 1120, 2000, true, |_| 0);
        assert_eq!(ev[0], TrackerEvent::Resumed { game_id: 1, gap_secs: 120 });
        once(&mut c, &s, 2005, false);
        once(&mut c, &s, 3000, false);
        let ss = saved(&c);
        assert_eq!(ss.len(), 1);
        assert_eq!(ss[0].duration_secs, 2005 - 120);
    }

    #[test]
    fn crash_pendant_afk() {
        let (mut c, s, _) = setup();
        let idle = |t: i64| if t >= 500 { t - 500 } else { 0 };
        play(&mut c, &s, 0, 1500, true, idle);
        recover(&mut c).unwrap();
        once(&mut c, &s, 9000, false);
        let ss = saved(&c);
        assert_eq!((ss[0].afk_secs, ss[0].duration_secs), (1000, 500));
    }

    #[test]
    fn mise_en_veille_du_pc() {
        let (mut c, s, _) = setup();
        play(&mut c, &s, 0, 1000, true, |_| 0);
        // Une heure sans vérification (veille), le jeu est toujours ouvert au réveil.
        let ev = once(&mut c, &s, 4600, true);
        assert_eq!(ev[0], TrackerEvent::Interrupted { game_id: 1, last_seen: 1000 });
        assert!(matches!(ev[1], TrackerEvent::Saved { duration_secs: 1000, .. }));
        assert_eq!(ev[2], TrackerEvent::Started { game_id: 1 });
    }

    #[test]
    fn courte_veille_fusionnee() {
        let (mut c, s, _) = setup();
        play(&mut c, &s, 0, 1000, true, |_| 0);
        let ev = once(&mut c, &s, 1200, true); // 200 s de veille
        assert_eq!(ev[1], TrackerEvent::Resumed { game_id: 1, gap_secs: 200 });
        play(&mut c, &s, 1205, 2000, true, |_| 0);
        once(&mut c, &s, 2005, false);
        once(&mut c, &s, 3000, false);
        assert_eq!(saved(&c)[0].duration_secs, 2005 - 200);
    }

    #[test]
    fn deux_jeux_en_parallele() {
        let (mut c, s, cod) = setup();
        let obs = db::add_game(&c, "OBS", "obs64.exe", "creation", 0).unwrap();
        let both: HashSet<String> = ["cod.exe", "obs64.exe"].map(String::from).into();
        for t in (0..=1000).step_by(5) {
            tick(&mut c, &s, &TickInput { now: t, running: &both, idle_secs: 0 }).unwrap();
        }
        once(&mut c, &s, 1005, false);
        once(&mut c, &s, 2000, false);
        assert_eq!(db::tracked_secs(&c, cod).unwrap(), 1005);
        assert_eq!(db::tracked_secs(&c, obs).unwrap(), 1005);
    }

    #[test]
    fn nom_de_processus_insensible_a_la_casse() {
        let (mut c, s, _) = setup();
        let set: HashSet<String> = HashSet::from(["cod.exe".to_string()]);
        db::add_game(&c, "Valorant", "VALORANT.exe", "game", 0).unwrap();
        let ev = tick(&mut c, &s, &TickInput { now: 0, running: &set, idle_secs: 0 }).unwrap();
        assert_eq!(ev.len(), 1);
        let set2: HashSet<String> = HashSet::from(["valorant.exe".to_string()]);
        let ev = tick(&mut c, &s, &TickInput { now: 5, running: &set2, idle_secs: 0 }).unwrap();
        assert!(ev.contains(&TrackerEvent::Started { game_id: 2 }));
    }
}
