// Pont entre l'interface et le backend Rust.
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export interface GameCard {
  id: number;
  name: string;
  processName: string;
  category: string;
  baselineSecs: number;
  trackedSecs: number;
  totalSecs: number;
  weekSecs: number;
  weeklyLimitSecs: number | null;
  lastPlayedAt: number | null;
  sessionCount: number;
  live: boolean;
}

export interface Summary {
  weekSecs: number;
  lastWeekSecs: number;
  totalSecs: number;
  trackedSecs: number;
  streakDays: number;
  streakRecord: number;
  weekStart: number;
}

export interface LiveInfo {
  gameId: number;
  name: string;
  startedAt: number;
  netSecs: number;
  afk: boolean;
}

export interface Library {
  games: GameCard[];
  summary: Summary;
  live: LiveInfo[];
  paused: boolean;
  checkIntervalSecs: number;
  at: number;
}

export interface LiveTick {
  live: LiveInfo[];
  paused: boolean;
  at: number;
}

/** Vrai dans l'appli Tauri ; faux dans un simple navigateur (aperçu de développement). */
export const inTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

export async function getLibrary(): Promise<Library> {
  if (!inTauri) {
    // Aperçu navigateur en développement seulement ; retiré de la version finale.
    if (import.meta.env.DEV) return (await import("./mock")).mockLibrary();
    throw new Error("Meiki Tracker doit être lancé comme application.");
  }
  return invoke<Library>("get_library");
}

export async function onTrackerChanged(cb: () => void): Promise<UnlistenFn> {
  if (!inTauri) return () => {};
  return listen("tracker-changed", () => cb());
}

export async function onTrackerTick(cb: (tick: LiveTick) => void): Promise<UnlistenFn> {
  if (!inTauri) return () => {};
  return listen<LiveTick>("tracker-tick", (e) => cb(e.payload));
}
