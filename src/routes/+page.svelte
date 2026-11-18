<script lang="ts">
  import GameCard from "$lib/components/GameCard.svelte";
  import LiveBanner from "$lib/components/LiveBanner.svelte";
  import StatTile from "$lib/components/StatTile.svelte";
  import { fmtDuration, percentChange } from "$lib/format";
  import { t } from "$lib/i18n";
  import { library } from "$lib/stores/library.svelte";

  type SortKey = "total" | "recent";
  const SORT_KEY = "meiki.sort";

  function loadSort(): SortKey {
    try {
      return localStorage.getItem(SORT_KEY) === "recent" ? "recent" : "total";
    } catch {
      return "total";
    }
  }
  let sort = $state<SortKey>(loadSort());
  function setSort(s: SortKey) {
    sort = s;
    try {
      localStorage.setItem(SORT_KEY, s);
    } catch {
      /* stockage indisponible : on garde juste le choix en mémoire */
    }
  }

  const data = $derived(library.data);

  // Temps des sessions en cours, à ajouter aux totaux (le backend ne les compte pas).
  function liveSecsFor(gameId: number): number {
    const l = library.liveFor(gameId);
    return l ? library.liveSecs(l) : 0;
  }
  const liveTotal = $derived(library.live.reduce((sum, l) => sum + library.liveSecs(l), 0));
  const liveWeek = $derived(
    data
      ? library.live.reduce(
          (sum, l) => sum + Math.min(library.liveSecs(l), Math.max(0, library.now - data.summary.weekStart)),
          0,
        )
      : 0,
  );

  const weekSecs = $derived((data?.summary.weekSecs ?? 0) + liveWeek);
  const change = $derived(data ? percentChange(weekSecs, data.summary.lastWeekSecs) : null);

  const games = $derived.by(() => {
    if (!data) return [];
    const list = [...data.games];
    if (sort === "total") {
      list.sort((a, b) => b.totalSecs + liveSecsFor(b.id) - (a.totalSecs + liveSecsFor(a.id)));
    } else {
      list.sort((a, b) => (b.live ? 1 : 0) - (a.live ? 1 : 0) || (b.lastPlayedAt ?? 0) - (a.lastPlayedAt ?? 0));
    }
    return list;
  });
  const weekRef = $derived(Math.max(0, ...games.map((g) => g.weekSecs + liveSecsFor(g.id))));
</script>

<header>
  <div>
    <h1>{t.library.title}</h1>
    <p class="count">{t.library.count(data?.games.length ?? 0)}</p>
  </div>
  <button class="btn btn-primary add" type="button" title={t.library.addSoon}>
    <span class="plus" aria-hidden="true">+</span>
    {t.library.add}
  </button>
</header>

{#if library.error && !data}
  <div class="state">
    <p>{t.app.error}</p>
    <p class="detail">{library.error}</p>
    <button class="btn" type="button" onclick={() => library.refresh()}>{t.app.retry}</button>
  </div>
{:else if !data}
  <div class="state">{t.app.loading}</div>
{:else}
  {#each library.live as l (l.gameId)}
    <div class="live"><LiveBanner info={l} secs={library.liveSecs(l)} /></div>
  {/each}

  <section class="tiles">
    <StatTile
      label={t.library.tiles.week}
      value={fmtDuration(weekSecs)}
      sub={change === null ? t.library.tiles.weekNoRef : t.library.tiles.weekVs(change)}
      tone={change === null ? "neutral" : change >= 0 ? "up" : "down"}
    />
    <StatTile
      label={t.library.tiles.total}
      value={fmtDuration(data.summary.totalSecs + liveTotal)}
      sub={t.library.tiles.totalSub(fmtDuration(data.summary.trackedSecs + liveTotal))}
    />
    <StatTile
      label={t.library.tiles.streak}
      value={t.library.tiles.streakValue(data.summary.streakDays)}
      sub={t.library.tiles.streakRecord(data.summary.streakRecord)}
    />
  </section>

  {#if games.length === 0}
    <div class="empty">
      <h2>{t.library.empty.title}</h2>
      <p>{t.library.empty.text}</p>
    </div>
  {:else}
    <div class="grid-head">
      <span class="sort-label">{t.library.sort}</span>
      <div class="segmented" role="group" aria-label={t.library.sort}>
        <button type="button" class:on={sort === "total"} aria-pressed={sort === "total"} onclick={() => setSort("total")}>
          {t.library.sortTotal}
        </button>
        <button type="button" class:on={sort === "recent"} aria-pressed={sort === "recent"} onclick={() => setSort("recent")}>
          {t.library.sortRecent}
        </button>
      </div>
    </div>

    <section class="grid">
      {#each games as g (g.id)}
        <GameCard game={g} liveSecs={liveSecsFor(g.id)} {weekRef} />
      {/each}
    </section>
  {/if}
{/if}

<style>
  header {
    display: flex;
    align-items: flex-end;
    justify-content: space-between;
    gap: 16px;
    margin-bottom: 24px;
  }
  h1 {
    font-size: 34px;
    line-height: 1;
  }
  .count {
    margin: 6px 0 0;
    color: var(--text-3);
  }
  .add {
    min-height: 48px;
    padding: 0 22px;
    font-size: 15px;
  }
  .plus {
    font-size: 22px;
    line-height: 1;
    font-weight: 500;
  }
  .live {
    margin-bottom: 16px;
  }
  .tiles {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 16px;
    margin-bottom: 28px;
  }
  .grid-head {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 10px;
    margin-bottom: 12px;
  }
  .sort-label {
    color: var(--text-3);
    font-size: 13px;
  }
  .segmented {
    display: inline-flex;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 3px;
  }
  .segmented button {
    min-height: 36px;
    padding: 0 14px;
    border: 0;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--text-3);
    font-weight: 500;
    font-size: 13px;
    cursor: pointer;
  }
  .segmented button:hover {
    color: var(--text);
  }
  .segmented button.on {
    background: var(--accent-bg);
    color: var(--accent);
  }
  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(270px, 1fr));
    gap: 16px;
  }
  .empty,
  .state {
    border: 1px dashed var(--border-strong);
    border-radius: var(--radius-lg);
    padding: 40px 24px;
    text-align: center;
    color: var(--text-3);
  }
  .empty h2 {
    color: var(--text);
    font-size: 22px;
    margin-bottom: 8px;
  }
  .empty p {
    margin: 0 auto;
    max-width: 420px;
  }
  .detail {
    font-size: 12px;
  }
</style>
