# Meiki Tracker

Application Windows qui mesure automatiquement le temps passé sur chaque jeu (et logiciel),
quel que soit le launcher — y compris Battle.net / Call of Duty, qui n'affiche pas le temps de jeu.

- **Sans risque pour l'anti-triche** : l'appli lit uniquement la liste des programmes ouverts
  (comme le Gestionnaire des tâches). Pas de lecture mémoire, pas d'injection, pas de hook.
- **100 % local** : aucune donnée envoyée sur Internet, pas de télémétrie.
  Seule exception, **désactivée par défaut** : la synchronisation des succès Steam (voir plus bas).
- **Zéro perte** : la session en cours est enregistrée toutes les 5 s et récupérée après un crash.
- **Léger** : une vérification toutes les 5 s (≈ 0,01 % de processeur).

## Succès Steam (option)

Dans Paramètres → Succès Steam, tu peux afficher tes succès sur la fiche de chaque jeu Steam.
C'est la seule fonction qui utilise Internet, et seulement si tu l'actives :

- l'appli contacte uniquement les serveurs de Steam (`api.steampowered.com` et ses images) ;
- elle utilise ta propre clé API Steam (gratuite : https://steamcommunity.com/dev/apikey) ;
- la clé reste sur ton PC et n'est **jamais** copiée dans les sauvegardes ;
- ton profil Steam doit avoir « Détails des jeux » en public (sinon Steam ne donne pas les succès).

Le compte Steam et les jeux installés sont détectés en lisant les fichiers de Steam sur le PC (sans Internet).
La synchro se fait quand tu fermes un jeu lié, au lancement de l'appli (au plus toutes les 12 h) ou avec le bouton.

## Installer

Lancer `Meiki Tracker_1.1.0_x64-setup.exe` (installation pour l'utilisateur courant, sans droits administrateur).

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
| `src-tauri/src/steam.rs` | Succès Steam : lecture locale de Steam + synchro (option) |
| `src-tauri/src/backup.rs` | Export, import, sauvegarde automatique |
| `src-tauri/src/desktop.rs` | Zone de notification, notifications, fermeture = cacher |
| `src-tauri/src/commands.rs` | Commandes appelées par l'interface |
| `src/routes/` | Écrans : Bibliothèque, fiche d'un jeu, Statistiques, Paramètres |
| `src/lib/i18n/fr.ts` | Tous les textes de l'interface (ajouter `en.ts` pour l'anglais) |
| `src-tauri/icons/source/logo.svg` | Logo source (`npx tauri icon` pour régénérer les icônes) |
