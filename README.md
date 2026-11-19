# Meiki Tracker

**Le temps de jeu que ton launcher ne te montre pas.**
Meiki Tracker mesure automatiquement le temps passé sur chaque jeu (et logiciel), quel que soit le
launcher — y compris Battle.net / Call of Duty, qui n'affiche aucun temps de jeu.

Gratuit, open source, pour Windows. · *[English below](#english)*

[**⬇ Télécharger la dernière version**](https://github.com/Meikitoo/meiki-tracker/releases/latest)

## Pourquoi Meiki Tracker

- **Sans risque pour l'anti-triche** : l'appli lit uniquement la liste des programmes ouverts
  (comme le Gestionnaire des tâches). Pas de lecture mémoire, pas d'injection, pas de hook.
- **Tes données restent chez toi** : tout est stocké sur ton PC, pas de compte, pas de télémétrie.
- **Zéro perte** : la session en cours est enregistrée toutes les 5 s et récupérée après un crash.
- **Léger** : une vérification toutes les 5 s (≈ 0,01 % de processeur).

## Fonctions

- Suivi automatique : la session démarre quand le jeu s'ouvre et s'arrête quand il se ferme.
- Temps AFK retiré (clavier, souris et manette), relances rapprochées fusionnées.
- Bibliothèque : temps total, semaine, série de jours, temps de départ (heures jouées avant l'appli).
- Fiche de chaque jeu : 14 derniers jours, limite hebdomadaire, sessions avec tags et notes.
- **Mon récap** : bilan de la semaine, du mois ou de l'année en image, carrée ou verticale (TikTok, stories).
- **Succès Steam** (option) : progression, rareté, nouveaux succès signalés en fin de partie.
- Sauvegardes : export / import, sauvegarde automatique chaque semaine dans le dossier de ton choix.
- Discret : icône près de l'horloge, lancement avec Windows, notification de fin de session.
- Français et anglais.

## Confidentialité

Meiki Tracker n'envoie **aucune** de tes données sur Internet. Seules deux fonctions se connectent,
et uniquement si tu les utilises :

| Fonction | Contacte | Quand |
|---|---|---|
| Succès Steam | serveurs de Steam (`api.steampowered.com`) | seulement si tu l'actives (désactivé par défaut), avec ta propre clé API Steam |
| Mises à jour | GitHub (ce dépôt) | quand tu cliques sur « Rechercher une mise à jour », ou au démarrage si tu actives l'option (désactivée par défaut) |

La clé API Steam reste sur ton PC et n'est jamais copiée dans les sauvegardes.

## Installer

1. Télécharge `MeikiTracker_x.y.z_x64-setup.exe` dans les [Releases](https://github.com/Meikitoo/meiki-tracker/releases/latest).
2. Lance-le (installation pour ton utilisateur, sans droits administrateur).
3. Windows peut afficher « Windows a protégé votre ordinateur » (SmartScreen) tant que l'installateur
   n'est pas signé : clique sur **Informations complémentaires → Exécuter quand même**.

Les mises à jour suivantes s'installent depuis l'appli (Paramètres → À propos).
Elles sont signées : l'appli refuse tout fichier qui ne vient pas du projet.

## Soutenir

Meiki Tracker est gratuit. Si l'appli te plaît, tu peux soutenir le projet
(le lien de don arrive bientôt), le partager, ou poster ton récap avec **#MeikiTracker**.

## Succès Steam (option)

Dans Paramètres → Succès Steam :

1. active la synchronisation ;
2. crée ta clé API Steam gratuite sur https://steamcommunity.com/dev/apikey (nom de domaine : `localhost`) ;
3. ton compte Steam est détecté automatiquement ;
4. sur la fiche d'un jeu Steam, clique sur « Lier à Steam ».

Ton profil Steam doit avoir « Détails des jeux » en public, sinon Steam ne donne pas les succès.

---

## English

**The play time your launcher doesn't show you.**
Meiki Tracker automatically measures the time you spend in each game (and app), whatever the
launcher — including Battle.net / Call of Duty, which shows no play time at all.
Free, open source, for Windows.

- **Anti-cheat safe**: it only reads the list of running programs (like Task Manager).
  No memory reading, no injection, no hooks.
- **Your data stays with you**: everything is stored on your PC, no account, no telemetry.
  Only two features connect, and only when you use them: Steam achievements (off by default,
  with your own Steam API key) and update checks (GitHub, on click or on startup if you enable it).
- **No lost time**: the running session is saved every 5 s and recovered after a crash.
- **Lightweight**: one check every 5 s.
- Features: AFK removal, session merging, weekly limits, tags and notes, shareable **recap** images,
  Steam achievements, backups, tray icon, start with Windows. French and English.

**Install**: download `MeikiTracker_x.y.z_x64-setup.exe` from the
[Releases](https://github.com/Meikitoo/meiki-tracker/releases/latest). If SmartScreen warns you
(unsigned installer), click **More info → Run anyway**. Later updates install from the app
(Settings → About) and are signature-checked.

---

## Développer / Development

Prérequis : Rust (stable, MSVC), Node.js 20+, Visual Studio Build Tools (C++), WebView2.

```bash
npm install
npm run tauri dev            # lancer l'appli en développement
npm run check                # vérification TypeScript / Svelte
cd src-tauri && cargo test   # tests (suivi, sauvegardes, Steam, récap)
```

Créer une version publiable (installateur signé + `latest.json` pour les mises à jour) :
voir [RELEASING.md](RELEASING.md).

En développement uniquement, la variable `MEIKI_DB` permet d'utiliser une base de test
au lieu de la vraie (`%APPDATA%\MeikiTracker\meiki.db`).

Outil console de debug (jamais inclus dans l'installateur) :

```bash
cd src-tauri
cargo run --bin meiki-debug --features outil-debug          # liste des commandes
```

### Organisation du code

| Dossier / fichier | Rôle |
|---|---|
| `src-tauri/src/tracker.rs` | Moteur de sessions (AFK, fusion, crash, veille) — testé |
| `src-tauri/src/service.rs` | Boucle de fond (toutes les 5 s) |
| `src-tauri/src/system.rs`, `exeinfo.rs` | Lecture des processus, inactivité, icônes des .exe |
| `src-tauri/src/db.rs` | Base SQLite et schéma |
| `src-tauri/src/library.rs`, `detail.rs` | Calculs pour les écrans |
| `src-tauri/src/steam.rs` | Succès Steam : lecture locale de Steam + synchro (option) |
| `src-tauri/src/recap.rs`, `src/lib/recap/card.ts` | « Mon récap » : calculs et dessin de l'image |
| `src-tauri/src/i18n.rs` | Langue des textes côté Rust (erreurs, notifications, menu) |
| `src-tauri/src/backup.rs` | Export, import, sauvegarde automatique |
| `src-tauri/src/desktop.rs` | Zone de notification, notifications, fermeture = cacher |
| `src-tauri/src/commands.rs` | Commandes appelées par l'interface |
| `src/routes/` | Écrans : Bibliothèque, fiche d'un jeu, Statistiques, Mon récap, Paramètres |
| `src/lib/i18n/fr.ts`, `en.ts` | Tous les textes de l'interface, en français et en anglais |
| `src/lib/links.ts` | Liens externes (dépôt, lien de don) |
| `src-tauri/icons/source/logo.svg` | Logo source (`npx tauri icon` pour régénérer les icônes) |

## Licence

[MIT](LICENSE) © 2026 Meikitoow
