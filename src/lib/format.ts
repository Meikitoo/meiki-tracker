// Formats d'affichage des durées : 453h · 4h12 · 12 min · 01:47:12

/** Durée compacte : ≥ 10 h → « 453h », ≥ 1 h → « 4h12 », sinon « 12 min ». */
export function fmtDuration(secs: number): string {
  const s = Math.max(0, Math.floor(secs));
  const h = Math.floor(s / 3600);
  const m = Math.floor((s % 3600) / 60);
  if (h >= 10) return `${h}h`;
  if (h >= 1) return `${h}h${String(m).padStart(2, "0")}`;
  return `${m} min`;
}

/** Chrono HH:MM:SS. */
export function fmtClock(secs: number): string {
  const s = Math.max(0, Math.floor(secs));
  const pad = (n: number) => String(n).padStart(2, "0");
  return `${pad(Math.floor(s / 3600))}:${pad(Math.floor((s % 3600) / 60))}:${pad(s % 60)}`;
}

/** Variation en % entre deux valeurs, ou null si pas de référence. */
export function percentChange(current: number, previous: number): number | null {
  if (previous <= 0) return null;
  return Math.round(((current - previous) / previous) * 100);
}
