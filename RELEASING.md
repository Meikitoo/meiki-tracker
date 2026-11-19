# Publier une nouvelle version

## Une fois pour toutes

- La **clé de signature des mises à jour** est dans `%USERPROFILE%\.tauri\meiki-tracker.key`.
  - Elle ne doit **jamais** être publiée, ni envoyée à qui que ce soit.
  - **Fais-en une copie de sauvegarde**, par exemple sur une clé USB ou dans un gestionnaire de mots de passe.
    Si elle est perdue, les utilisateurs déjà installés ne pourront plus recevoir de mise à jour automatique :
    ils devraient réinstaller à la main.
- La clé publique correspondante est dans `src-tauri/tauri.conf.json` (`plugins.updater.pubkey`).
  Celle-là peut être publiée sans risque.

## À chaque version

1. Change le numéro de version, le même dans les 3 fichiers :
   - `src-tauri/tauri.conf.json` ;
   - `src-tauri/Cargo.toml` ;
   - `package.json`.
2. Vérifie que tout passe :
   ```bash
   npm run check
   cd src-tauri && cargo test
   ```
3. Commite, puis pousse sur GitHub.
4. Prépare la version :
   ```bash
   node scripts/release.mjs "Ce qui change dans cette version"
   ```
   Le script compile l'installateur signé et prépare `release/vX.Y.Z/`, avec l'installateur et `latest.json`.
5. Sur GitHub → **Releases → Draft a new release** :
   - tag `vX.Y.Z` ;
   - glisse les deux fichiers ;
   - clique sur **Publish release**.

Les utilisateurs voient alors « Mise à jour X.Y.Z disponible » dans Paramètres → À propos
(ou au démarrage, s'ils ont activé l'option).

## Remarque

`npm run tauri build` seul échoue sans la clé (« A public key has been found, but no private key »).
C'est normal : pour un installateur publiable, passe toujours par `scripts/release.mjs`.
