// Cache des icônes de jeux (chargées une seule fois chacune).
import { getGameIcon } from "$lib/api";

class IconStore {
  private urls = $state<Record<number, string | null>>({});
  private pending = new Set<number>();

  /** Icône du jeu, ou null si inconnue / pas encore chargée (chargement lancé). */
  get(id: number): string | null {
    if (!(id in this.urls) && !this.pending.has(id)) {
      this.pending.add(id);
      getGameIcon(id)
        .then((url) => (this.urls[id] = url))
        .catch(() => (this.urls[id] = null))
        .finally(() => this.pending.delete(id));
    }
    return this.urls[id] ?? null;
  }

  forget(id: number) {
    delete this.urls[id];
  }
}

export const icons = new IconStore();
