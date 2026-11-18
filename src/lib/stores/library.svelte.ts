// État partagé de la bibliothèque + chrono en direct.
import { getLibrary, onTrackerChanged, onTrackerTick, type Library, type LiveInfo } from "$lib/api";

class LibraryStore {
  data = $state<Library | null>(null);
  error = $state<string | null>(null);
  live = $state<LiveInfo[]>([]);
  paused = $state(false);
  /** Instant (s) auquel correspondent les `netSecs` de `live`. */
  liveAt = $state(0);
  /** Horloge locale, mise à jour chaque seconde. */
  now = $state(Math.floor(Date.now() / 1000));

  private started = false;

  async refresh() {
    try {
      const lib = await getLibrary();
      this.data = lib;
      this.live = lib.live;
      this.liveAt = lib.at;
      this.paused = lib.paused;
      this.error = null;
    } catch (e) {
      this.error = String(e);
    }
  }

  /** À appeler une fois au démarrage de l'interface. */
  async start() {
    if (this.started) return;
    this.started = true;
    await this.refresh();
    await onTrackerChanged(() => this.refresh());
    await onTrackerTick((tick) => {
      this.live = tick.live;
      this.liveAt = tick.at;
      this.paused = tick.paused;
    });
    setInterval(() => (this.now = Math.floor(Date.now() / 1000)), 1000);
    // Recalcul périodique (changement de jour/semaine, etc.)
    setInterval(() => this.refresh(), 60_000);
  }

  /** Temps compté d'une session en cours, à la seconde près. */
  liveSecs(l: LiveInfo): number {
    return l.afk ? l.netSecs : l.netSecs + Math.max(0, this.now - this.liveAt);
  }

  liveFor(gameId: number): LiveInfo | undefined {
    return this.live.find((l) => l.gameId === gameId);
  }
}

export const library = new LibraryStore();
