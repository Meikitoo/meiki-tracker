// Dessin de la carte « Mon récap » sur un <canvas>.
// L'aperçu et l'image enregistrée sont le même dessin : ce qu'on voit est ce qu'on partage.
import type { Recap, RecapPeriod } from "$lib/api";
import type { Dictionnaire } from "$lib/i18n/fr";

export type CardFormat = "square" | "story";
export const CARD_SIZES: Record<CardFormat, [number, number]> = {
  square: [1080, 1080],
  story: [1080, 1920],
};

const C = {
  bg: "#0B0D10",
  surface: "#14181D",
  border: "#232A31",
  borderStrong: "#2E3740",
  text: "#FFFFFF",
  text2: "#C9D1D9",
  text3: "#9AA4AE",
  accent: "#22D3EE",
  accentBg: "#0E2A33",
};
const RAJ = (px: number) => `700 ${px}px Rajdhani, sans-serif`;
const INTER = (weight: number, px: number) => `${weight} ${px}px Inter, "Segoe UI", sans-serif`;

export interface CardInput {
  recap: Recap;
  period: RecapPeriod;
  format: CardFormat;
  /** Icônes des jeux du top (data URL), par id. */
  icons: Record<number, string | null>;
  T: Dictionnaire["recap"];
  locale: string;
  fmtDuration: (secs: number) => string;
}

/** Polices embarquées chargées avant de dessiner (sinon le canvas prend une police de secours). */
export async function loadCardFonts() {
  await Promise.all([
    document.fonts.load(RAJ(40)),
    document.fonts.load(INTER(400, 20)),
    document.fonts.load(INTER(500, 20)),
    document.fonts.load(INTER(600, 20)),
  ]).catch(() => {});
}

function loadImage(src: string): Promise<HTMLImageElement | null> {
  return new Promise((resolve) => {
    const img = new Image();
    img.onload = () => resolve(img);
    img.onerror = () => resolve(null);
    img.src = src;
  });
}

/** Coupe le texte avec « … » s'il dépasse `max` pixels. */
function fit(ctx: CanvasRenderingContext2D, text: string, max: number): string {
  if (ctx.measureText(text).width <= max) return text;
  let s = text;
  while (s.length > 1 && ctx.measureText(s + "…").width > max) s = s.slice(0, -1);
  return s.trimEnd() + "…";
}

function rangeLabel(r: Recap, period: RecapPeriod, locale: string): string {
  const d = (ts: number, o: Intl.DateTimeFormatOptions) => new Intl.DateTimeFormat(locale, o).format(new Date(ts * 1000));
  if (period === "year") return d(r.start, { year: "numeric" });
  if (period === "month") return d(r.start, { month: "long", year: "numeric" });
  return `${d(r.start, { day: "numeric", month: "short" })} – ${d(r.end, { day: "numeric", month: "short", year: "numeric" })}`;
}

export async function drawCard(canvas: HTMLCanvasElement, input: CardInput) {
  const { recap: r, period, format, T, locale, fmtDuration } = input;
  const [W, H] = CARD_SIZES[format];
  const story = format === "story";
  canvas.width = W;
  canvas.height = H;
  const ctx = canvas.getContext("2d")!;
  const P = 80;
  const inner = W - 2 * P;

  const images: Record<number, HTMLImageElement | null> = {};
  await Promise.all(
    r.topGames.map(async (g) => {
      const src = input.icons[g.id];
      images[g.id] = src ? await loadImage(src) : null;
    }),
  );

  // --- Fond : couleur, halo cyan, grille discrète, cadre HUD
  ctx.fillStyle = C.bg;
  ctx.fillRect(0, 0, W, H);
  const glow = ctx.createRadialGradient(W * 0.9, 0, 0, W * 0.9, 0, story ? 1000 : 760);
  glow.addColorStop(0, "rgba(34, 211, 238, 0.20)");
  glow.addColorStop(1, "rgba(34, 211, 238, 0)");
  ctx.fillStyle = glow;
  ctx.fillRect(0, 0, W, H);
  ctx.strokeStyle = "rgba(255, 255, 255, 0.035)";
  ctx.lineWidth = 1;
  for (let x = 60; x < W; x += 60) {
    ctx.beginPath();
    ctx.moveTo(x + 0.5, 0);
    ctx.lineTo(x + 0.5, H);
    ctx.stroke();
  }
  for (let y = 60; y < H; y += 60) {
    ctx.beginPath();
    ctx.moveTo(0, y + 0.5);
    ctx.lineTo(W, y + 0.5);
    ctx.stroke();
  }
  ctx.strokeStyle = C.border;
  ctx.lineWidth = 2;
  ctx.beginPath();
  ctx.roundRect(24, 24, W - 48, H - 48, 28);
  ctx.stroke();
  // Coins cyan
  ctx.strokeStyle = C.accent;
  ctx.lineWidth = 4;
  const k = 46;
  for (const [cx, cy, sx, sy] of [
    [24, 24, 1, 1],
    [W - 24, 24, -1, 1],
    [24, H - 24, 1, -1],
    [W - 24, H - 24, -1, -1],
  ]) {
    ctx.beginPath();
    ctx.moveTo(cx, cy + sy * k);
    ctx.lineTo(cx, cy + sy * 14);
    ctx.quadraticCurveTo(cx, cy, cx + sx * 14, cy);
    ctx.lineTo(cx + sx * k, cy);
    ctx.stroke();
  }

  ctx.textBaseline = "alphabetic";
  ctx.letterSpacing = "0px";

  // --- En-tête : logo, nom, période
  ctx.fillStyle = C.bg;
  ctx.strokeStyle = C.accent;
  ctx.lineWidth = 3;
  ctx.beginPath();
  ctx.roundRect(P, P, 72, 72, 14);
  ctx.fill();
  ctx.stroke();
  ctx.fillStyle = C.accent;
  ctx.font = RAJ(52);
  ctx.textAlign = "center";
  ctx.fillText("M", P + 36, P + 54);
  ctx.textAlign = "left";
  ctx.fillStyle = C.text;
  ctx.font = RAJ(36);
  ctx.letterSpacing = "2px";
  ctx.fillText("MEIKI TRACKER", P + 96, P + 34);
  ctx.fillStyle = C.text3;
  ctx.font = INTER(600, 17);
  ctx.letterSpacing = "3px";
  ctx.fillText("BY MEIKITOOW", P + 97, P + 64);

  ctx.textAlign = "right";
  ctx.fillStyle = C.accent;
  ctx.font = RAJ(36);
  ctx.letterSpacing = "3px";
  ctx.fillText(T.cardPeriod[period], W - P, P + 34);
  ctx.fillStyle = C.text3;
  ctx.font = INTER(500, 22);
  ctx.letterSpacing = "0px";
  ctx.fillText(rangeLabel(r, period, locale), W - P, P + 66);
  ctx.textAlign = "left";

  // --- Temps total
  let y = story ? P + 260 : P + 160;
  ctx.fillStyle = C.text3;
  ctx.font = INTER(600, 24);
  ctx.letterSpacing = "4px";
  ctx.fillText(T.played, P, y);
  ctx.letterSpacing = "0px";
  const bigSize = story ? 250 : 200;
  y += story ? 220 : 178;
  ctx.fillStyle = C.accent;
  ctx.font = RAJ(bigSize);
  ctx.shadowColor = "rgba(34, 211, 238, 0.45)";
  ctx.shadowBlur = 40;
  ctx.fillText(fmtDuration(r.totalSecs), P - 6, y);
  ctx.shadowBlur = 0;

  // --- Tuiles de stats
  y += story ? 70 : 46;
  const tiles: [string, string][] = [
    [String(r.sessionCount), T.sessions],
    [String(r.daysPlayed), T.daysPlayed],
    [String(r.bestStreak), T.streak],
    [r.longest ? fmtDuration(r.longest.secs) : "—", T.longest],
  ];
  const cols = story ? 2 : 4;
  const gap = 20;
  const tileW = (inner - gap * (cols - 1)) / cols;
  const tileH = story ? 160 : 128;
  tiles.forEach(([value, label], i) => {
    const tx = P + (i % cols) * (tileW + gap);
    const ty = y + Math.floor(i / cols) * (tileH + gap);
    ctx.fillStyle = C.surface;
    ctx.strokeStyle = C.border;
    ctx.lineWidth = 2;
    ctx.beginPath();
    ctx.roundRect(tx, ty, tileW, tileH, 18);
    ctx.fill();
    ctx.stroke();
    ctx.fillStyle = C.text;
    ctx.font = RAJ(story ? 76 : 56);
    ctx.fillText(fit(ctx, value, tileW - 48), tx + 24, ty + (story ? 88 : 68));
    ctx.fillStyle = C.text3;
    ctx.font = INTER(600, story ? 21 : 17);
    ctx.letterSpacing = "1px";
    ctx.fillText(fit(ctx, label.toUpperCase(), tileW - 48), tx + 24, ty + (story ? 128 : 102));
    ctx.letterSpacing = "0px";
  });
  y += Math.ceil(tiles.length / cols) * (tileH + gap) - gap;

  // --- Top jeux
  y += story ? 90 : 56;
  ctx.fillStyle = C.text3;
  ctx.font = INTER(600, 22);
  ctx.letterSpacing = "4px";
  ctx.fillText(T.topGames, P, y);
  ctx.letterSpacing = "0px";
  y += story ? 30 : 22;
  const games = r.topGames.slice(0, story ? 5 : 3);
  const rowH = story ? 100 : 74;
  const icon = story ? 64 : 56;
  if (games.length === 0) {
    ctx.fillStyle = C.text3;
    ctx.font = INTER(500, 26);
    ctx.fillText(T.noGames, P, y + 46);
    y += rowH;
  }
  const maxSecs = games[0]?.secs || 1;
  games.forEach((g, i) => {
    const ry = y + i * rowH;
    const cy = ry + rowH / 2;
    // Icône (ou initiale)
    const img = images[g.id];
    ctx.save();
    ctx.beginPath();
    ctx.roundRect(P, cy - icon / 2, icon, icon, 12);
    ctx.clip();
    if (img) ctx.drawImage(img, P, cy - icon / 2, icon, icon);
    else {
      ctx.fillStyle = C.borderStrong;
      ctx.fillRect(P, cy - icon / 2, icon, icon);
      ctx.fillStyle = C.text2;
      ctx.font = RAJ(icon * 0.6);
      ctx.textAlign = "center";
      ctx.fillText(g.name.charAt(0).toUpperCase(), P + icon / 2, cy + icon * 0.21);
      ctx.textAlign = "left";
    }
    ctx.restore();
    // Temps à droite
    const timeText = fmtDuration(g.secs);
    ctx.font = RAJ(story ? 48 : 40);
    const timeW = ctx.measureText(timeText).width;
    ctx.fillStyle = i === 0 ? C.accent : C.text;
    ctx.textAlign = "right";
    ctx.fillText(timeText, W - P, cy + (story ? 16 : 13));
    ctx.textAlign = "left";
    // Nom + barre
    const nx = P + icon + 22;
    const nw = W - P - timeW - 28 - nx;
    ctx.fillStyle = C.text;
    ctx.font = INTER(600, story ? 32 : 28);
    ctx.fillText(fit(ctx, `${i + 1}. ${g.name}`, nw), nx, cy - 4);
    ctx.fillStyle = C.border;
    ctx.beginPath();
    ctx.roundRect(nx, cy + 14, nw, 8, 4);
    ctx.fill();
    ctx.fillStyle = i === 0 ? C.accent : "rgba(34, 211, 238, 0.45)";
    ctx.beginPath();
    ctx.roundRect(nx, cy + 14, Math.max(8, (nw * g.secs) / maxSecs), 8, 4);
    ctx.fill();
  });
  y += games.length * rowH;

  // --- Lignes en plus (format vertical)
  if (story) {
    ctx.font = INTER(500, 26);
    ctx.fillStyle = C.text2;
    const lines: string[] = [];
    if (r.busiestDay) {
      const day = new Intl.DateTimeFormat(locale, { weekday: "long", day: "numeric", month: "long" }).format(new Date(r.busiestDay.dayStart * 1000));
      lines.push(T.busiest(day, fmtDuration(r.busiestDay.secs)));
    }
    if (r.longest) lines.push(T.longestOn(fmtDuration(r.longest.secs), r.longest.gameName));
    y += 30;
    for (const line of lines) {
      y += 48;
      ctx.fillText(fit(ctx, line, inner), P, y);
    }
  }

  // --- Pastilles : tags et succès
  const chips: string[] = r.tags.map((t) => `${t.tag} · ${fmtDuration(t.secs)}`);
  if (r.achievements > 0) chips.push(T.achievements(r.achievements));
  if (chips.length) {
    const chipH = story ? 58 : 50;
    let cx = P;
    let cy = y + (story ? 50 : 22);
    const maxRows = story ? 2 : 1;
    let rows = 1;
    ctx.font = INTER(600, story ? 24 : 21);
    for (const chip of chips) {
      const w = Math.min(ctx.measureText(chip).width + 44, inner);
      if (cx + w > W - P) {
        if (rows >= maxRows) break;
        rows++;
        cx = P;
        cy += chipH + 14;
      }
      ctx.fillStyle = C.accentBg;
      ctx.strokeStyle = "rgba(34, 211, 238, 0.55)";
      ctx.lineWidth = 2;
      ctx.beginPath();
      ctx.roundRect(cx, cy, w, chipH, chipH / 2);
      ctx.fill();
      ctx.stroke();
      ctx.fillStyle = C.text;
      ctx.fillText(fit(ctx, chip, w - 44), cx + 22, cy + chipH / 2 + 8);
      cx += w + 14;
    }
  }

  // --- Pied
  ctx.fillStyle = C.text3;
  ctx.font = INTER(500, story ? 22 : 19);
  ctx.fillText(fit(ctx, T.footer, inner - 260), P, H - P + 20);
  ctx.textAlign = "right";
  ctx.fillStyle = C.accent;
  ctx.font = RAJ(story ? 30 : 26);
  ctx.letterSpacing = "2px";
  ctx.fillText("#MEIKITRACKER", W - P, H - P + 20);
  ctx.textAlign = "left";
  ctx.letterSpacing = "0px";
}
