// Données factices, utilisées UNIQUEMENT quand l'interface est ouverte dans un
// navigateur (hors appli), pour travailler le design. Jamais dans l'appli réelle.
import type { GameDetail, Library, ProgramCandidate } from "./api";

export function mockDetail(id: number): GameDetail | null {
  const lib = mockLibrary();
  const g = lib.games.find((x) => x.id === id);
  if (!g) return null;
  const now = Math.floor(Date.now() / 1000);
  const midnight = new Date();
  midnight.setHours(0, 0, 0, 0);
  const today = Math.floor(midnight.getTime() / 1000);
  const pattern = [0, 5400, 7200, 0, 3000, 9800, 12600, 4200, 0, 6600, 2400, 10800, 8040, 1800];
  const days = pattern.map((secs, i) => ({ dayStart: today - (13 - i) * 86400, secs }));
  const tags = ["Ranked", "Fun", null, "Avec potes", "Stream"];
  const sessions = pattern
    .map((secs, i) => ({ secs, i }))
    .filter((x) => x.secs > 0)
    .reverse()
    .map(({ secs, i }, n) => {
      const start = today - (13 - i) * 86400 + 20 * 3600;
      return {
        id: 100 + n,
        startedAt: start,
        endedAt: start + secs + (n === 1 ? 900 : 0),
        durationSecs: secs,
        afkSecs: n === 1 ? 900 : 0,
        gapSecs: 0,
        tag: tags[n % tags.length],
        note: n === 0 ? "Top 1 en Résurgence" : null,
      };
    });
  return {
    game: {
      id: g.id,
      name: g.name,
      processName: g.processName,
      category: g.category as "game" | "creation",
      baselineSecs: g.baselineSecs,
      baselineSetAt: now - 20 * 86400,
      weeklyLimitSecs: g.weeklyLimitSecs,
      addedAt: now - 20 * 86400,
    },
    trackedSecs: g.trackedSecs,
    totalSecs: g.totalSecs,
    sessionCount: sessions.length,
    longestSecs: 12600,
    days,
    sessions,
    pending: null,
    live: g.live,
    at: now,
  };
}

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
