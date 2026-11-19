//! Langue des textes produits côté Rust (erreurs, notifications, icône de la zone
//! de notification). L'interface a ses propres fichiers de textes (src/lib/i18n).

use crate::db;
use rusqlite::Connection;
use std::sync::atomic::{AtomicBool, Ordering};

pub const KEY_LANGUAGE: &str = "language";

static ENGLISH: AtomicBool = AtomicBool::new(false);

pub fn is_en() -> bool {
    ENGLISH.load(Ordering::Relaxed)
}

/// Texte dans la langue active.
pub fn tr(fr: &'static str, en: &'static str) -> &'static str {
    if is_en() { en } else { fr }
}

/// "fr" ou "en" ; toute autre valeur est ignorée.
pub fn set(lang: &str) {
    match lang {
        "en" => ENGLISH.store(true, Ordering::Relaxed),
        "fr" => ENGLISH.store(false, Ordering::Relaxed),
        _ => {}
    }
}

/// Langue enregistrée (None : jamais choisie, l'interface la détecte au premier lancement).
pub fn saved(conn: &Connection) -> Option<String> {
    db::get_setting(conn, KEY_LANGUAGE).ok().flatten().filter(|l| l == "fr" || l == "en")
}

/// Applique la langue enregistrée (au démarrage).
pub fn load(conn: &Connection) {
    if let Some(l) = saved(conn) {
        set(&l);
    }
}

/// `format!` dans la langue active : `trf!("Bonjour {nom}", "Hello {nom}")`.
#[macro_export]
macro_rules! trf {
    ($fr:literal, $en:literal $(, $arg:expr)* $(,)?) => {
        if $crate::i18n::is_en() { format!($en $(, $arg)*) } else { format!($fr $(, $arg)*) }
    };
}
