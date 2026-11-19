<script lang="ts">
  // Barres du temps joué par jour. Une seule série : pas de légende ;
  // la limite est une ligne pointillée nommée, et les barres qui la dépassent
  // passent en rouge (couleur d'alerte) — l'info ne repose pas que sur la couleur.
  import type { DayStat } from "$lib/api";
  import { fmtDay, fmtDuration } from "$lib/format";
  import { t } from "$lib/i18n";

  let { days, dailyLimit = null, todayExtra = 0 }: { days: DayStat[]; dailyLimit?: number | null; todayExtra?: number } =
    $props();

  const T = t.game.chart;
  const STEPS = [900, 1800, 3600, 7200, 10800, 14400, 21600, 43200];

  const values = $derived(days.map((d, i) => d.secs + (i === days.length - 1 ? todayExtra : 0)));
  const peak = $derived(Math.max(...values, dailyLimit ?? 0, 1));
  // Pas de graduation « rond » pour ~3-4 lignes de grille.
  const step = $derived(STEPS.find((s) => peak / s <= 4) ?? 86400);
  const top = $derived(Math.ceil((peak * 1.08) / step) * step);
  const ticks = $derived(Array.from({ length: Math.round(top / step) + 1 }, (_, i) => i * step));

  let hovered = $state<number | null>(null);

  function pct(v: number) {
    return (v / top) * 100;
  }
  function label(i: number) {
    const over = dailyLimit !== null && values[i] > dailyLimit;
    const day = i === days.length - 1 ? T.today : fmtDay(days[i].dayStart);
    return `${day} : ${fmtDuration(values[i])}${over ? ` (${T.overLimit})` : ""}`;
  }
  function tickLabel(secs: number) {
    if (secs === 0) return "0";
    return secs % 3600 === 0 ? `${secs / 3600}h` : fmtDuration(secs);
  }
  function dayNumber(ts: number) {
    return new Date(ts * 1000).getDate();
  }
  function weekdayLetter(ts: number) {
    return new Intl.DateTimeFormat(t.locale, { weekday: "narrow" }).format(new Date(ts * 1000));
  }
</script>

<div class="chart">
  <div class="plot">
    <div class="axis" aria-hidden="true">
      {#each ticks as tk (tk)}
        <!-- Graduation masquée si elle tombe sur l'étiquette de la limite. -->
        {#if dailyLimit === null || Math.abs(pct(tk) - pct(dailyLimit)) > 9}
          <span style:bottom="{pct(tk)}%">{tickLabel(tk)}</span>
        {/if}
      {/each}
      {#if dailyLimit !== null}
        <span class="limit-tick" style:bottom="{pct(dailyLimit)}%">{fmtDuration(dailyLimit)}</span>
      {/if}
    </div>

    <div class="area">
      {#each ticks as tk (tk)}
        <div class="grid" class:base={tk === 0} style:bottom="{pct(tk)}%"></div>
      {/each}

      {#if dailyLimit !== null}
        <div class="limit" style:bottom="{pct(dailyLimit)}%" title={T.limitLine}></div>
      {/if}

      <div class="bars">
        {#each days as d, i (d.dayStart)}
          {@const over = dailyLimit !== null && values[i] > dailyLimit}
          <!-- Zone de survol plus large que la barre (toute la colonne). -->
          <button
            type="button"
            class="col"
            aria-label={label(i)}
            onmouseenter={() => (hovered = i)}
            onmouseleave={() => (hovered = null)}
            onfocus={() => (hovered = i)}
            onblur={() => (hovered = null)}
          >
            {#if values[i] > 0}
              <div class="bar" class:over class:dim={hovered !== null && hovered !== i} style:height="{pct(values[i])}%"></div>
            {/if}
            {#if hovered === i}
              <div class="tip" style:bottom="calc({pct(values[i])}% + 8px)" class:left={i > days.length - 3} class:right={i < 2}>
                {label(i)}
              </div>
            {/if}
          </button>
        {/each}
      </div>
    </div>
  </div>

  <div class="xlabels" aria-hidden="true">
    {#each days as d, i (d.dayStart)}
      <span class:today={i === days.length - 1}>
        <span class="wd">{weekdayLetter(d.dayStart)}</span>
        {dayNumber(d.dayStart)}
      </span>
    {/each}
  </div>
</div>

<style>
  .chart {
    --axis-w: 44px;
  }
  .plot {
    display: flex;
    height: 200px;
  }
  .axis {
    width: var(--axis-w);
    position: relative;
    flex-shrink: 0;
  }
  .axis span {
    position: absolute;
    right: 10px;
    transform: translateY(50%);
    font-size: 11px;
    color: var(--text-3);
    font-variant-numeric: tabular-nums;
  }
  .area {
    position: relative;
    flex: 1;
  }
  .grid {
    position: absolute;
    left: 0;
    right: 0;
    height: 1px;
    background: var(--border);
    opacity: 0.6;
  }
  .grid.base {
    opacity: 1;
    background: var(--border-strong);
  }
  .limit {
    position: absolute;
    left: 0;
    right: 0;
    border-top: 2px dashed var(--alert);
    z-index: 2;
    pointer-events: none;
  }
  .axis .limit-tick {
    color: var(--alert-2);
    font-weight: 600;
  }
  .bars {
    position: absolute;
    inset: 0;
    display: flex;
    gap: 8px;
  }
  .col {
    position: relative;
    flex: 1;
    height: 100%;
    display: flex;
    align-items: flex-end;
    justify-content: center;
    outline: none;
    border: 0;
    padding: 0;
    background: transparent;
    cursor: default;
    border-radius: 4px;
  }
  .col:focus-visible {
    box-shadow: inset 0 0 0 2px var(--accent);
  }
  .bar {
    width: 100%;
    max-width: 36px;
    background: var(--accent);
    border-radius: 4px 4px 0 0;
    transition: opacity 0.12s;
  }
  .bar.over {
    background: var(--alert);
  }
  .bar.dim {
    opacity: 0.45;
  }
  .tip {
    position: absolute;
    left: 50%;
    transform: translateX(-50%);
    white-space: nowrap;
    background: var(--bg);
    border: 1px solid var(--border-strong);
    color: var(--text);
    font-size: 12px;
    font-weight: 600;
    padding: 6px 10px;
    border-radius: var(--radius-sm);
    z-index: 3;
    pointer-events: none;
  }
  .tip.left {
    left: auto;
    right: 0;
    transform: none;
  }
  .tip.right {
    left: 0;
    transform: none;
  }
  .xlabels {
    display: flex;
    gap: 8px;
    margin-left: var(--axis-w);
    margin-top: 8px;
  }
  .xlabels > span {
    flex: 1;
    text-align: center;
    font-size: 12px;
    color: var(--text-3);
    font-variant-numeric: tabular-nums;
    display: flex;
    flex-direction: column;
    line-height: 1.25;
  }
  .wd {
    text-transform: uppercase;
    font-size: 10px;
  }
  .xlabels .today {
    color: var(--accent);
    font-weight: 600;
  }
</style>
