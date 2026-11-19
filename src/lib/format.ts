// Formats d'affichage des durées : 453h · 4h12 · 12 min · 01:47:12
import { t } from "$lib/i18n";

/** Durée compacte : ≥ 10 h → « 453h », ≥ 1 h → « 4h12 » (« 7h » si pile), sinon « 12 min ». */
export function fmtDuration(secs: number): string {
  const s = Math.max(0, Math.floor(secs));
  const h = Math.floor(s / 3600);
  const m = Math.floor((s % 3600) / 60);
  if (h >= 10 || (h >= 1 && m === 0)) return `${h}h`;
  if (h >= 1) return `${h}h${String(m).padStart(2, "0")}`;
  return `${m} min`;
}

/** Chrono HH:MM:SS. */
export function fmtClock(secs: number): string {
  const s = Math.max(0, Math.floor(secs));
  const pad = (n: number) => String(n).padStart(2, "0");
  return `${pad(Math.floor(s / 3600))}:${pad(Math.floor((s % 3600) / 60))}:${pad(s % 60)}`;
}

const dayFmt = new Intl.DateTimeFormat(t.locale, { weekday: "short", day: "numeric", month: "short" });
const dateFmt = new Intl.DateTimeFormat(t.locale, { day: "numeric", month: "long", year: "numeric" });
const timeFmt = new Intl.DateTimeFormat(t.locale, { hour: "2-digit", minute: "2-digit" });
const weekdayFmt = new Intl.DateTimeFormat(t.locale, { weekday: "short" });

/** « lun. 17 nov. » */
export const fmtDay = (ts: number) => dayFmt.format(new Date(ts * 1000));
/** « 17 novembre 2026 » */
export const fmtDate = (ts: number) => dateFmt.format(new Date(ts * 1000));
/** « 21:04 » */
export const fmtTime = (ts: number) => timeFmt.format(new Date(ts * 1000));
/** « lun. » */
export const fmtWeekday = (ts: number) => weekdayFmt.format(new Date(ts * 1000));

/** Variation en % entre deux valeurs, ou null si pas de référence. */
export function percentChange(current: number, previous: number): number | null {
  if (previous <= 0) return null;
  return Math.round(((current - previous) / previous) * 100);
}
