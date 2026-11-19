// Mises à jour de l'appli (GitHub Releases, signées). Rien n'est contacté tant
// qu'on n'appelle pas `check()` : bouton dans Paramètres, ou option « au démarrage ».
import { inTauri } from "$lib/api";
import { t } from "$lib/i18n";
import type { Update } from "@tauri-apps/plugin-updater";

type Status = "idle" | "checking" | "none" | "available" | "installing" | "error";

class UpdateStore {
  status = $state<Status>("idle");
  version = $state<string | null>(null);
  notes = $state<string | null>(null);
  error = $state<string | null>(null);
  private update: Update | null = null;

  async check() {
    if (!inTauri || this.status === "checking" || this.status === "installing") return;
    this.status = "checking";
    this.error = null;
    try {
      const { check } = await import("@tauri-apps/plugin-updater");
      const u = await check();
      this.update = u;
      if (u) {
        this.version = u.version;
        this.notes = u.body ?? null;
        this.status = "available";
      } else {
        this.status = "none";
      }
    } catch (e) {
      // Pas de réseau, dépôt pas encore publié… : message simple plutôt que l'erreur technique.
      const msg = String(e);
      this.error = /release JSON|fetch|network|connect|dns|404|timed out/i.test(msg) ? t.settings.updateUnreachable : msg;
      this.status = "error";
    }
  }

  /** Télécharge, vérifie la signature, installe puis redémarre l'appli. */
  async install() {
    if (!this.update) return;
    this.status = "installing";
    try {
      await this.update.downloadAndInstall();
      const { relaunch } = await import("@tauri-apps/plugin-process");
      await relaunch();
    } catch (e) {
      this.error = String(e);
      this.status = "error";
    }
  }
}

export const updates = new UpdateStore();
