// Données factices, utilisées UNIQUEMENT quand l'interface est ouverte dans un
// navigateur (hors appli), pour travailler le design. Jamais dans l'appli réelle.
import type { Library, ProgramCandidate } from "./api";

export function mockPrograms(includeBackground: boolean): ProgramCandidate[] {
  const list: ProgramCandidate[] = [
    { processName: "cod.exe", exePath: "C:\\Call of Duty\\_retail_\\cod.exe", displayName: "Call of Duty", icon: null, windowed: true, alreadyTracked: true },
    { processName: "Discord.exe", exePath: null, displayName: "Discord", icon: null, windowed: true, alreadyTracked: false },
    { processName: "obs64.exe", exePath: null, displayName: "OBS Studio", icon: null, windowed: true, alreadyTracked: false },
    { processName: "VALORANT-Win64-Shipping.exe", exePath: null, displayName: "VALORANT", icon: null, windowed: true, alreadyTracked: false },
  ];
  if (includeBackground) {
    list.push({ processName: "GameLoopService.exe", exePath: null, displayName: "GameLoopService", icon: null, windowed: false, alreadyTracked: false });
  }
  return list;
}

export function mockLibrary(): Library {
  const now = Math.floor(Date.now() / 1000);
  const h = 3600;
  return {
    at: now,
    paused: false,
    checkIntervalSecs: 5,
    live: [{ gameId: 1, name: "Call of Duty", startedAt: now - 6432, netSecs: 6432, afk: false }],
    summary: {
      weekSecs: 14 * h + 20 * 60,
      lastWeekSecs: 12 * h + 5 * 60,
      totalSecs: 618 * h,
      trackedSecs: 38 * h,
      streakDays: 5,
      streakRecord: 12,
      weekStart: now - 3 * 86400,
    },
    games: [
      { id: 1, name: "Call of Duty", processName: "cod.exe", category: "game", baselineSecs: 432 * h, trackedSecs: 21 * h, totalSecs: 453 * h, weekSecs: 9 * h + 10 * 60, weeklyLimitSecs: 10 * h, lastPlayedAt: now - 6432, sessionCount: 14, live: true },
      { id: 2, name: "Valorant", processName: "VALORANT-Win64-Shipping.exe", category: "game", baselineSecs: 120 * h, trackedSecs: 12 * h, totalSecs: 132 * h, weekSecs: 4 * h + 12 * 60, weeklyLimitSecs: null, lastPlayedAt: now - 86400, sessionCount: 8, live: false },
      { id: 3, name: "OBS Studio", processName: "obs64.exe", category: "creation", baselineSecs: 0, trackedSecs: 5 * h, totalSecs: 5 * h, weekSecs: 58 * 60, weeklyLimitSecs: null, lastPlayedAt: now - 2 * 86400, sessionCount: 6, live: false },
      { id: 4, name: "Minecraft", processName: "javaw.exe", category: "game", baselineSecs: 28 * h, trackedSecs: 0, totalSecs: 28 * h, weekSecs: 0, weeklyLimitSecs: null, lastPlayedAt: null, sessionCount: 0, live: false },
    ],
  };
}
