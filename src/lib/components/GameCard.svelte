<script lang="ts">
  import type { GameCard } from "$lib/api";
  import { fmtDuration } from "$lib/format";
  import { t } from "$lib/i18n";

  let {
    game,
    liveSecs = 0,
    weekRef,
  }: { game: GameCard; liveSecs?: number; weekRef: number } = $props();

  const total = $derived(game.totalSecs + liveSecs);
  const tracked = $derived(game.trackedSecs + liveSecs);
  const week = $derived(game.weekSecs + liveSecs);
  const limit = $derived(game.weeklyLimitSecs);
  // Sans limite hebdo : barre relative au jeu le plus joué de la semaine.
  const ratio = $derived(limit ? week / limit : weekRef > 0 ? week / weekRef : 0);
  const nearLimit = $derived(limit !== null && ratio >= 0.9);
  const initial = $derived(game.name.trim().charAt(0).toUpperCase() || "?");
</script>

<a class="card" class:live={game.live} href={`/jeu?id=${game.id}`}>
  <div class="head">
    <div class="icon title" aria-hidden="true">{initial}</div>
    <div class="ident">
      <div class="name" title={game.name}>{game.name}</div>
      <div class="exe" title={game.processName}>{game.processName}</div>
    </div>
    {#if game.live}<span class="badge">● {t.library.live}</span>{/if}
  </div>

  <div class="total num">{fmtDuration(total)}</div>
  <div class="sub">{t.library.card.sub(fmtDuration(tracked), fmtDuration(game.baselineSecs))}</div>

  <div class="week">
    <div class="week-row">
      <span>{t.library.card.week}</span>
      <span class:alert={nearLimit}>
        {limit ? t.library.card.weekLimit(fmtDuration(week), fmtDuration(limit)) : fmtDuration(week)}
      </span>
    </div>
    <div
      class="bar"
      role="progressbar"
      aria-label={t.library.card.week}
      aria-valuemin="0"
      aria-valuemax="100"
      aria-valuenow={Math.round(Math.min(1, ratio) * 100)}
    >
      <div class="fill" class:alert={nearLimit} style:width="{Math.min(1, ratio) * 100}%"></div>
    </div>
  </div>
</a>

<style>
  .card {
    display: flex;
    flex-direction: column;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    padding: 16px 18px 18px;
    min-width: 0;
    transition: border-color 0.15s;
  }
  .card:hover {
    border-color: var(--border-strong);
  }
  .card.live {
    border-color: var(--accent);
  }
  .head {
    display: flex;
    align-items: center;
    gap: 12px;
    min-width: 0;
  }
  .icon {
    width: 44px;
    height: 44px;
    flex-shrink: 0;
    border-radius: var(--radius);
    background: var(--accent-bg);
    color: var(--accent);
    display: grid;
    place-items: center;
    font-size: 22px;
  }
  .ident {
    min-width: 0;
    flex: 1;
  }
  .name {
    font-weight: 600;
    font-size: 15px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .exe {
    font-size: 12px;
    color: var(--text-3);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .badge {
    flex-shrink: 0;
    font-family: var(--font-title);
    font-weight: 700;
    text-transform: uppercase;
    font-size: 12px;
    letter-spacing: 0.08em;
    color: var(--accent);
    background: var(--accent-bg);
    border: 1px solid #164453;
    border-radius: var(--radius-sm);
    padding: 2px 8px;
  }
  .total {
    font-size: 44px;
    line-height: 1;
    margin-top: 18px;
  }
  .sub {
    margin-top: 6px;
    font-size: 13px;
    color: var(--text-3);
  }
  .week {
    margin-top: 16px;
  }
  .week-row {
    display: flex;
    justify-content: space-between;
    font-size: 12px;
    color: var(--text-3);
    margin-bottom: 6px;
  }
  .week-row span:last-child {
    color: var(--text-2);
    font-weight: 600;
  }
  .week-row .alert {
    color: var(--alert-2) !important;
  }
  .bar {
    height: 6px;
    background: var(--border);
    border-radius: 3px;
    overflow: hidden;
  }
  .fill {
    height: 100%;
    background: var(--accent);
    border-radius: 3px;
  }
  .fill.alert {
    background: var(--alert);
  }
</style>
