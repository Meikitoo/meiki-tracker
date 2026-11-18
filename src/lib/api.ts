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

export const setTrackingPaused = (paused: boolean) => mutate("set_tracking_paused", { paused });
export const setAutostart = (enabled: boolean) => mutate("set_autostart", { enabled });
export const setNotifications = (enabled: boolean) => mutate("set_notifications", { enabled });

export interface AppSettings {
  checkIntervalSecs: number;
  minSessionSecs: number;
  afkThresholdSecs: number;
  mergeWindowSecs: number;
  autostart: boolean;
  notifications: boolean;
  theme: string;
  backupDir: string | null;
}

export interface SettingsView {
  settings: AppSettings;
  autostart: boolean;
  lastAutoBackupAt: number | null;
  suggestedBackupName: string;
}

export async function getSettings(): Promise<SettingsView> {
  if (!inTauri) {
    return {
      settings: { checkIntervalSecs: 5, minSessionSecs: 120, afkThresholdSecs: 600, mergeWindowSecs: 300, autostart: false, notifications: true, theme: "dark", backupDir: null },
      autostart: false,
      lastAutoBackupAt: null,
      suggestedBackupName: "meiki-tracker.meikibackup",
    };
  }
  return invoke<SettingsView>("get_settings");
}

export const updateTrackingSettings = (s: { minSessionSecs: number; afkThresholdSecs: number; mergeWindowSecs: number }) =>
  mutate("update_tracking_settings", { s });
export const setBackupDir = (path: string | null) => mutate("set_backup_dir", { path });

const BACKUP_FILTER = [{ name: "Sauvegarde Meiki Tracker", extensions: ["meikibackup"] }];

/** Exporte vers un fichier choisi par l'utilisateur. Renvoie le chemin, ou null si annulé. */
export async function exportBackup(title: string, defaultName: string): Promise<string | null> {
  if (!inTauri) return null;
  const { save } = await import("@tauri-apps/plugin-dialog");
  const path = await save({ title, defaultPath: defaultName, filters: BACKUP_FILTER });
  if (!path) return null;
  await invoke("export_backup", { path });
  return path;
}

export async function pickBackupFile(title: string): Promise<string | null> {
  if (!inTauri) return null;
  const { open } = await import("@tauri-apps/plugin-dialog");
  const path = await open({ title, multiple: false, directory: false, filters: BACKUP_FILTER });
  return path ?? null;
}

export async function pickFolder(title: string): Promise<string | null> {
  if (!inTauri) return null;
  const { open } = await import("@tauri-apps/plugin-dialog");
  const path = await open({ title, multiple: false, directory: true });
  return path ?? null;
}

export interface ImportSummary {
  games: number;
  sessions: number;
  safetyBackup: string;
}

export async function importBackup(path: string): Promise<ImportSummary> {
  return invoke<ImportSummary>("import_backup", { path });
}

export interface Overview {
  days: DayStat[];
  games: { id: number; name: string; weekSecs: number; totalSecs: number }[];
  at: number;
}

export async function getOverview(): Promise<Overview> {
  if (!inTauri) {
    const d = (await mock()).mockDetail(1)!;
    const lib = (await mock()).mockLibrary();
    return { days: d.days, games: lib.games.map((g) => ({ id: g.id, name: g.name, weekSecs: g.weekSecs, totalSecs: g.totalSecs })), at: d.at };
  }
  return invoke<Overview>("get_overview");
}

// ------------------------------------------------------------------ Succès Steam (option)

export interface SteamSettings {
  enabled: boolean;
  hasKey: boolean;
  keyHint: string | null;
  steamId: string | null;
  detected: { steamId: string; persona: string } | null;
  lastSyncAt: number | null;
  linkedGames: number;
}

export interface Achievement {
  apiName: string;
  name: string;
  description: string | null;
  hidden: boolean;
  icon: string | null;
  achieved: boolean;
  unlockedAt: number | null;
  globalPercent: number | null;
}

export interface Achievements {
  enabled: boolean;
  appid: number | null;
  syncedAt: number | null;
  total: number;
  unlocked: number;
  items: Achievement[];
}

export interface SyncResult {
  total: number;
  unlocked: number;
  newUnlocked: number;
}

export async function getSteamSettings(): Promise<SteamSettings> {
  if (!inTauri) return { enabled: false, hasKey: false, keyHint: null, steamId: null, detected: null, lastSyncAt: null, linkedGames: 0 };
  return invoke<SteamSettings>("get_steam_settings");
}
export const setSteamEnabled = (enabled: boolean) => mutate("set_steam_enabled", { enabled });
export const forgetSteamAccount = () => mutate("forget_steam_account", {});

export async function saveSteamAccount(apiKey: string | null, steamId: string): Promise<{ persona: string; public: boolean }> {
  return invoke("save_steam_account", { apiKey, steamId });
}
export async function steamSyncAll(): Promise<{ games: number; newUnlocked: number; errors: string[] }> {
  return invoke("steam_sync_all");
}
export async function steamSyncGame(id: number): Promise<SyncResult> {
  return invoke("steam_sync_game", { id });
}
export async function getAchievements(id: number): Promise<Achievements> {
  if (!inTauri) return { enabled: false, appid: null, syncedAt: null, total: 0, unlocked: 0, items: [] };
  return invoke<Achievements>("get_achievements", { id });
}
export async function steamLinkCandidates(id: number): Promise<{ detected: number | null; installed: { appid: number; name: string }[] }> {
  if (!inTauri) return { detected: null, installed: [] };
  return invoke("steam_link_candidates", { id });
}
export async function linkSteamGame(id: number, appid: number | null): Promise<SyncResult | null> {
  return invoke("link_steam_game", { id, appid });
}

/** Ouvre une page web dans le navigateur (liste d'adresses autorisées côté Tauri). */
export async function openWeb(url: string): Promise<void> {
  if (!inTauri) {
    window.open(url, "_blank");
    return;
  }
  const { openUrl } = await import("@tauri-apps/plugin-opener");
  await openUrl(url);
}

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
