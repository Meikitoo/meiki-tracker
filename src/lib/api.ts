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
  // Hors appli : aperçu navigateur en développement seulement (retiré de la version finale).
  if (!inTauri) return (await mock()).mockLibrary();
  return invoke<Library>("get_library");
}

export interface ProgramCandidate {
  processName: string;
  exePath: string | null;
  displayName: string;
  icon: string | null;
  windowed: boolean;
  alreadyTracked: boolean;
}

export type Category = "game" | "creation";

export interface NewGame {
  processName: string;
  name: string;
  category: Category;
  baselineSecs: number;
  exePath: string | null;
}

async function mock() {
  if (!import.meta.env.DEV) throw new Error("Meiki Tracker doit être lancé comme application.");
  return import("./mock");
}

export async function listRunningPrograms(includeBackground: boolean): Promise<ProgramCandidate[]> {
  if (!inTauri) return (await mock()).mockPrograms(includeBackground);
  return invoke<ProgramCandidate[]>("list_running_programs", { includeBackground });
}

/** Ouvre le sélecteur de fichier Windows et renvoie les infos du .exe choisi (ou null si annulé). */
export async function browseExe(dialogTitle: string): Promise<ProgramCandidate | null> {
  if (!inTauri) return (await mock()).mockPrograms(false)[0];
  const { open } = await import("@tauri-apps/plugin-dialog");
  const path = await open({
    title: dialogTitle,
    multiple: false,
    directory: false,
    filters: [{ name: "Programme", extensions: ["exe"] }],
  });
  if (!path) return null;
  return invoke<ProgramCandidate>("describe_exe", { path });
}

export async function addGame(game: NewGame): Promise<{ id: number; restored: boolean }> {
  if (!inTauri) return { id: 99, restored: false };
  return invoke("add_game", { game });
}

export interface GameInfo {
  id: number;
  name: string;
  processName: string;
  category: Category;
  baselineSecs: number;
  baselineSetAt: number | null;
  weeklyLimitSecs: number | null;
  addedAt: number;
}

export interface DayStat {
  dayStart: number;
  secs: number;
}

export interface SessionRow {
  id: number;
  startedAt: number;
  endedAt: number;
  durationSecs: number;
  afkSecs: number;
  gapSecs: number;
  tag: string | null;
  note: string | null;
}

export interface GameDetail {
  game: GameInfo;
  trackedSecs: number;
  totalSecs: number;
  sessionCount: number;
  longestSecs: number;
  days: DayStat[];
  sessions: SessionRow[];
  pending: { startedAt: number; endedAt: number; netSecs: number } | null;
  live: boolean;
  at: number;
}

export async function getGameDetail(id: number): Promise<GameDetail | null> {
  if (!inTauri) return (await mock()).mockDetail(id);
  return invoke<GameDetail | null>("get_game_detail", { id });
}

/** Appel d'une commande de modification (en aperçu navigateur : sans effet). */
async function mutate(cmd: string, args: Record<string, unknown>): Promise<void> {
  if (!inTauri) return;
  await invoke(cmd, args);
}

export const renameGame = (id: number, name: string) => mutate("rename_game", { id, name });
export const setBaseline = (id: number, secs: number) => mutate("set_baseline", { id, secs });
export const setWeeklyLimit = (id: number, secs: number | null) => mutate("set_weekly_limit", { id, secs });
export const archiveGame = (id: number) => mutate("archive_game", { id });
export const updateSession = (id: number, tag: string | null, note: string | null) =>
  mutate("update_session", { id, tag, note });
export const deleteSession = (id: number) => mutate("delete_session", { id });

export async function getGameIcon(id: number): Promise<string | null> {
  if (!inTauri) return null;
  return invoke<string | null>("get_game_icon", { id });
}

export async function onTrackerChanged(cb: () => void): Promise<UnlistenFn> {
  if (!inTauri) return () => {};
  return listen("tracker-changed", () => cb());
}

export async function onTrackerTick(cb: (tick: LiveTick) => void): Promise<UnlistenFn> {
  if (!inTauri) return () => {};
  return listen<LiveTick>("tracker-tick", (e) => cb(e.payload));
}
