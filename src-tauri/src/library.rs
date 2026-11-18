//! Calcul des données de l'écran « Bibliothèque » : totaux par jeu, temps de la
//! semaine, série de jours consécutifs, sessions en cours.
//!
//! Convention : les sessions *en cours* (jeu ouvert) ne sont PAS incluses dans les
//! totaux. L'interface les ajoute elle-même chaque seconde grâce au chrono en direct.

use crate::db::{self, LiveSession};
use chrono::{Datelike, Duration, Local, NaiveDate, TimeZone};
use rusqlite::{Connection, Result};
use serde::Serialize;
use std::collections::{BTreeSet, HashMap};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GameCard {
    pub id: i64,
    pub name: String,
    pub process_name: String,
    pub category: String,
    pub baseline_secs: i64,
    pub tracked_secs: i64,
    pub total_secs: i64,
    pub week_secs: i64,
    pub weekly_limit_secs: Option<i64>,
    pub last_played_at: Option<i64>,
    pub session_count: i64,
    pub live: bool,
}

#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Summary {
    pub week_secs: i64,
    /// Même période la semaine dernière (du lundi jusqu'au même jour/heure).
    pub last_week_secs: i64,
    pub total_secs: i64,
    pub tracked_secs: i64,
    pub streak_days: i64,
    pub streak_record: i64,
    pub week_start: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveInfo {
    pub game_id: i64,
    pub name: String,
    pub started_at: i64,
    /// Temps compté à l'instant `at` du message.
    pub net_secs: i64,
    pub afk: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Library {
    pub games: Vec<GameCard>,
    pub summary: Summary,
    pub live: Vec<LiveInfo>,
    pub paused: bool,
    pub check_interval_secs: i64,
    /// Instant du calcul (secondes Unix), référence du chrono.
    pub at: i64,
}

/// Message envoyé à l'interface à chaque vérification.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveTick {
    pub live: Vec<LiveInfo>,
    pub paused: bool,
    pub at: i64,
}

/// Une plage de jeu : [début, fin] avec sa durée nette.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Span {
    pub(crate) game_id: i64,
    pub(crate) start: i64,
    pub(crate) end: i64,
    pub(crate) net: i64,
}

impl Span {
    /// Part de la durée nette tombant dans [a, b), au prorata du temps réel.
    pub(crate) fn net_within(&self, a: i64, b: i64) -> i64 {
        let wall = self.end - self.start;
        if wall <= 0 {
            return if self.start >= a && self.start < b { self.net } else { 0 };
        }
        let overlap = (self.end.min(b) - self.start.max(a)).max(0);
        ((self.net as i128 * overlap as i128) / wall as i128) as i64
    }
}

pub(crate) fn local_midnight(date: NaiveDate) -> i64 {
    let naive = date.and_hms_opt(0, 0, 0).expect("minuit valide");
    Local
        .from_local_datetime(&naive)
        .earliest()
        .map(|d| d.timestamp())
        .unwrap_or_else(|| naive.and_utc().timestamp())
}

pub(crate) fn local_date(ts: i64) -> NaiveDate {
    Local
        .timestamp_opt(ts, 0)
        .single()
        .map(|d| d.date_naive())
        .unwrap_or_default()
}

/// Lundi 00:00 (heure locale) de la semaine contenant `now`.
pub fn week_start(now: i64) -> i64 {
    let today = local_date(now);
    local_midnight(today - Duration::days(today.weekday().num_days_from_monday() as i64))
}

/// Infos des sessions en cours (jeu ouvert), pour le chrono.
pub fn live_infos(conn: &Connection, now: i64) -> Result<Vec<LiveInfo>> {
    let names: HashMap<i64, String> =
        db::active_games(conn)?.into_iter().map(|g| (g.id, g.name)).collect();
    Ok(db::live_sessions(conn)?
        .into_iter()
        .filter(|l| l.ended_at.is_none())
        .filter_map(|l| {
            Some(LiveInfo {
                game_id: l.game_id,
                name: names.get(&l.game_id)?.clone(),
                started_at: l.started_at,
                net_secs: l.net_secs(now),
                afk: l.afk_start.is_some(),
            })
        })
        .collect())
}

pub fn build(conn: &Connection, now: i64, paused: bool, check_interval_secs: i64) -> Result<Library> {
    let games = db::active_games(conn)?;
    let lives = db::live_sessions(conn)?;

    // Sessions terminées + sessions fermées en attente de fusion (déjà définitives en pratique).
    let mut spans: Vec<Span> = Vec::new();
    {
        let mut stmt = conn.prepare("SELECT game_id, started_at, ended_at, duration_secs FROM sessions")?;
        let rows = stmt.query_map([], |r| {
            Ok(Span { game_id: r.get(0)?, start: r.get(1)?, end: r.get(2)?, net: r.get(3)? })
        })?;
        for s in rows {
            spans.push(s?);
        }
    }
    let pending = |l: &LiveSession| {
        l.ended_at.map(|end| Span { game_id: l.game_id, start: l.started_at, end, net: l.net_secs(end) })
    };
    spans.extend(lives.iter().filter_map(pending));

    let running: HashMap<i64, &LiveSession> =
        lives.iter().filter(|l| l.ended_at.is_none()).map(|l| (l.game_id, l)).collect();

    let ws = week_start(now);
    // Début de la semaine dernière : le lundi de la semaine qui contient « dimanche soir ».
    // (robuste aux changements d'heure, contrairement à « ws − 7 jours »)
    let lws = week_start(ws - 3_600);
    let elapsed = now - ws;

    let mut cards = Vec::with_capacity(games.len());
    let mut summary = Summary {
        week_secs: 0,
        last_week_secs: 0,
        total_secs: 0,
        tracked_secs: 0,
        streak_days: 0,
        streak_record: 0,
        week_start: ws,
    };
    let mut days: BTreeSet<NaiveDate> = BTreeSet::new();

    for g in &games {
        let mine: Vec<&Span> = spans.iter().filter(|s| s.game_id == g.id).collect();
        let tracked: i64 = mine.iter().map(|s| s.net).sum();
        let week: i64 = mine.iter().map(|s| s.net_within(ws, now + 1)).sum();
        let last_week: i64 = mine.iter().map(|s| s.net_within(lws, lws + elapsed)).sum();
        let live = running.get(&g.id);
        let last_played_at = mine
            .iter()
            .map(|s| s.start)
            .chain(live.map(|l| l.started_at))
            .max();

        for s in &mine {
            add_days(&mut days, s.start, s.end);
        }
        if let Some(l) = live {
            add_days(&mut days, l.started_at, now);
        }

        summary.week_secs += week;
        summary.last_week_secs += last_week;
        summary.tracked_secs += tracked;
        summary.total_secs += tracked + g.baseline_secs;

        cards.push(GameCard {
            id: g.id,
            name: g.name.clone(),
            process_name: g.process_name.clone(),
            category: g.category.clone(),
            baseline_secs: g.baseline_secs,
            tracked_secs: tracked,
            total_secs: tracked + g.baseline_secs,
            week_secs: week,
            weekly_limit_secs: g.weekly_limit_secs,
            last_played_at,
            session_count: mine.len() as i64 + i64::from(live.is_some()),
            live: live.is_some(),
        });
    }

    let (streak, record) = streaks(&days, local_date(now));
    summary.streak_days = streak;
    summary.streak_record = record;

    Ok(Library {
        games: cards,
        summary,
        live: live_infos(conn, now)?,
        paused,
        check_interval_secs,
        at: now,
    })
}

fn add_days(days: &mut BTreeSet<NaiveDate>, start: i64, end: i64) {
    let (mut d, last) = (local_date(start), local_date(end.max(start)));
    while d <= last {
        days.insert(d);
        d += Duration::days(1);
    }
}

/// (série actuelle, record). La série reste valable tant qu'on a joué hier :
/// elle ne retombe à 0 que si ni aujourd'hui ni hier n'ont de session.
fn streaks(days: &BTreeSet<NaiveDate>, today: NaiveDate) -> (i64, i64) {
    let mut record = 0;
    let mut run = 0;
    let mut prev: Option<NaiveDate> = None;
    for &d in days {
        run = match prev {
            Some(p) if d - p == Duration::days(1) => run + 1,
            _ => 1,
        };
        record = record.max(run);
        prev = Some(d);
    }
    let mut cur = 0;
    let mut d = if days.contains(&today) { today } else { today - Duration::days(1) };
    while days.contains(&d) {
        cur += 1;
        d -= Duration::days(1);
    }
    (cur, record)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    #[test]
    fn serie_et_record() {
        let days: BTreeSet<_> = [
            date(2026, 11, 1), date(2026, 11, 2), date(2026, 11, 3), date(2026, 11, 4),
            date(2026, 11, 10), date(2026, 11, 11),
        ]
        .into();
        assert_eq!(streaks(&days, date(2026, 11, 11)), (2, 4));
        assert_eq!(streaks(&days, date(2026, 11, 12)), (2, 4), "hier compte encore");
        assert_eq!(streaks(&days, date(2026, 11, 13)), (0, 4));
    }

    #[test]
    fn prorata_sur_une_periode() {
        let s = Span { game_id: 1, start: 0, end: 1000, net: 500 };
        assert_eq!(s.net_within(0, 1000), 500);
        assert_eq!(s.net_within(500, 2000), 250);
        assert_eq!(s.net_within(2000, 3000), 0);
    }

    #[test]
    fn semaine_commence_le_lundi() {
        let now = db::now();
        let ws = week_start(now);
        let d = Local.timestamp_opt(ws, 0).unwrap();
        assert_eq!(d.weekday(), chrono::Weekday::Mon);
        assert!(ws <= now && now - ws < 8 * 86_400);
    }

    #[test]
    fn bibliotheque_exclut_la_session_en_cours() {
        let conn = db::open_in_memory().unwrap();
        let now = db::now();
        let g = db::add_game(&conn, "COD", "cod.exe", "game", 3600).unwrap();
        db::insert_session(&conn, &db::Session {
            id: 0, game_id: g, started_at: now - 7200, ended_at: now - 3600,
            duration_secs: 3600, afk_secs: 0, gap_secs: 0, tag: None, note: None,
        }).unwrap();
        let mut l = LiveSession::new(g, now - 600);
        l.last_seen = now;
        db::save_live(&conn, &l).unwrap();

        let lib = build(&conn, now, false, 5).unwrap();
        let c = &lib.games[0];
        assert_eq!((c.tracked_secs, c.total_secs, c.baseline_secs), (3600, 7200, 3600));
        assert!(c.live);
        assert_eq!(lib.live.len(), 1);
        assert_eq!(lib.live[0].net_secs, 600);
        assert_eq!(lib.summary.total_secs, 7200);
        assert!(lib.summary.streak_days >= 1);
    }
}
