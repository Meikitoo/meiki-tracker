// Langue active de l'interface (français ou anglais).
//
// La langue est choisie une fois au chargement de la page : en changer recharge
// la fenêtre (simple et sans risque d'écran à moitié traduit).
// - Source de vérité : le réglage « language » de la base (partagé avec le Rust).
// - Copie locale (localStorage) pour la connaître dès le premier affichage.
// - Premier lancement : langue de Windows (français → fr, sinon anglais).
import { en } from "./en";
import { fr, type Dictionnaire } from "./fr";

export type Lang = "fr" | "en";
export const LANGS: { id: Lang; label: string }[] = [
  { id: "fr", label: "Français" },
  { id: "en", label: "English" },
];

const STORAGE_KEY = "meiki.lang";

export function detectLang(): Lang {
  if (typeof navigator !== "undefined" && navigator.language?.toLowerCase().startsWith("fr")) return "fr";
  return "en";
}

function initialLang(): Lang {
  try {
    const v = localStorage.getItem(STORAGE_KEY);
    if (v === "fr" || v === "en") return v;
  } catch {
    // stockage indisponible : on détecte
  }
  return typeof window === "undefined" ? "fr" : detectLang();
}

export const lang: Lang = initialLang();
export const t: Dictionnaire = lang === "en" ? en : fr;

export function rememberLang(l: Lang) {
  try {
    localStorage.setItem(STORAGE_KEY, l);
  } catch {
    // tant pis : la base reste la référence
  }
}
