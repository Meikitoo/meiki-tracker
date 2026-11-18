//! Données de la fiche d'un jeu : tuiles, 14 derniers jours, liste des sessions.
//! Comme pour la bibliothèque, la session en cours (jeu ouvert) n'est pas comptée
//! ici : l'interface l'ajoute en direct.

use crate::db::{self, Game};
use crate::library::{local_date, local_midnight, Span};
use chrono::Duration;
use rusqlite::{Connection, Result};
use serde::Serialize;

pub const CHART_DAYS: i64 = 14;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GameInfo {
    pub id: i64,
    pub name: String,
    pub process_name: String,
    pub category: String,
    pub baseline_secs: i64,
    pub baseline_set_at: Option<i64>,
    pub weekly_limit_secs: Option<i64>,
    pub added_at: i64,
}

#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DayStat {
    /// Minuit local du jour (secondes Unix).
    pub day_start: i64,
    pub secs: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionRow {
    pub id: i64,
    pub started_at: i64,
    pub ended_at: i64,
    pub duration_secs: i64,
    pub afk_secs: i64,
    pub gap_secs: i64,
    pub tag: Option<String>,
    pub note: Option<String>,
}

/// Session fermée il y a moins de 5 min, en attente d'une éventuelle relance.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PendingSession {
    pub started_at: i64,
    pub ended_at: i64,
    pub net_secs: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GameDetail {
    pub game: GameInfo,
    pub tracked_secs: i64,
    pub total_secs: i64,
    pub session_count: i64,
    pub longest_secs: i64,
    pub days: Vec<DayStat>,
    pub sessions: Vec<SessionRow>,
    pub pending: Option<PendingSession>,
    pub live: bool,
    pub at: i64,
}

pub fn build(conn: &Connection, id: i64, now: i64) -> Result<Option<GameDetail>> {
    let Some(g) = db::get_game(conn, id)? else { return Ok(None) };
    if g.archived {
        return Ok(None);
    }
    let Game { name, process_name, category, baseline_secs, baseline_set_at, weekly_limit_secs, added_at, .. } = g;

    let sessions: Vec<SessionRow> = db::sessions(conn, Some(id), i64::MAX)?
        .into_iter()
        .map(|s| SessionRow {
            id: s.id,
            started_at: s.started_at,
            ended_at: s.ended_at,
            duration_secs: s.duration_secs,
            afk_secs: s.afk_secs,
            gap_secs: s.gap_secs,
            tag: s.tag,
            note: s.note,
        })
        .collect();

    let live_row = db::live_sessions(conn)?.into_iter().find(|l| l.game_id == id);
    let pending = live_row.as_ref().and_then(|l| {
        l.ended_at.map(|end| PendingSession { started_at: l.started_at, ended_at: end, net_secs: l.net_secs(end) })
    });
    let live = live_row.as_ref().is_some_and(|l| l.ended_at.is_none());

    let mut spans: Vec<Span> = sessions
        .iter()
        .map(|s| Span { game_id: id, start: s.started_at, end: s.ended_at, net: s.duration_secs })
        .collect();
    if let Some(p) = &pending {
        spans.push(Span { game_id: id, start: p.started_at, end: p.ended_at, net: p.net_secs });
    }

    let tracked: i64 = spans.iter().map(|s| s.net).sum();
    let longest = spans.iter().map(|s| s.net).max().unwrap_or(0);

    Ok(Some(GameDetail {
        game: GameInfo { id, name, process_name, category, baseline_secs, baseline_set_at, weekly_limit_secs, added_at },
        tracked_secs: tracked,
        total_secs: tracked + baseline_secs,
        session_count: spans.len() as i64,
        longest_secs: longest,
        days: daily(&spans, now, CHART_DAYS),
        sessions,
        pending,
        live,
        at: now,
    }))
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GameShare {
    pub id: i64,
    pub name: String,
    pub week_secs: i64,
    pub total_secs: i64,
}

/// Page Statistiques : tous jeux confondus (hors session en cours, ajoutée en direct).
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Overview {
    pub days: Vec<DayStat>,
    pub games: Vec<GameShare>,
    pub at: i64,
}

pub fn overview(conn: &Connection, now: i64) -> Result<Overview> {
    let lib = crate::library::build(conn, now, false, 5)?;
    let mut spans = Vec::new();
    {
        let mut stmt = conn.prepare(
            "SELECT s.game_id, s.started_at, s.ended_at, s.duration_secs FROM sessions s
             JOIN games g ON g.id = s.game_id WHERE g.archived = 0",
        )?;
        let rows = stmt.query_map([], |r| Ok(Span { game_id: r.get(0)?, start: r.get(1)?, end: r.get(2)?, net: r.get(3)? }))?;
        for s in rows {
            spans.push(s?);
        }
    }
    for l in db::live_sessions(conn)? {
        if let Some(end) = l.ended_at {
            spans.push(Span { game_id: l.game_id, start: l.started_at, end, net: l.net_secs(end) });
        }
    }
    let mut games: Vec<GameShare> = lib
        .games
        .into_iter()
        .map(|g| GameShare { id: g.id, name: g.name, week_secs: g.week_secs, total_secs: g.total_secs })
        .collect();
    games.sort_by(|a, b| b.week_secs.cmp(&a.week_secs).then(b.total_secs.cmp(&a.total_secs)));
    Ok(Overview { days: daily(&spans, now, CHART_DAYS), games, at: now })
}

/// Temps joué par jour (heure locale) sur les `n` derniers jours, aujourd'hui compris.
fn daily(spans: &[Span], now: i64, n: i64) -> Vec<DayStat> {
    let today = local_date(now);
    (0..n)
        .rev()
        .map(|back| {
            let day = today - Duration::days(back);
            let (a, b) = (local_midnight(day), local_midnight(day + Duration::days(1)));
            DayStat { day_start: a, secs: spans.iter().map(|s| s.net_within(a, b)).sum() }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fiche_complete() {
        let c = db::open_in_memory().unwrap();
        let now = db::now();
        let id = db::add_game(&c, "COD", "cod.exe", "game", 1000).unwrap();
        for (start, dur) in [(now - 3 * 86_400, 3600), (now - 7200, 1800)] {
            db::insert_session(&c, &db::Session {
                id: 0, game_id: id, started_at: start, ended_at: start + dur,
                duration_secs: dur, afk_secs: 0, gap_secs: 0, tag: None, note: None,
            }).unwrap();
        }
        let d = build(&c, id, now).unwrap().unwrap();
        assert_eq!((d.tracked_secs, d.total_secs, d.session_count, d.longest_secs), (5400, 6400, 2, 3600));
        assert_eq!(d.days.len(), 14);
        assert_eq!(d.days.iter().map(|x| x.secs).sum::<i64>(), 5400);
        assert_eq!(d.sessions[0].duration_secs, 1800, "plus récente d'abord");

        db::archive_game(&c, id).unwrap();
        assert!(build(&c, id, now).unwrap().is_none(), "un jeu retiré n'a plus de fiche");
    }

    #[test]
    fn edition_des_sessions() {
        let c = db::open_in_memory().unwrap();
        let id = db::add_game(&c, "COD", "cod.exe", "game", 0).unwrap();
        let sid = db::insert_session(&c, &db::Session {
            id: 0, game_id: id, started_at: 0, ended_at: 600,
            duration_secs: 600, afk_secs: 0, gap_secs: 0, tag: None, note: None,
        }).unwrap();
        db::update_session(&c, sid, Some("Ranked"), Some("Top 1")).unwrap();
        let s = &db::sessions(&c, Some(id), 10).unwrap()[0];
        assert_eq!((s.tag.as_deref(), s.note.as_deref()), (Some("Ranked"), Some("Top 1")));
        db::delete_session(&c, sid).unwrap();
        assert!(db::sessions(&c, Some(id), 10).unwrap().is_empty());
    }
}
