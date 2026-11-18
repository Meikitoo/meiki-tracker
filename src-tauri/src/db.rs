//! Base de données locale SQLite : ouverture, schéma et accès aux tables.
//!
//! Toutes les dates sont des secondes Unix (UTC). Les durées sont en secondes.

use rusqlite::{params, Connection, OptionalExtension, Result};
use std::path::{Path, PathBuf};

/// Version du schéma. À incrémenter à chaque migration ajoutée dans `migrate`.
const SCHEMA_VERSION: i32 = 1;

/// Dossier des données : %APPDATA%\MeikiTracker
pub fn data_dir() -> PathBuf {
    let base = std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    base.join("MeikiTracker")
}

/// Chemin de la base. La variable d'environnement MEIKI_DB permet d'utiliser
/// une base de test (développement uniquement).
pub fn default_db_path() -> PathBuf {
    match std::env::var_os("MEIKI_DB") {
        Some(p) if !p.is_empty() => PathBuf::from(p),
        _ => data_dir().join("meiki.db"),
    }
}

/// Ouvre (ou crée) la base au chemin donné et applique les migrations.
pub fn open(path: &Path) -> Result<Connection> {
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let conn = Connection::open(path)?;
    // WAL + synchronous NORMAL : écritures rapides et résistantes aux coupures
    // (au pire on perd le dernier battement de quelques secondes, jamais la base).
    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.pragma_update(None, "synchronous", "NORMAL")?;
    conn.busy_timeout(std::time::Duration::from_secs(5))?;
    setup(&conn)?;
    Ok(conn)
}

/// Base en mémoire, utilisée par les tests.
pub fn open_in_memory() -> Result<Connection> {
    let conn = Connection::open_in_memory()?;
    setup(&conn)?;
    Ok(conn)
}

fn setup(conn: &Connection) -> Result<()> {
    conn.pragma_update(None, "foreign_keys", "ON")?;
    migrate(conn)
}

fn migrate(conn: &Connection) -> Result<()> {
    let version: i32 = conn.pragma_query_value(None, "user_version", |r| r.get(0))?;
    if version < 1 {
        conn.execute_batch(
            r#"
            BEGIN;
            CREATE TABLE games (
                id               INTEGER PRIMARY KEY,
                name             TEXT    NOT NULL,
                process_name     TEXT    NOT NULL UNIQUE COLLATE NOCASE,
                category         TEXT    NOT NULL DEFAULT 'game',
                baseline_secs    INTEGER NOT NULL DEFAULT 0,
                baseline_set_at  INTEGER,
                weekly_limit_secs INTEGER,
                icon_png         BLOB,
                added_at         INTEGER NOT NULL,
                archived         INTEGER NOT NULL DEFAULT 0
            );
            CREATE TABLE sessions (
                id            INTEGER PRIMARY KEY,
                game_id       INTEGER NOT NULL REFERENCES games(id) ON DELETE CASCADE,
                started_at    INTEGER NOT NULL,
                ended_at      INTEGER NOT NULL,
                duration_secs INTEGER NOT NULL,
                afk_secs      INTEGER NOT NULL DEFAULT 0,
                gap_secs      INTEGER NOT NULL DEFAULT 0,
                tag           TEXT,
                note          TEXT
            );
            CREATE INDEX idx_sessions_game_start ON sessions(game_id, started_at);
            CREATE INDEX idx_sessions_start ON sessions(started_at);
            CREATE TABLE live_sessions (
                game_id       INTEGER PRIMARY KEY REFERENCES games(id) ON DELETE CASCADE,
                started_at    INTEGER NOT NULL,
                segment_start INTEGER NOT NULL,
                last_seen     INTEGER NOT NULL,
                afk_secs      INTEGER NOT NULL DEFAULT 0,
                gap_secs      INTEGER NOT NULL DEFAULT 0,
                afk_start     INTEGER,
                ended_at      INTEGER
            );
            CREATE TABLE settings (
                key   TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );
            PRAGMA user_version = 1;
            COMMIT;
            "#,
        )?;
    }
    debug_assert!(SCHEMA_VERSION == 1);
    Ok(())
}

pub fn now() -> i64 {
    chrono::Utc::now().timestamp()
}

// ---------------------------------------------------------------- Jeux

#[derive(Debug, Clone, PartialEq)]
pub struct Game {
    pub id: i64,
    pub name: String,
    pub process_name: String,
    pub category: String,
    pub baseline_secs: i64,
    pub baseline_set_at: Option<i64>,
    pub weekly_limit_secs: Option<i64>,
    pub added_at: i64,
    pub archived: bool,
}

const GAME_COLS: &str = "id, name, process_name, category, baseline_secs, baseline_set_at, \
                         weekly_limit_secs, added_at, archived";

fn row_to_game(r: &rusqlite::Row) -> Result<Game> {
    Ok(Game {
        id: r.get(0)?,
        name: r.get(1)?,
        process_name: r.get(2)?,
        category: r.get(3)?,
        baseline_secs: r.get(4)?,
        baseline_set_at: r.get(5)?,
        weekly_limit_secs: r.get(6)?,
        added_at: r.get(7)?,
        archived: r.get(8)?,
    })
}

pub fn add_game(
    conn: &Connection,
    name: &str,
    process_name: &str,
    category: &str,
    baseline_secs: i64,
) -> Result<i64> {
    let now = now();
    let baseline_set_at = (baseline_secs > 0).then_some(now);
    conn.execute(
        "INSERT INTO games (name, process_name, category, baseline_secs, baseline_set_at, added_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![name, process_name.to_lowercase(), category, baseline_secs, baseline_set_at, now],
    )?;
    Ok(conn.last_insert_rowid())
}

/// Jeux suivis (non archivés).
pub fn active_games(conn: &Connection) -> Result<Vec<Game>> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {GAME_COLS} FROM games WHERE archived = 0 ORDER BY name"
    ))?;
    let rows = stmt.query_map([], row_to_game)?;
    rows.collect()
}

pub fn get_game(conn: &Connection, id: i64) -> Result<Option<Game>> {
    conn.query_row(
        &format!("SELECT {GAME_COLS} FROM games WHERE id = ?1"),
        [id],
        row_to_game,
    )
    .optional()
}

// ---------------------------------------------------------------- Sessions terminées

#[derive(Debug, Clone, PartialEq)]
pub struct Session {
    pub id: i64,
    pub game_id: i64,
    pub started_at: i64,
    pub ended_at: i64,
    /// Durée nette comptée (fin − début − AFK − pauses entre relances fusionnées).
    pub duration_secs: i64,
    pub afk_secs: i64,
    pub gap_secs: i64,
    pub tag: Option<String>,
    pub note: Option<String>,
}

pub fn insert_session(conn: &Connection, s: &Session) -> Result<i64> {
    conn.execute(
        "INSERT INTO sessions (game_id, started_at, ended_at, duration_secs, afk_secs, gap_secs, tag, note)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![s.game_id, s.started_at, s.ended_at, s.duration_secs, s.afk_secs, s.gap_secs, s.tag, s.note],
    )?;
    Ok(conn.last_insert_rowid())
}

/// Sessions, les plus récentes d'abord. `game_id = None` : tous les jeux.
pub fn sessions(conn: &Connection, game_id: Option<i64>, limit: i64) -> Result<Vec<Session>> {
    let mut stmt = conn.prepare(
        "SELECT id, game_id, started_at, ended_at, duration_secs, afk_secs, gap_secs, tag, note
         FROM sessions WHERE (?1 IS NULL OR game_id = ?1)
         ORDER BY started_at DESC LIMIT ?2",
    )?;
    let rows = stmt.query_map(params![game_id, limit], |r| {
        Ok(Session {
            id: r.get(0)?,
            game_id: r.get(1)?,
            started_at: r.get(2)?,
            ended_at: r.get(3)?,
            duration_secs: r.get(4)?,
            afk_secs: r.get(5)?,
            gap_secs: r.get(6)?,
            tag: r.get(7)?,
            note: r.get(8)?,
        })
    })?;
    rows.collect()
}

/// Somme des sessions trackées d'un jeu (sans le temps de départ).
pub fn tracked_secs(conn: &Connection, game_id: i64) -> Result<i64> {
    conn.query_row(
        "SELECT COALESCE(SUM(duration_secs), 0) FROM sessions WHERE game_id = ?1",
        [game_id],
        |r| r.get(0),
    )
}

// ---------------------------------------------------------------- Sessions en cours

/// Session en cours (ou tout juste terminée, en attente d'une éventuelle fusion).
/// Écrite à chaque vérification : c'est elle qui permet la récupération après crash.
#[derive(Debug, Clone, PartialEq)]
pub struct LiveSession {
    pub game_id: i64,
    pub started_at: i64,
    /// Début du segment actuel (= début, ou instant de la dernière relance fusionnée).
    pub segment_start: i64,
    /// Dernier instant où le jeu a été vu ouvert.
    pub last_seen: i64,
    pub afk_secs: i64,
    pub gap_secs: i64,
    /// Début de la période AFK en cours, si le joueur est inactif en ce moment.
    pub afk_start: Option<i64>,
    /// `Some` : le jeu est fermé, on attend la fin de la fenêtre de fusion.
    pub ended_at: Option<i64>,
}

impl LiveSession {
    pub fn new(game_id: i64, now: i64) -> Self {
        LiveSession {
            game_id,
            started_at: now,
            segment_start: now,
            last_seen: now,
            afk_secs: 0,
            gap_secs: 0,
            afk_start: None,
            ended_at: None,
        }
    }

    /// Temps net compté à l'instant `now` (pour le chrono en direct).
    pub fn net_secs(&self, now: i64) -> i64 {
        let end = self.ended_at.unwrap_or(now);
        let pending_afk = self.afk_start.map(|s| (end - s).max(0)).unwrap_or(0);
        (end - self.started_at - self.afk_secs - self.gap_secs - pending_afk).max(0)
    }
}

pub fn live_sessions(conn: &Connection) -> Result<Vec<LiveSession>> {
    let mut stmt = conn.prepare(
        "SELECT game_id, started_at, segment_start, last_seen, afk_secs, gap_secs, afk_start, ended_at
         FROM live_sessions",
    )?;
    let rows = stmt.query_map([], |r| {
        Ok(LiveSession {
            game_id: r.get(0)?,
            started_at: r.get(1)?,
            segment_start: r.get(2)?,
            last_seen: r.get(3)?,
            afk_secs: r.get(4)?,
            gap_secs: r.get(5)?,
            afk_start: r.get(6)?,
            ended_at: r.get(7)?,
        })
    })?;
    rows.collect()
}

pub fn save_live(conn: &Connection, l: &LiveSession) -> Result<()> {
    conn.execute(
        "INSERT OR REPLACE INTO live_sessions
         (game_id, started_at, segment_start, last_seen, afk_secs, gap_secs, afk_start, ended_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![l.game_id, l.started_at, l.segment_start, l.last_seen, l.afk_secs, l.gap_secs, l.afk_start, l.ended_at],
    )?;
    Ok(())
}

pub fn delete_live(conn: &Connection, game_id: i64) -> Result<()> {
    conn.execute("DELETE FROM live_sessions WHERE game_id = ?1", [game_id])?;
    Ok(())
}

// ---------------------------------------------------------------- Réglages (clé/valeur)

pub fn get_setting(conn: &Connection, key: &str) -> Result<Option<String>> {
    conn.query_row("SELECT value FROM settings WHERE key = ?1", [key], |r| r.get(0))
        .optional()
}

pub fn set_setting(conn: &Connection, key: &str, value: &str) -> Result<()> {
    conn.execute(
        "INSERT INTO settings (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![key, value],
    )?;
    Ok(())
}
