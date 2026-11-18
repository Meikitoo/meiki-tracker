//! Succès Steam — option désactivée par défaut.
//!
//! - Partie locale (toujours sans Internet) : lecture des fichiers de configuration
//!   de Steam pour trouver le compte connecté et les jeux installés.
//! - Partie Internet : UNIQUEMENT si l'utilisateur active la synchronisation. L'appli
//!   contacte alors seulement les serveurs de Steam (api.steampowered.com et ses
//!   images), avec la clé API de l'utilisateur, pour lire ses succès. Rien d'autre.

use crate::db;
use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Duration;

pub const KEY_ENABLED: &str = "steam_enabled";
pub const KEY_API_KEY: &str = "steam_api_key";
pub const KEY_STEAM_ID: &str = "steam_id";
pub const KEY_LAST_SYNC: &str = "steam_last_sync_at";

/// Langue des noms et descriptions de succès renvoyés par Steam.
const LANG: &str = "french";
const API: &str = "https://api.steampowered.com";
/// Outils Steam installés comme des jeux, jamais proposés.
const IGNORED_APPS: &[u32] = &[228980];

// ====================================================================== VDF

/// Format texte de configuration de Steam (« KeyValues »).
#[derive(Debug, Clone, PartialEq)]
pub enum Vdf {
    Str(String),
    Obj(Vec<(String, Vdf)>),
}

impl Vdf {
    pub fn get(&self, key: &str) -> Option<&Vdf> {
        match self {
            Vdf::Obj(items) => items.iter().find(|(k, _)| k.eq_ignore_ascii_case(key)).map(|(_, v)| v),
            Vdf::Str(_) => None,
        }
    }
    pub fn str(&self, key: &str) -> Option<&str> {
        match self.get(key)? {
            Vdf::Str(s) => Some(s),
            Vdf::Obj(_) => None,
        }
    }
    pub fn entries(&self) -> &[(String, Vdf)] {
        match self {
            Vdf::Obj(items) => items,
            Vdf::Str(_) => &[],
        }
    }
}

enum Tok {
    Open,
    Close,
    Text(String),
}

fn tokenize(src: &str) -> Vec<Tok> {
    let mut out = Vec::new();
    let mut it = src.chars().peekable();
    while let Some(c) = it.next() {
        match c {
            '{' => out.push(Tok::Open),
            '}' => out.push(Tok::Close),
            '"' => {
                let mut s = String::new();
                while let Some(c) = it.next() {
                    match c {
                        '"' => break,
                        '\\' => match it.next() {
                            Some('n') => s.push('\n'),
                            Some('t') => s.push('\t'),
                            Some(o) => s.push(o),
                            None => {}
                        },
                        o => s.push(o),
                    }
                }
                out.push(Tok::Text(s));
            }
            '/' if it.peek() == Some(&'/') => {
                for c in it.by_ref() {
                    if c == '\n' {
                        break;
                    }
                }
            }
            c if c.is_whitespace() => {}
            c => {
                let mut s = String::from(c);
                while let Some(&n) = it.peek() {
                    if n.is_whitespace() || n == '{' || n == '}' || n == '"' {
                        break;
                    }
                    s.push(n);
                    it.next();
                }
                // Conditions du type [$WIN32] : ignorées.
                if !s.starts_with('[') {
                    out.push(Tok::Text(s));
                }
            }
        }
    }
    out
}

fn parse_obj(toks: &mut std::iter::Peekable<std::vec::IntoIter<Tok>>) -> Vec<(String, Vdf)> {
    let mut items = Vec::new();
    while let Some(t) = toks.next() {
        match t {
            Tok::Close => break,
            Tok::Open => {
                // Bloc sans clé : on le lit pour rester aligné, puis on l'ignore.
                parse_obj(toks);
            }
            Tok::Text(key) => match toks.next() {
                Some(Tok::Open) => items.push((key, Vdf::Obj(parse_obj(toks)))),
                Some(Tok::Text(v)) => items.push((key, Vdf::Str(v))),
                Some(Tok::Close) | None => break,
            },
        }
    }
    items
}

/// Lit un texte VDF. La racine est un objet contenant le ou les blocs du fichier.
pub fn parse_vdf(src: &str) -> Vdf {
    let mut toks = tokenize(src).into_iter().peekable();
    Vdf::Obj(parse_obj(&mut toks))
}

fn read_vdf(path: &Path) -> Option<Vdf> {
    let bytes = std::fs::read(path).ok()?;
    Some(parse_vdf(&String::from_utf8_lossy(&bytes)))
}

// ====================================================================== Partie locale

/// Dossier d'installation de Steam (registre, sinon emplacement par défaut).
pub fn steam_dir() -> Option<PathBuf> {
    let from_registry = registry_steam_path().map(PathBuf::from).filter(|p| p.is_dir());
    from_registry.or_else(|| {
        let p = PathBuf::from(r"C:\Program Files (x86)\Steam");
        p.is_dir().then_some(p)
    })
}

#[cfg(windows)]
fn registry_steam_path() -> Option<String> {
    use windows::core::w;
    use windows::Win32::Foundation::ERROR_SUCCESS;
    use windows::Win32::System::Registry::{RegGetValueW, HKEY_CURRENT_USER, RRF_RT_REG_SZ};
    let mut buf = [0u16; 1024];
    let mut size = (buf.len() * 2) as u32;
    // SAFETY : tampon et taille valides ; lecture seule d'une valeur du registre.
    let res = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            w!("Software\\Valve\\Steam"),
            w!("SteamPath"),
            RRF_RT_REG_SZ,
            None,
            Some(buf.as_mut_ptr().cast()),
            Some(&mut size),
        )
    };
    if res != ERROR_SUCCESS {
        return None;
    }
    let len = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    Some(String::from_utf16_lossy(&buf[..len]).replace('/', "\\"))
}

#[cfg(not(windows))]
fn registry_steam_path() -> Option<String> {
    None
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SteamAccount {
    pub steam_id: String,
    pub persona: String,
}

/// Compte Steam utilisé en dernier sur ce PC (fichier config/loginusers.vdf).
pub fn detect_account_in(steam: &Path) -> Option<SteamAccount> {
    let root = read_vdf(&steam.join("config").join("loginusers.vdf"))?;
    let users = root.get("users")?;
    let accounts: Vec<SteamAccount> = users
        .entries()
        .iter()
        .filter(|(id, _)| valid_steam_id(id))
        .map(|(id, v)| (id, v))
        .map(|(id, v)| SteamAccount {
            steam_id: id.clone(),
            persona: v.str("PersonaName").or(v.str("AccountName")).unwrap_or("").to_string(),
        })
        .collect();
    let recent = users
        .entries()
        .iter()
        .find(|(id, v)| valid_steam_id(id) && v.str("MostRecent") == Some("1"))
        .map(|(id, _)| id.clone());
    match recent {
        Some(id) => accounts.into_iter().find(|a| a.steam_id == id),
        None => accounts.into_iter().next(),
    }
}

pub fn detect_account() -> Option<SteamAccount> {
    detect_account_in(&steam_dir()?)
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct InstalledGame {
    pub appid: u32,
    pub name: String,
    #[serde(skip)]
    pub dir: PathBuf,
}

/// Jeux Steam installés sur ce PC, toutes bibliothèques confondues.
pub fn installed_games_in(steam: &Path) -> Vec<InstalledGame> {
    let mut libs: Vec<PathBuf> = vec![steam.to_path_buf()];
    if let Some(root) = read_vdf(&steam.join("steamapps").join("libraryfolders.vdf")) {
        if let Some(folders) = root.get("libraryfolders") {
            for (_, f) in folders.entries() {
                if let Some(p) = f.str("path") {
                    let p = PathBuf::from(p);
                    if !libs.iter().any(|l| same_path(l, &p)) {
                        libs.push(p);
                    }
                }
            }
        }
    }
    let mut games = Vec::new();
    for lib in libs {
        let apps = lib.join("steamapps");
        let Ok(entries) = std::fs::read_dir(&apps) else { continue };
        for e in entries.flatten() {
            let name = e.file_name().to_string_lossy().to_lowercase();
            if !(name.starts_with("appmanifest_") && name.ends_with(".acf")) {
                continue;
            }
            let Some(root) = read_vdf(&e.path()) else { continue };
            let Some(state) = root.get("AppState") else { continue };
            let (Some(appid), Some(title), Some(dir)) = (
                state.str("appid").and_then(|a| a.parse::<u32>().ok()),
                state.str("name"),
                state.str("installdir"),
            ) else {
                continue;
            };
            if IGNORED_APPS.contains(&appid) || games.iter().any(|g: &InstalledGame| g.appid == appid) {
                continue;
            }
            games.push(InstalledGame { appid, name: title.to_string(), dir: apps.join("common").join(dir) });
        }
    }
    games.sort_by_key(|g| g.name.to_lowercase());
    games
}

pub fn installed_games() -> Vec<InstalledGame> {
    steam_dir().map(|d| installed_games_in(&d)).unwrap_or_default()
}

fn norm(p: &Path) -> String {
    p.to_string_lossy().replace('/', "\\").trim_end_matches('\\').to_lowercase()
}

fn same_path(a: &Path, b: &Path) -> bool {
    norm(a) == norm(b)
}

/// Jeu Steam dont le dossier contient ce .exe (chemin complet connu).
pub fn appid_for_exe(exe: &Path, games: &[InstalledGame]) -> Option<u32> {
    let exe = norm(exe);
    games
        .iter()
        .find(|g| {
            let dir = norm(&g.dir);
            exe.starts_with(&dir) && exe[dir.len()..].starts_with('\\')
        })
        .map(|g| g.appid)
}

/// Jeu Steam dont le dossier contient un fichier portant ce nom de processus.
pub fn appid_for_process(process_name: &str, games: &[InstalledGame]) -> Option<u32> {
    let target = process_name.to_lowercase();
    games.iter().find(|g| contains_file(&g.dir, &target, 5)).map(|g| g.appid)
}

fn contains_file(dir: &Path, target: &str, depth: u32) -> bool {
    let Ok(entries) = std::fs::read_dir(dir) else { return false };
    let mut subdirs = Vec::new();
    for e in entries.flatten() {
        let Ok(ft) = e.file_type() else { continue };
        if ft.is_file() && e.file_name().to_string_lossy().to_lowercase() == target {
            return true;
        }
        if ft.is_dir() {
            subdirs.push(e.path());
        }
    }
    depth > 0 && subdirs.iter().any(|d| contains_file(d, target, depth - 1))
}

pub fn valid_steam_id(s: &str) -> bool {
    s.len() == 17 && s.starts_with("7656119") && s.chars().all(|c| c.is_ascii_digit())
}

pub fn valid_api_key(s: &str) -> bool {
    s.len() == 32 && s.chars().all(|c| c.is_ascii_hexdigit())
}

// ====================================================================== Réglages

#[derive(Debug, Clone, Default)]
pub struct SteamConfig {
    pub enabled: bool,
    pub api_key: Option<String>,
    pub steam_id: Option<String>,
}

pub fn config(conn: &Connection) -> rusqlite::Result<SteamConfig> {
    let get = |k: &str| -> rusqlite::Result<Option<String>> { Ok(db::get_setting(conn, k)?.filter(|v| !v.is_empty())) };
    Ok(SteamConfig {
        enabled: get(KEY_ENABLED)?.as_deref() == Some("true"),
        api_key: get(KEY_API_KEY)?,
        steam_id: get(KEY_STEAM_ID)?,
    })
}

/// Clé et compte, seulement si la synchronisation est activée et configurée.
fn credentials(conn: &Connection) -> Result<(String, String), String> {
    let c = config(conn).map_err(|e| e.to_string())?;
    if !c.enabled {
        return Err("La synchronisation des succès Steam est désactivée (Paramètres).".into());
    }
    match (c.api_key, c.steam_id) {
        (Some(k), Some(id)) => Ok((k, id)),
        _ => Err("Renseigne ta clé API et ton compte Steam dans les Paramètres.".into()),
    }
}

pub fn game_appid(conn: &Connection, game_id: i64) -> rusqlite::Result<Option<u32>> {
    Ok(conn
        .query_row("SELECT steam_appid FROM games WHERE id = ?1", [game_id], |r| r.get::<_, Option<u32>>(0))
        .optional()?
        .flatten())
}

/// Lie un jeu à Steam (ou le délie avec `None` : ses succès sont alors effacés).
pub fn set_game_appid(conn: &Connection, game_id: i64, appid: Option<u32>) -> rusqlite::Result<bool> {
    let Some(old) = conn
        .query_row("SELECT steam_appid FROM games WHERE id = ?1", [game_id], |r| r.get::<_, Option<u32>>(0))
        .optional()?
    else {
        return Ok(false);
    };
    if old != appid {
        // Autre jeu Steam (ou délié) : les anciens succès ne correspondent plus.
        conn.execute("DELETE FROM steam_achievements WHERE game_id = ?1", [game_id])?;
        conn.execute(
            "UPDATE games SET steam_appid = ?2, steam_synced_at = NULL WHERE id = ?1",
            params![game_id, appid],
        )?;
    }
    Ok(true)
}

// ====================================================================== Partie Internet

fn agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .http_status_as_error(false)
        .timeout_global(Some(Duration::from_secs(25)))
        .user_agent("MeikiTracker")
        .build()
        .into()
}

const NETWORK_ERROR: &str = "Impossible de joindre Steam. Vérifie ta connexion Internet.";

fn get_json(agent: &ureq::Agent, path: &str, query: &[(&str, &str)]) -> Result<(u16, Value), String> {
    let mut resp = agent
        .get(format!("{API}/{path}"))
        .query_pairs(query.iter().copied())
        .call()
        .map_err(|_| NETWORK_ERROR.to_string())?;
    let status = resp.status().as_u16();
    let body = resp.body_mut().read_json::<Value>().unwrap_or(Value::Null);
    Ok((status, body))
}

fn http_error(status: u16) -> String {
    match status {
        401 | 403 => "Steam refuse la clé API. Vérifie-la (32 caractères) sur steamcommunity.com/dev/apikey.".into(),
        429 => "Steam limite le nombre de demandes. Réessaie dans quelques minutes.".into(),
        500..=599 => "Les serveurs de Steam ne répondent pas pour le moment. Réessaie plus tard.".into(),
        s => format!("Réponse inattendue de Steam (code {s})."),
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountCheck {
    pub persona: String,
    /// Profil public : nécessaire pour que Steam donne les succès.
    pub public: bool,
}

/// Vérifie la clé et le compte (un seul appel à Steam).
pub fn check_account(api_key: &str, steam_id: &str) -> Result<AccountCheck, String> {
    let (status, body) = get_json(&agent(), "ISteamUser/GetPlayerSummaries/v2/", &[("key", api_key), ("steamids", steam_id)])?;
    if status != 200 {
        return Err(http_error(status));
    }
    let player = body["response"]["players"]
        .as_array()
        .and_then(|p| p.first())
        .ok_or("Aucun compte Steam ne correspond à cet identifiant.")?;
    Ok(AccountCheck {
        persona: player["personaname"].as_str().unwrap_or("").to_string(),
        public: player["communityvisibilitystate"].as_i64() == Some(3),
    })
}

/// Un succès tel que renvoyé par Steam.
#[derive(Debug, Clone, PartialEq)]
pub struct RemoteAchievement {
    pub api_name: String,
    pub name: String,
    pub description: Option<String>,
    pub hidden: bool,
    pub icon_url: Option<String>,
    pub achieved: bool,
    pub unlocked_at: Option<i64>,
    pub global_percent: Option<f64>,
}

fn player_error(msg: &str) -> String {
    let m = msg.to_lowercase();
    if m.contains("not public") || m.contains("private") {
        "Ton profil Steam est privé : dans Steam, Profil → Modifier le profil → Confidentialité, mets « Détails des jeux » sur Public.".into()
    } else if m.contains("no stats") {
        "Ce jeu n'a pas de succès Steam.".into()
    } else {
        format!("Steam a répondu : {msg}")
    }
}

fn as_percent(v: &Value) -> Option<f64> {
    v.as_f64().or_else(|| v.as_str().and_then(|s| s.parse().ok()))
}

/// Succès d'un jeu pour ce compte : liste complète (schéma) + état du joueur + rareté.
pub fn fetch_achievements(api_key: &str, steam_id: &str, appid: u32) -> Result<Vec<RemoteAchievement>, String> {
    let agent = agent();
    let app = appid.to_string();

    let (status, schema) = get_json(&agent, "ISteamUserStats/GetSchemaForGame/v2/", &[("key", api_key), ("appid", &app), ("l", LANG)])?;
    if status != 200 {
        return Err(http_error(status));
    }
    let list = schema["game"]["availableGameStats"]["achievements"].as_array().cloned().unwrap_or_default();
    if list.is_empty() {
        return Err("Ce jeu n'a pas de succès Steam.".into());
    }

    let (status, player) = get_json(
        &agent,
        "ISteamUserStats/GetPlayerAchievements/v1/",
        &[("key", api_key), ("steamid", steam_id), ("appid", &app), ("l", LANG)],
    )?;
    if let Some(msg) = player["playerstats"]["error"].as_str() {
        return Err(player_error(msg));
    }
    if status != 200 {
        return Err(http_error(status));
    }
    let mut mine: HashMap<String, Option<i64>> = HashMap::new();
    for a in player["playerstats"]["achievements"].as_array().into_iter().flatten() {
        if a["achieved"].as_i64() == Some(1) {
            let at = a["unlocktime"].as_i64().filter(|t| *t > 0);
            mine.insert(a["apiname"].as_str().unwrap_or("").to_string(), at);
        }
    }

    // Rareté mondiale : facultative, une erreur ici ne bloque rien.
    let mut rarity: HashMap<String, f64> = HashMap::new();
    if let Ok((200, global)) = get_json(&agent, "ISteamUserStats/GetGlobalAchievementPercentagesForApp/v2/", &[("gameid", &app)]) {
        for a in global["achievementpercentages"]["achievements"].as_array().into_iter().flatten() {
            if let (Some(n), Some(p)) = (a["name"].as_str(), as_percent(&a["percent"])) {
                rarity.insert(n.to_string(), p);
            }
        }
    }

    Ok(list
        .iter()
        .filter_map(|a| {
            let api_name = a["name"].as_str()?.to_string();
            let unlocked = mine.get(&api_name);
            Some(RemoteAchievement {
                name: a["displayName"].as_str().filter(|s| !s.is_empty()).unwrap_or(&api_name).to_string(),
                description: a["description"].as_str().filter(|s| !s.trim().is_empty()).map(str::to_string),
                hidden: a["hidden"].as_i64() == Some(1),
                icon_url: a["icon"].as_str().filter(|u| allowed_image_url(u)).map(str::to_string),
                achieved: unlocked.is_some(),
                unlocked_at: unlocked.copied().flatten(),
                global_percent: rarity.get(&api_name).copied(),
                api_name,
            })
        })
        .collect())
}

/// On ne télécharge des images que depuis les serveurs de Steam.
fn allowed_image_url(url: &str) -> bool {
    let Some(rest) = url.strip_prefix("https://") else { return false };
    let host = rest.split('/').next().unwrap_or("").to_lowercase();
    ["steamstatic.com", "akamaihd.net", "steampowered.com", "steamcommunity.com"]
        .iter()
        .any(|d| host == *d || host.ends_with(&format!(".{d}")))
}

/// Télécharge des icônes en parallèle (6 à la fois). Les échecs sont simplement ignorés.
fn download_icons(urls: Vec<String>) -> HashMap<String, Vec<u8>> {
    let agent = agent();
    let queue = Mutex::new(urls);
    let out = Mutex::new(HashMap::new());
    std::thread::scope(|s| {
        for _ in 0..6 {
            s.spawn(|| loop {
                let Some(url) = queue.lock().ok().and_then(|mut q| q.pop()) else { break };
                let Ok(mut resp) = agent.get(&url).call() else { continue };
                if resp.status().as_u16() != 200 {
                    continue;
                }
                if let Ok(bytes) = resp.body_mut().with_config().limit(512 * 1024).read_to_vec() {
                    if let Ok(mut o) = out.lock() {
                        o.insert(url, bytes);
                    }
                }
            });
        }
    });
    out.into_inner().unwrap_or_default()
}

// ====================================================================== Stockage

/// Enregistre les succès d'un jeu. Renvoie le nombre de succès nouvellement débloqués.
pub fn store(conn: &mut Connection, game_id: i64, list: &[RemoteAchievement], icons: &HashMap<String, Vec<u8>>, now: i64) -> rusqlite::Result<usize> {
    let before: HashSet<String> = {
        let mut st = conn.prepare("SELECT api_name FROM steam_achievements WHERE game_id = ?1 AND achieved = 1")?;
        st.query_map([game_id], |r| r.get(0))?.collect::<rusqlite::Result<_>>()?
    };
    let first_sync = conn
        .query_row("SELECT COUNT(*) FROM steam_achievements WHERE game_id = ?1", [game_id], |r| r.get::<_, i64>(0))?
        == 0;
    let tx = conn.transaction()?;
    tx.execute("CREATE TEMP TABLE IF NOT EXISTS keep_names (n TEXT PRIMARY KEY)", [])?;
    tx.execute("DELETE FROM keep_names", [])?;
    for (i, a) in list.iter().enumerate() {
        tx.execute("INSERT OR IGNORE INTO keep_names (n) VALUES (?1)", [&a.api_name])?;
        let icon = a.icon_url.as_ref().and_then(|u| icons.get(u));
        tx.execute(
            "INSERT INTO steam_achievements
               (game_id, api_name, name, description, hidden, icon_url, icon, achieved, unlocked_at, global_percent, sort_order)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
             ON CONFLICT(game_id, api_name) DO UPDATE SET
               name = excluded.name, description = excluded.description, hidden = excluded.hidden,
               icon = CASE WHEN excluded.icon IS NOT NULL THEN excluded.icon
                           WHEN icon_url IS excluded.icon_url THEN icon END,
               icon_url = excluded.icon_url,
               achieved = excluded.achieved, unlocked_at = excluded.unlocked_at,
               global_percent = COALESCE(excluded.global_percent, global_percent),
               sort_order = excluded.sort_order",
            params![game_id, a.api_name, a.name, a.description, a.hidden, a.icon_url, icon, a.achieved, a.unlocked_at, a.global_percent, i as i64],
        )?;
    }
    tx.execute("DELETE FROM steam_achievements WHERE game_id = ?1 AND api_name NOT IN (SELECT n FROM keep_names)", [game_id])?;
    tx.execute("DELETE FROM keep_names", [])?;
    tx.execute("UPDATE games SET steam_synced_at = ?2 WHERE id = ?1", params![game_id, now])?;
    tx.commit()?;
    // Au premier passage, tout est « déjà débloqué » : pas de notification.
    Ok(if first_sync { 0 } else { list.iter().filter(|a| a.achieved && !before.contains(&a.api_name)).count() })
}

/// URLs d'icônes à télécharger (absentes, ou dont l'adresse a changé).
fn missing_icons(conn: &Connection, game_id: i64, list: &[RemoteAchievement]) -> rusqlite::Result<Vec<String>> {
    let mut st = conn.prepare("SELECT icon_url FROM steam_achievements WHERE game_id = ?1 AND icon IS NOT NULL")?;
    let have: HashSet<String> = st.query_map([game_id], |r| r.get::<_, Option<String>>(0))?.filter_map(|r| r.ok().flatten()).collect();
    let mut seen = HashSet::new();
    Ok(list
        .iter()
        .filter_map(|a| a.icon_url.clone())
        .filter(|u| !have.contains(u) && seen.insert(u.clone()))
        .collect())
}

// ====================================================================== Synchronisation

/// Une seule synchronisation à la fois (bouton + automatique).
static SYNC_LOCK: Mutex<()> = Mutex::new(());

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncResult {
    pub total: usize,
    pub unlocked: usize,
    pub new_unlocked: usize,
}

/// Synchronise un jeu. Ouvre sa propre connexion : la base n'est jamais bloquée
/// pendant l'attente du réseau.
pub fn sync_game(db_path: &Path, game_id: i64) -> Result<SyncResult, String> {
    let _guard = SYNC_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let mut conn = db::open(db_path).map_err(|e| e.to_string())?;
    sync_game_with(&mut conn, game_id)
}

fn sync_game_with(conn: &mut Connection, game_id: i64) -> Result<SyncResult, String> {
    let (key, steam_id) = credentials(conn)?;
    let appid = game_appid(conn, game_id).map_err(|e| e.to_string())?.ok_or("Ce jeu n'est pas lié à Steam.")?;
    let list = fetch_achievements(&key, &steam_id, appid)?;
    let wanted = missing_icons(conn, game_id, &list).map_err(|e| e.to_string())?;
    let icons = download_icons(wanted);
    let new_unlocked = store(conn, game_id, &list, &icons, db::now()).map_err(|e| e.to_string())?;
    Ok(SyncResult { total: list.len(), unlocked: list.iter().filter(|a| a.achieved).count(), new_unlocked })
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncAllResult {
    pub games: usize,
    pub new_unlocked: usize,
    /// « Nom du jeu : raison » pour chaque échec.
    pub errors: Vec<String>,
}

/// Synchronise tous les jeux liés à Steam.
pub fn sync_all(db_path: &Path) -> Result<SyncAllResult, String> {
    let _guard = SYNC_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let mut conn = db::open(db_path).map_err(|e| e.to_string())?;
    credentials(&conn)?;
    let linked: Vec<(i64, String)> = {
        let mut st = conn
            .prepare("SELECT id, name FROM games WHERE archived = 0 AND steam_appid IS NOT NULL ORDER BY name")
            .map_err(|e| e.to_string())?;
        st.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
            .and_then(|rows| rows.collect())
            .map_err(|e| e.to_string())?
    };
    let mut result = SyncAllResult { games: 0, new_unlocked: 0, errors: Vec::new() };
    for (id, name) in linked {
        match sync_game_with(&mut conn, id) {
            Ok(r) => {
                result.games += 1;
                result.new_unlocked += r.new_unlocked;
            }
            Err(e) => {
                let network = e == NETWORK_ERROR;
                result.errors.push(format!("{name} : {e}"));
                if network {
                    break; // pas d'Internet : inutile d'insister pour les autres jeux
                }
            }
        }
    }
    db::set_setting(&conn, KEY_LAST_SYNC, &db::now().to_string()).map_err(|e| e.to_string())?;
    Ok(result)
}

// ====================================================================== Lecture pour l'interface

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AchievementRow {
    pub api_name: String,
    pub name: String,
    pub description: Option<String>,
    pub hidden: bool,
    /// Icône en « data URL ».
    pub icon: Option<String>,
    pub achieved: bool,
    pub unlocked_at: Option<i64>,
    pub global_percent: Option<f64>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AchievementsView {
    /// Synchronisation activée dans les Paramètres.
    pub enabled: bool,
    pub appid: Option<u32>,
    pub synced_at: Option<i64>,
    pub total: usize,
    pub unlocked: usize,
    pub items: Vec<AchievementRow>,
}

fn image_data_url(bytes: &[u8]) -> String {
    use base64::Engine;
    let mime = if bytes.starts_with(&[0x89, b'P', b'N', b'G']) { "image/png" } else { "image/jpeg" };
    format!("data:{mime};base64,{}", base64::engine::general_purpose::STANDARD.encode(bytes))
}

/// Succès d'un jeu : débloqués (du plus récent au plus ancien), puis les autres.
pub fn view(conn: &Connection, game_id: i64) -> rusqlite::Result<AchievementsView> {
    let enabled = config(conn)?.enabled;
    let (appid, synced_at): (Option<u32>, Option<i64>) = conn
        .query_row("SELECT steam_appid, steam_synced_at FROM games WHERE id = ?1", [game_id], |r| Ok((r.get(0)?, r.get(1)?)))
        .optional()?
        .unwrap_or((None, None));
    let mut st = conn.prepare(
        "SELECT api_name, name, description, hidden, icon, achieved, unlocked_at, global_percent
         FROM steam_achievements WHERE game_id = ?1
         ORDER BY achieved DESC,
                  CASE WHEN achieved = 1 THEN -COALESCE(unlocked_at, 0) END,
                  sort_order",
    )?;
    let items: Vec<AchievementRow> = st
        .query_map([game_id], |r| {
            let icon: Option<Vec<u8>> = r.get(4)?;
            Ok(AchievementRow {
                api_name: r.get(0)?,
                name: r.get(1)?,
                description: r.get(2)?,
                hidden: r.get(3)?,
                icon: icon.map(|b| image_data_url(&b)),
                achieved: r.get(5)?,
                unlocked_at: r.get(6)?,
                global_percent: r.get(7)?,
            })
        })?
        .collect::<rusqlite::Result<_>>()?;
    Ok(AchievementsView {
        enabled,
        appid,
        synced_at,
        total: items.len(),
        unlocked: items.iter().filter(|a| a.achieved).count(),
        items,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lecture_vdf() {
        let src = r#"
"users"
{
	"76561198000000001"
	{
		"AccountName"		"compte1"
		"PersonaName"		"Ancien"
		"MostRecent"		"0"
	}
	// commentaire
	"76561198000000002"
	{
		"AccountName"		"compte2"
		"PersonaName"		"Meiki \"Toow\""
		"MostRecent"		"1"
	}
}"#;
        let root = parse_vdf(src);
        let users = root.get("users").unwrap();
        assert_eq!(users.entries().len(), 2);
        assert_eq!(users.get("76561198000000002").unwrap().str("PersonaName"), Some("Meiki \"Toow\""));
    }

    fn fake_steam(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("meiki-steam-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(d.join("config")).unwrap();
        std::fs::create_dir_all(d.join("steamapps")).unwrap();
        d
    }

    #[test]
    fn compte_et_jeux_installes() {
        let steam = fake_steam("lib");
        let other = steam.join("AutreBibli");
        std::fs::create_dir_all(other.join("steamapps").join("common").join("Jeu B").join("bin")).unwrap();
        std::fs::write(other.join("steamapps").join("common").join("Jeu B").join("bin").join("JeuB.exe"), b"").unwrap();
        std::fs::write(
            steam.join("config").join("loginusers.vdf"),
            "\"users\" { \"76561198000000001\" { \"PersonaName\" \"A\" \"MostRecent\" \"0\" } \"76561198000000002\" { \"PersonaName\" \"B\" \"MostRecent\" \"1\" } }",
        )
        .unwrap();
        let other_esc = other.to_string_lossy().replace('\\', "\\\\");
        std::fs::write(
            steam.join("steamapps").join("libraryfolders.vdf"),
            format!("\"libraryfolders\" {{ \"0\" {{ \"path\" \"{}\" }} \"1\" {{ \"path\" \"{other_esc}\" }} }}", steam.to_string_lossy().replace('\\', "\\\\")),
        )
        .unwrap();
        std::fs::write(steam.join("steamapps").join("appmanifest_10.acf"), "\"AppState\" { \"appid\" \"10\" \"name\" \"Zeta\" \"installdir\" \"Zeta\" }").unwrap();
        std::fs::write(steam.join("steamapps").join("appmanifest_228980.acf"), "\"AppState\" { \"appid\" \"228980\" \"name\" \"Redist\" \"installdir\" \"x\" }").unwrap();
        std::fs::write(other.join("steamapps").join("appmanifest_20.acf"), "\"AppState\" { \"appid\" \"20\" \"name\" \"Alpha\" \"installdir\" \"Jeu B\" }").unwrap();

        assert_eq!(detect_account_in(&steam), Some(SteamAccount { steam_id: "76561198000000002".into(), persona: "B".into() }));
        let games = installed_games_in(&steam);
        assert_eq!(games.iter().map(|g| (g.appid, g.name.as_str())).collect::<Vec<_>>(), vec![(20, "Alpha"), (10, "Zeta")]);
        let exe = other.join("steamapps").join("common").join("Jeu B").join("bin").join("JeuB.exe");
        assert_eq!(appid_for_exe(&exe, &games), Some(20));
        assert_eq!(appid_for_exe(Path::new(r"C:\Ailleurs\JeuB.exe"), &games), None);
        assert_eq!(appid_for_process("jeub.EXE", &games), Some(20));
        assert_eq!(appid_for_process("inconnu.exe", &games), None);
    }

    fn ach(name: &str, achieved: bool, url: &str) -> RemoteAchievement {
        RemoteAchievement {
            api_name: name.into(),
            name: name.to_uppercase(),
            description: None,
            hidden: false,
            icon_url: Some(url.into()),
            achieved,
            unlocked_at: achieved.then_some(100),
            global_percent: Some(12.5),
        }
    }

    #[test]
    fn stockage_et_nouveaux_succes() {
        let mut c = db::open_in_memory().unwrap();
        let g = db::add_game(&c, "Jeu", "jeu.exe", "game", 0).unwrap();
        set_game_appid(&c, g, Some(10)).unwrap();
        let icons: HashMap<String, Vec<u8>> = [("u1".to_string(), vec![1u8])].into();
        let first = vec![ach("a", true, "u1"), ach("b", false, "u2")];
        assert_eq!(store(&mut c, g, &first, &icons, 1).unwrap(), 0, "premier passage : pas de « nouveau »");
        assert_eq!(missing_icons(&c, g, &first).unwrap(), vec!["u2".to_string()]);

        let second = vec![ach("a", true, "u1"), ach("b", true, "u2"), ach("c", false, "u3")];
        assert_eq!(store(&mut c, g, &second, &HashMap::new(), 2).unwrap(), 1);
        let v = view(&c, g).unwrap();
        assert_eq!((v.total, v.unlocked, v.synced_at, v.appid), (3, 2, Some(2), Some(10)));
        assert!(v.items[0].achieved && v.items[2].name == "C");
        assert!(v.items.iter().find(|i| i.api_name == "a").unwrap().icon.is_some(), "icône gardée");

        // Un succès retiré par le jeu disparaît.
        store(&mut c, g, &second[..2], &HashMap::new(), 3).unwrap();
        assert_eq!(view(&c, g).unwrap().total, 2);

        // Délier efface les succès.
        set_game_appid(&c, g, None).unwrap();
        let v = view(&c, g).unwrap();
        assert_eq!((v.total, v.appid), (0, None));
    }

    #[test]
    fn validations() {
        assert!(valid_steam_id("76561198000000002"));
        assert!(!valid_steam_id("1234"));
        assert!(valid_api_key("0123456789ABCDEF0123456789abcdef"));
        assert!(!valid_api_key("pas-une-cle"));
        assert!(allowed_image_url("https://cdn.akamai.steamstatic.com/steamcommunity/public/images/apps/1/a.jpg"));
        assert!(!allowed_image_url("https://evil.com/steamstatic.com/a.jpg"));
        assert!(!allowed_image_url("http://cdn.akamai.steamstatic.com/a.jpg"));
    }
}
