// Liens externes de l'appli. Chaque adresse doit aussi être autorisée dans
// src-tauri/capabilities/default.json (opener:allow-open-url), sinon elle est refusée.

/** Code source (GitHub). */
export const REPO_URL = "https://github.com/Meikitoo/meiki-tracker";
export const RELEASES_URL = `${REPO_URL}/releases`;

/**
 * Lien de don (Ko-fi, PayPal, Tipeee…). Vide : le bouton « Soutenir » est masqué.
 * Pour l'activer : mettre l'adresse ici ET l'ajouter dans capabilities/default.json.
 */
export const DONATE_URL = "";
