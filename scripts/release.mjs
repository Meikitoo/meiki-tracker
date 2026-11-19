// Prépare une version publiable de Meiki Tracker.
//
//   node scripts/release.mjs "Notes de version (facultatif)"
//
// 1. compile l'installateur et le signe avec la clé de mise à jour
//    (par défaut %USERPROFILE%\.tauri\meiki-tracker.key, jamais dans le dépôt) ;
// 2. copie dans release/vX.Y.Z/ :
//      - MeikiTracker_X.Y.Z_x64-setup.exe  (nom sans espace : GitHub les remplace)
//      - latest.json                       (lu par l'appli pour proposer la mise à jour)
// 3. affiche les étapes pour publier la Release sur GitHub.
import { spawnSync } from "node:child_process";
import { copyFileSync, existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { homedir } from "node:os";
import { join } from "node:path";

const REPO = "Meikitoo/meiki-tracker";
const root = join(import.meta.dirname, "..");
const conf = JSON.parse(readFileSync(join(root, "src-tauri", "tauri.conf.json"), "utf8"));
const version = conf.version;
const notes = process.argv[2] ?? "";

const keyPath = process.env.MEIKI_SIGNING_KEY ?? join(homedir(), ".tauri", "meiki-tracker.key");
if (!existsSync(keyPath)) {
  console.error(`Clé de signature introuvable : ${keyPath}`);
  console.error("Sans elle, les utilisateurs ne pourraient pas recevoir la mise à jour.");
  process.exit(1);
}

console.log(`\n=== Meiki Tracker ${version} : compilation signée ===\n`);
const build = spawnSync("npm run tauri build", {
  cwd: root,
  stdio: "inherit",
  shell: true,
  env: {
    ...process.env,
    TAURI_SIGNING_PRIVATE_KEY: readFileSync(keyPath, "utf8"),
    TAURI_SIGNING_PRIVATE_KEY_PASSWORD: process.env.TAURI_SIGNING_PRIVATE_KEY_PASSWORD ?? "",
  },
});
if (build.status !== 0) process.exit(build.status ?? 1);

const nsis = join(root, "src-tauri", "target", "release", "bundle", "nsis");
const built = join(nsis, `${conf.productName}_${version}_x64-setup.exe`);
const sig = `${built}.sig`;
for (const f of [built, sig]) {
  if (!existsSync(f)) {
    console.error(`Fichier attendu absent : ${f}`);
    process.exit(1);
  }
}

const tag = `v${version}`;
const assetName = `MeikiTracker_${version}_x64-setup.exe`;
const out = join(root, "release", tag);
mkdirSync(out, { recursive: true });
copyFileSync(built, join(out, assetName));
const latest = {
  version,
  notes,
  pub_date: new Date().toISOString(),
  platforms: {
    "windows-x86_64": {
      signature: readFileSync(sig, "utf8").trim(),
      url: `https://github.com/${REPO}/releases/download/${tag}/${assetName}`,
    },
  },
};
writeFileSync(join(out, "latest.json"), JSON.stringify(latest, null, 2) + "\n");

console.log(`
=== Prêt : ${out} ===

Pour publier sur GitHub :
  1. https://github.com/${REPO}/releases/new
  2. Tag : ${tag}   (créé sur la branche main)
  3. Titre : Meiki Tracker ${version}
  4. Glisse les 2 fichiers : ${assetName} et latest.json
  5. « Publish release » (pas « pre-release » : l'appli lit la dernière release publiée)
`);
