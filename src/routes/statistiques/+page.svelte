<script lang="ts">
  import { getOverview, type Overview } from "$lib/api";
  import BarChart from "$lib/components/BarChart.svelte";
  import { fmtDuration } from "$lib/format";
  import { t } from "$lib/i18n";
  import { library } from "$lib/stores/library.svelte";

  const T = t.stats;
  let data = $state<Overview | null>(null);
  let error = $state<string | null>(null);

  // Recharge à chaque rafraîchissement de la bibliothèque (session terminée, etc.).
  $effect(() => {
    void library.data;
    getOverview()
      .then((o) => ((data = o), (error = null)))
      .catch((e) => (error = String(e)));
  });

  // Sessions en cours ajoutées en direct.
  const liveTotal = $derived(library.live.reduce((s, l) => s + library.liveSecs(l), 0));
  function liveFor(id: number) {
    const l = library.liveFor(id);
    return l ? library.liveSecs(l) : 0;
  }
  const byWeek = $derived(
    (data?.games ?? []).map((g) => ({ ...g, week: g.weekSecs + liveFor(g.id) })).sort((a, b) => b.week - a.week),
  );
  const byTotal = $derived(
    (data?.games ?? []).map((g) => ({ ...g, total: g.totalSecs + liveFor(g.id) })).sort((a, b) => b.total - a.total),
  );
  const maxWeek = $derived(Math.max(1, ...byWeek.map((g) => g.week)));
  const maxTotal = $derived(Math.max(1, ...byTotal.map((g) => g.total)));
</script>

<h1>{T.title}</h1>

{#if error}
  <p class="muted">{error}</p>
{:else if data && data.games.length === 0}
  <p class="muted">{T.empty}</p>
{:else if data}
  <section class="panel">
    <h2>{T.days}</h2>
    <BarChart days={data.days} todayExtra={liveTotal} />
  </section>

  <div class="cols">
    {#each [{ title: T.week, rows: byWeek.map((g) => ({ id: g.id, name: g.name, v: g.week })), max: maxWeek }, { title: T.total, rows: byTotal.map((g) => ({ id: g.id, name: g.name, v: g.total })), max: maxTotal }] as block (block.title)}
      <section class="panel">
        <h2>{block.title}</h2>
        <ul class="hbars">
          {#each block.rows as r (r.id)}
            <li>
              <a href={`/jeu?id=${r.id}`} class="name" title={r.name}>{r.name}</a>
              <div class="track" aria-hidden="true"><div class="fill" style:width="{(r.v / block.max) * 100}%"></div></div>
              <span class="val num">{fmtDuration(r.v)}</span>
            </li>
          {/each}
        </ul>
      </section>
    {/each}
  </div>
{/if}

<style>
  h1 {
    font-size: 34px;
    margin-bottom: 24px;
  }
  .panel {
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    padding: 18px 20px 20px;
    margin-bottom: 20px;
  }
  h2 {
    font-size: 18px;
    letter-spacing: 0.06em;
    margin-bottom: 16px;
  }
  .cols {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 20px;
  }
  .hbars {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .hbars li {
    display: grid;
    grid-template-columns: 130px 1fr 64px;
    align-items: center;
    gap: 12px;
    min-height: 32px;
  }
  .name {
    font-weight: 600;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .name:hover {
    color: var(--accent);
  }
  .track {
    height: 10px;
    background: var(--border);
    border-radius: 5px;
    overflow: hidden;
  }
  .fill {
    height: 100%;
    background: var(--accent);
    border-radius: 5px;
  }
  .val {
    text-align: right;
    font-size: 18px;
  }
  .muted {
    color: var(--text-3);
  }
</style>
