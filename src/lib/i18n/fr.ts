// Tous les textes de l'interface, en français.
// Pour ajouter l'anglais plus tard : créer en.ts avec les mêmes clés.
export const fr = {
  app: {
    name: "MEIKI TRACKER",
    by: "BY MEIKITOOW",
    loading: "Chargement…",
    error: "Impossible de charger les données",
    retry: "Réessayer",
  },
  nav: {
    library: "Bibliothèque",
    stats: "Statistiques",
    settings: "Paramètres",
  },
  tracking: {
    active: "Tracking actif",
    paused: "Tracking en pause",
    every: (s: number) => `Vérification toutes les ${s} s`,
    playing: (name: string) => `En jeu : ${name}`,
  },
  library: {
    title: "Ta bibliothèque",
    count: (n: number) => (n === 0 ? "Aucun jeu suivi" : n === 1 ? "1 jeu suivi" : `${n} jeux suivis`),
    add: "Ajouter un jeu",
    addSoon: "Arrive à l'étape 4",
    live: "En jeu",
    afk: "AFK — chrono en pause",
    tiles: {
      week: "Cette semaine",
      weekVs: (p: number) => `${p > 0 ? "+" : ""}${p} % vs semaine dernière`,
      weekNoRef: "Pas de référence la semaine dernière",
      total: "Total tous jeux",
      totalSub: (tracked: string) => `dont ${tracked} trackées`,
      streak: "Série",
      streakValue: (n: number) => (n <= 1 ? `${n} jour` : `${n} jours`),
      streakRecord: (n: number) => `Record : ${n <= 1 ? `${n} jour` : `${n} jours`}`,
    },
    sort: "Trier",
    sortTotal: "Temps total",
    sortRecent: "Dernier lancement",
    card: {
      sub: (tracked: string, baseline: string) => `dont ${tracked} trackées · départ ${baseline}`,
      week: "Cette semaine",
      weekLimit: (week: string, limit: string) => `${week} / ${limit}`,
    },
    empty: {
      title: "Aucun jeu pour l'instant",
      text: "Lance ton jeu, puis clique sur « Ajouter un jeu » pour commencer à suivre ton temps.",
    },
  },
  game: {
    back: "Retour à la bibliothèque",
    soon: "La fiche détaillée du jeu arrive à l'étape 5.",
  },
  soon: "Bientôt disponible",
} as const;

export type Dictionnaire = typeof fr;
