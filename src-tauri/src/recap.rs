//! « Mon récap » : bilan d'une période (semaine, mois, année) à partager en image.
//! Comme ailleurs, une session coupée par la période est comptée au prorata, et la
//! session en cours (jeu ouvert) n'est pas incluse.

use crate::db;
use crate::library::{local_date, local_midnight, week_start, Span};
use chrono::{Datelike, Duration, NaiveDate};
use rusqlite::{Connection, Result};
use serde::Serialize;
use std::collections::{BTreeSet, HashMap};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Period {
    Week,
    Month,
    Year,
}

impl Period {
    pub fn parse(s: &str) -> Option<Period> {
        match s {
            "week" => Some(Period::Week),
            "month" => Some(Period::Month),
            "year" => Some(Period::Year),
            _ => None,
        }
    }

    /// Début de la période en cours (minuit local).
    pub fn start(self, now: i64) -> i64 {
        let today = local_date(now);
        match self {
            Period::Week => week_start(now),
            Period::Month => local_midnight(NaiveDate::from_ymd_opt(today.year(), today.month(), 1).unwrap_or(today)),
            Period::Year => local_midnight(NaiveDate::from_ymd_opt(today.year(), 1, 1).unwrap_or(today)),
        }
    }
}

#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RecapGame {
    pub id: i64,
    pub name: String,
    pub secs: i64,
}

#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RecapSession {
    pub game_name: String,
    pub secs: i64,
    pub started_at: i64,
}

#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RecapDay {
    pub day_start: i64,
    pub secs: i64,
}

#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RecapTag {
    pub tag: String,
    pub secs: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Recap {
    pub start: i64,
    pub end: i64,
    pub total_secs: i64,
    pub session_count: i64,
    pub days_played: i64,
    /// Plus longue suite de jours joués d'affilée dans la période.
    pub best_streak: i64,
    /// Jeux les plus joués (5 max), du plus au moins joué.
    pub top_games: Vec<RecapGame>,
    pub longest: Option<RecapSession>,
    pub busiest_day: Option<RecapDay>,
    /// Temps par tag (4 max).
    pub tags: Vec<RecapTag>,
    /// Succès Steam débloqués pendant la période.
    pub achievements: i64,
}

pub fn build(conn: &Connection, period: Period, now: i64) -> Result<Recap> {
    let start = period.start(now);
    let end = now;
    let games: HashMap<i64, String> = db::active_games(conn)?.into_iter().map(|g| (g.id, g.name)).collect();

    let mut total = 0i64;
    let mut count = 0i64;
    let mut per_game: HashMap<i64, i64> = HashMap::new();
    let mut per_tag: HashMap<String, i64> = HashMap::new();
    let mut per_day: HashMap<NaiveDate, i64> = HashMap::new();
    let mut longest: Option<RecapSession> = None;

    for s in db::sessions(conn, None, i64::MAX)? {
        let Some(name) = games.get(&s.game_id) else { continue };
        if s.ended_at < start || s.started_at >= end {
            continue;
        }
        let span = Span { game_id: s.game_id, start: s.started_at, end: s.ended_at, net: s.duration_secs };
        let secs = span.net_within(start, end);
        if secs <= 0 {
            continue;
        }
        total += secs;
        count += 1;
        *per_game.entry(s.game_id).or_default() += secs;
        if let Some(tag) = s.tag.filter(|t| !t.is_empty()) {
            *per_tag.entry(tag).or_default() += secs;
        }
        if s.started_at >= start && longest.as_ref().is_none_or(|l| s.duration_secs > l.secs) {
            longest = Some(RecapSession { game_name: name.clone(), secs: s.duration_secs, started_at: s.started_at });
        }
        // Répartition par jour (au prorata si la session passe minuit).
        let mut day = local_date(s.started_at.max(start));
        let last = local_date((s.ended_at - 1).max(s.started_at).min(end));
        while day <= last {
            let a = local_midnight(day);
            let b = local_midnight(day + Duration::days(1));
            let part = span.net_within(a.max(start), b.min(end));
            if part > 0 {
                *per_day.entry(day).or_default() += part;
            }
            day += Duration::days(1);
        }
    }

    let mut top_games: Vec<RecapGame> = per_game
        .into_iter()
        .map(|(id, secs)| RecapGame { id, name: games[&id].clone(), secs })
        .collect();
    top_games.sort_by(|a, b| b.secs.cmp(&a.secs).then_with(|| a.name.cmp(&b.name)));
    top_games.truncate(5);

    let mut tags: Vec<RecapTag> = per_tag.into_iter().map(|(tag, secs)| RecapTag { tag, secs }).collect();
    tags.sort_by(|a, b| b.secs.cmp(&a.secs).then_with(|| a.tag.cmp(&b.tag)));
    tags.truncate(4);

    let busiest_day = per_day
        .iter()
        .max_by(|a, b| a.1.cmp(b.1).then_with(|| b.0.cmp(a.0)))
        .map(|(d, secs)| RecapDay { day_start: local_midnight(*d), secs: *secs });

    let days: BTreeSet<NaiveDate> = per_day.keys().copied().collect();
    let mut best_streak = 0i64;
    let mut run = 0i64;
    let mut prev: Option<NaiveDate> = None;
    for d in &days {
        run = if prev.is_some_and(|p| *d == p + Duration::days(1)) { run + 1 } else { 1 };
        best_streak = best_streak.max(run);
        prev = Some(*d);
    }

    let achievements = conn.query_row(
        "SELECT COUNT(*) FROM steam_achievements a JOIN games g ON g.id = a.game_id
         WHERE g.archived = 0 AND a.achieved = 1 AND a.unlocked_at >= ?1 AND a.unlocked_at < ?2",
        [start, end + 1],
        |r| r.get(0),
    )?;

    Ok(Recap {
        start,
        end,
        total_secs: total,
        session_count: count,
        days_played: days.len() as i64,
        best_streak,
        top_games,
        longest,
        busiest_day,
        tags,
        achievements,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Local, TimeZone};

    fn at(y: i32, m: u32, d: u32, h: u32) -> i64 {
        Local.with_ymd_and_hms(y, m, d, h, 0, 0).earliest().unwrap().timestamp()
    }

    fn session(c: &Connection, game: i64, start: i64, secs: i64, tag: Option<&str>) {
        db::insert_session(c, &db::Session {
            id: 0, game_id: game, started_at: start, ended_at: start + secs, duration_secs: secs,
            afk_secs: 0, gap_secs: 0, tag: tag.map(str::to_string), note: None,
        })
        .unwrap();
    }

    #[test]
    fn recap_de_la_semaine() {
        let c = db::open_in_memory().unwrap();
        let cod = db::add_game(&c, "COD", "cod.exe", "game", 0).unwrap();
        let val = db::add_game(&c, "Valorant", "val.exe", "game", 0).unwrap();
        // Mercredi 18 nov. 2026 ; la semaine commence le lundi 16.
        let now = at(2026, 11, 18, 23);
        session(&c, cod, at(2026, 11, 15, 20), 3 * 3600, Some("Fun")); // dimanche : semaine d'avant
        session(&c, cod, at(2026, 11, 16, 20), 2 * 3600, Some("Ranked"));
        session(&c, val, at(2026, 11, 17, 20), 3600, None);
        session(&c, cod, at(2026, 11, 18, 20), 3 * 3600, Some("Ranked"));

        let r = build(&c, Period::Week, now).unwrap();
        assert_eq!(r.total_secs, 6 * 3600);
        assert_eq!((r.session_count, r.days_played, r.best_streak), (3, 3, 3));
        assert_eq!(r.top_games[0], RecapGame { id: cod, name: "COD".into(), secs: 5 * 3600 });
        assert_eq!(r.top_games.len(), 2);
        assert_eq!(r.longest.as_ref().unwrap().secs, 3 * 3600);
        assert_eq!(r.busiest_day.as_ref().unwrap().secs, 3 * 3600);
        assert_eq!(r.tags, vec![RecapTag { tag: "Ranked".into(), secs: 5 * 3600 }]);

        // Le mois inclut le dimanche : série de 4 jours, 9 h.
        let m = build(&c, Period::Month, now).unwrap();
        assert_eq!((m.total_secs, m.best_streak), (9 * 3600, 4));
    }

    #[test]
    fn session_a_cheval_sur_minuit_et_debut_de_periode() {
        let c = db::open_in_memory().unwrap();
        let g = db::add_game(&c, "Jeu", "jeu.exe", "game", 0).unwrap();
        // Dimanche 23 h → lundi 1 h : seule l'heure du lundi compte dans la semaine.
        session(&c, g, at(2026, 11, 15, 23), 2 * 3600, None);
        let r = build(&c, Period::Week, at(2026, 11, 18, 12)).unwrap();
        assert_eq!(r.total_secs, 3600);
        assert_eq!(r.days_played, 1);
        assert!(r.longest.is_none(), "la session a commencé avant la période");
    }
}
