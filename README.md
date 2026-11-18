# Meiki Tracker

Application Windows qui mesure automatiquement le temps passé sur chaque jeu (et logiciel),
quel que soit le launcher — y compris Battle.net / Call of Duty, qui n'affiche pas le temps de jeu.

- **Sans risque pour l'anti-triche** : l'appli lit uniquement la liste des programmes ouverts
  (comme le Gestionnaire des tâches). Pas de lecture mémoire, pas d'injection, pas de hook.
- **100 % local** : aucune donnée envoyée sur Internet, pas de télémétrie.
- **Zéro perte** : la session en cours est enregistrée toutes les 5 s et récupérée après un crash.
- **Léger** : une vérification toutes les 5 s (≈ 0,01 % de processeur).

## Installer

Lancer `Meiki Tracker_1.0.0_x64-setup.exe` (installation pour l'utilisateur courant, sans droits administrateur).

## Développer

Prérequis : Rust (stable, MSVC), Node.js, Visual Studio Build Tools (C++), WebView2.

```bash
npm install
npm run tauri dev      # lancer l'appli en développement
npm run tauri build    # créer l'installateur (src-tauri/target/release/bundle/nsis/)
cd src-tauri && cargo test   # tests de la logique de suivi
npm run check          # vérification TypeScript / Svelte
```

En développement uniquement, la variable `MEIKI_DB` permet d'utiliser une base de test
au lieu de la vraie (`%APPDATA%\MeikiTracker\meiki.db`).

Outil console de debug (sans interface) :

```bash
cd src-tauri
cargo run --bin meiki-debug          # liste des commandes
cargo run --bin meiki-debug -- suivre
```

## Organisation du code

| Dossier / fichier | Rôle |
|---|---|
| `src-tauri/src/tracker.rs` | Moteur de sessions (AFK, fusion, crash, veille) — testé |
| `src-tauri/src/service.rs` | Boucle de fond (toutes les 5 s) |
| `src-tauri/src/system.rs`, `exeinfo.rs` | Lecture des processus, inactivité, icônes des .exe |
| `src-tauri/src/db.rs` | Base SQLite et schéma |
| `src-tauri/src/library.rs`, `detail.rs` | Calculs pour les écrans |
| `src-tauri/src/backup.rs` | Export, import, sauvegarde automatique |
| `src-tauri/src/desktop.rs` | Zone de notification, notifications, fermeture = cacher |
| `src-tauri/src/commands.rs` | Commandes appelées par l'interface |
| `src/routes/` | Écrans : Bibliothèque, fiche d'un jeu, Statistiques, Paramètres |
| `src/lib/i18n/fr.ts` | Tous les textes de l'interface (ajouter `en.ts` pour l'anglais) |
| `src-tauri/icons/source/logo.svg` | Logo source (`npx tauri icon` pour régénérer les icônes) |
