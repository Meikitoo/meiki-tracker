<script lang="ts">
  import { goto } from "$app/navigation";
  import { page } from "$app/state";
  import {
    archiveGame,
    deleteSession,
    getGameDetail,
    renameGame,
    setBaseline,
    setWeeklyLimit,
    type GameDetail,
    type SessionRow,
  } from "$lib/api";
  import BarChart from "$lib/components/BarChart.svelte";
  import GameIcon from "$lib/components/GameIcon.svelte";
  import LiveBanner from "$lib/components/LiveBanner.svelte";
  import Modal from "$lib/components/Modal.svelte";
  import SessionItem from "$lib/components/SessionItem.svelte";
  import StatTile from "$lib/components/StatTile.svelte";
  import { fmtDate, fmtDay, fmtDuration, fmtTime } from "$lib/format";
  import { t } from "$lib/i18n";
  import { icons } from "$lib/stores/icons.svelte";
  import { library } from "$lib/stores/library.svelte";

  const T = t.game;
  const id = $derived(Number(page.url.searchParams.get("id")));

  let detail = $state<GameDetail | null>(null);
  let loaded = $state(false);
  let error = $state<string | null>(null);

  async function load() {
    try {
      detail = await getGameDetail(id);
      error = null;
    } catch (e) {
      error = String(e);
    } finally {
      loaded = true;
    }
  }

  // Recharge quand on change de jeu, et à chaque rafraîchissement de la bibliothèque
  // (session terminée, modification…), signalé par un nouvel objet `library.data`.
  $effect(() => {
    void id;
    void library.data;
    load();
  });

  // Session en cours : ajoutée en direct aux totaux.
  const live = $derived(library.liveFor(id));
  const liveSecs = $derived(live ? library.liveSecs(live) : 0);
  const game = $derived(detail?.game);
  const dailyLimit = $derived(game?.weeklyLimitSecs ? Math.round(game.weeklyLimitSecs / 7) : null);
  const longest = $derived(Math.max(detail?.longestSecs ?? 0, liveSecs));

  // --- Renommer (dans l'en-tête)
  let renaming = $state(false);
  let newName = $state("");
  let renameError = $state<string | null>(null);
  function startRename() {
    newName = game?.name ?? "";
    renameError = null;
    renaming = true;
  }
  async function saveRename() {
    if (!newName.trim()) {
      renameError = t.add.details.nameRequired;
      return;
    }
    try {
      await renameGame(id, newName);
      renaming = false;
      await library.refresh();
    } catch (e) {
      renameError = String(e);
    }
  }

  // --- Retirer le jeu
  let confirmRemove = $state(false);
  async function remove() {
    await archiveGame(id);
    confirmRemove = false;
    icons.forget(id);
    await library.refresh();
    goto("/");
  }

  // --- Supprimer une session
  let toDelete = $state<SessionRow | null>(null);
  let confirmDelete = $state(false);
  function askDelete(s: SessionRow) {
    toDelete = s;
    confirmDelete = true;
  }
  async function doDelete() {
    if (!toDelete) return;
    await deleteSession(toDelete.id);
    confirmDelete = false;
    toDelete = null;
    await library.refresh();
  }

  // --- Saisie d'un temps (temps de départ ou limite hebdo)
  let timeMode = $state<"baseline" | "limit">("baseline");
  let timeOpen = $state(false);
  let hours = $state<number | null>(null);
  let minutes = $state<number | null>(null);
  let timeError = $state<string | null>(null);
  function openTime(mode: "baseline" | "limit") {
    timeMode = mode;
    const secs = (mode === "baseline" ? game?.baselineSecs : game?.weeklyLimitSecs) ?? 0;
    hours = secs ? Math.floor(secs / 3600) : null;
    minutes = secs ? Math.floor((secs % 3600) / 60) : null;
    timeError = null;
    timeOpen = true;
  }
  async function saveTime(clear = false) {
    const h = hours ?? 0;
    const m = minutes ?? 0;
    if (!Number.isInteger(h) || !Number.isInteger(m) || h < 0 || m < 0 || m > 59) {
      timeError = T.dialogs.invalid;
      return;
    }
    const secs = h * 3600 + m * 60;
    if (timeMode === "limit" && secs > 168 * 3600) {
      timeError = T.dialogs.tooMuchWeek;
      return;
    }
    try {
      if (timeMode === "baseline") await setBaseline(id, secs);
      else await setWeeklyLimit(id, clear || secs === 0 ? null : secs);
      timeOpen = false;
      await library.refresh();
    } catch (e) {
      timeError = String(e);
    }
  }
</script>

<a class="back" href="/">← {T.back}</a>

{#if error && !detail}
  <p class="muted">{error}</p>
{:else if loaded && !detail}
  <p class="muted">{T.notFound}</p>
{:else if detail && game}
  <header>
    <GameIcon src={icons.get(id)} name={game.name} size={64} />
    <div class="ident">
      {#if renaming}
        <form
          class="rename"
          onsubmit={(e) => {
            e.preventDefault();
            saveRename();
          }}
        >
          <!-- svelte-ignore a11y_autofocus -->
          <input class="input" type="text" maxlength="80" bind:value={newName} aria-label={T.rename} autofocus />
          <button class="btn btn-primary" type="submit">{T.save}</button>
          <button class="btn" type="button" onclick={() => (renaming = false)}>{T.cancel}</button>
        </form>
        {#if renameError}<p class="error">{renameError}</p>{/if}
      {:else}
        <h1>{game.name}</h1>
      {/if}
      <p class="exe">{game.processName} · {T.categories[game.category] ?? game.category}</p>
    </div>
    {#if !renaming}
      <div class="head-actions">
        <button class="btn" type="button" onclick={startRename}>{T.rename}</button>
        <button class="btn danger-outline" type="button" onclick={() => (confirmRemove = true)}>{T.remove}</button>
      </div>
    {/if}
  </header>

  {#if live}
    <div class="live"><LiveBanner info={live} secs={liveSecs} /></div>
  {/if}

  <section class="tiles">
    <div class="tile-accent">
      <StatTile
        label={T.tiles.total}
        value={fmtDuration(detail.totalSecs + liveSecs)}
        sub={T.tiles.totalSub(fmtDuration(detail.trackedSecs + liveSecs), fmtDuration(game.baselineSecs))}
      />
    </div>
    <div class="tile-edit">
      <StatTile
        label={T.tiles.baseline}
        value={fmtDuration(game.baselineSecs)}
        sub={game.baselineSetAt ? T.tiles.baselineSet(fmtDate(game.baselineSetAt)) : T.tiles.baselineNone}
      />
      <button class="icon-btn pencil" type="button" aria-label={T.tiles.baselineEdit} title={T.tiles.baselineEdit} onclick={() => openTime("baseline")}>
        <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M4 20h4L19 9l-4-4L4 16v4zM14 6l4 4" /></svg>
      </button>
    </div>
    <StatTile
      label={T.tiles.tracked}
      value={fmtDuration(detail.trackedSecs + liveSecs)}
      sub={T.tiles.trackedSub(detail.sessionCount + (live ? 1 : 0))}
    />
    <StatTile label={T.tiles.longest} value={longest > 0 ? fmtDuration(longest) : "—"} sub={longest > 0 ? "" : T.tiles.longestNone} />
  </section>

  <section class="panel">
    <div class="panel-head">
      <h2>{T.chart.title}</h2>
      <div class="limit-info">
        <span>
          {game.weeklyLimitSecs && dailyLimit
            ? T.chart.limit(fmtDuration(game.weeklyLimitSecs), fmtDuration(dailyLimit))
            : T.chart.noLimit}
        </span>
        <button class="btn small-btn" type="button" onclick={() => openTime("limit")}>
          {game.weeklyLimitSecs ? T.chart.editLimit : T.chart.setLimit}
        </button>
      </div>
    </div>
    <BarChart days={detail.days} {dailyLimit} todayExtra={liveSecs} />
  </section>

  <section class="panel">
    <div class="panel-head">
      <h2>{T.sessions.title}</h2>
    </div>
    {#if detail.pending}
      <div class="pending">
        <strong>{fmtDay(detail.pending.startedAt)} · {fmtTime(detail.pending.startedAt)} → {fmtTime(detail.pending.endedAt)}</strong>
        <span class="num">{fmtDuration(detail.pending.netSecs)}</span>
        <p>{T.sessions.pending}</p>
      </div>
    {/if}
    {#if detail.sessions.length === 0 && !detail.pending}
      <p class="muted empty">{T.sessions.empty}</p>
    {:else}
      <ul class="sessions">
        {#each detail.sessions as s (s.id)}
          <SessionItem session={s} ondelete={askDelete} onchanged={() => library.refresh()} />
        {/each}
      </ul>
    {/if}
  </section>

  <Modal bind:open={confirmRemove} title={T.dialogs.removeTitle(game.name)}>
    <p>{T.dialogs.removeText}</p>
    {#snippet actions()}
      <button class="btn" type="button" onclick={() => (confirmRemove = false)}>{T.cancel}</button>
      <button class="btn btn-danger" type="button" onclick={remove}>{T.dialogs.removeConfirm}</button>
    {/snippet}
  </Modal>

  <Modal bind:open={confirmDelete} title={T.dialogs.deleteTitle}>
    {#if toDelete}<p>{T.dialogs.deleteText(fmtDuration(toDelete.durationSecs), fmtDay(toDelete.startedAt))}</p>{/if}
    {#snippet actions()}
      <button class="btn" type="button" onclick={() => (confirmDelete = false)}>{T.cancel}</button>
      <button class="btn btn-danger" type="button" onclick={doDelete}>{T.dialogs.deleteConfirm}</button>
    {/snippet}
  </Modal>

  <Modal bind:open={timeOpen} title={timeMode === "baseline" ? T.dialogs.baselineTitle : T.dialogs.limitTitle}>
    <p>{timeMode === "baseline" ? T.dialogs.baselineText : T.dialogs.limitText}</p>
    <form
      id="time-form"
      class="time"
      novalidate
      onsubmit={(e) => {
        e.preventDefault();
        saveTime();
      }}
    >
      <label><input class="input" type="number" min="0" step="1" inputmode="numeric" placeholder="0" bind:value={hours} /> {T.dialogs.hours}</label>
      <label><input class="input" type="number" min="0" max="59" step="1" inputmode="numeric" placeholder="0" bind:value={minutes} /> {T.dialogs.minutes}</label>
    </form>
    {#if timeError}<p class="error" role="alert">{timeError}</p>{/if}
    {#snippet actions()}
      {#if timeMode === "limit" && game.weeklyLimitSecs}
        <button class="btn" type="button" onclick={() => saveTime(true)}>{T.dialogs.limitRemove}</button>
      {/if}
      <button class="btn" type="button" onclick={() => (timeOpen = false)}>{T.cancel}</button>
      <button class="btn btn-primary" type="submit" form="time-form">{T.save}</button>
    {/snippet}
  </Modal>
{/if}

<style>
  .back {
    display: inline-flex;
    align-items: center;
    min-height: 44px;
    color: var(--text-3);
  }
  .back:hover {
    color: var(--text);
  }
  header {
    display: flex;
    align-items: center;
    gap: 18px;
    margin: 8px 0 22px;
  }
  .ident {
    flex: 1;
    min-width: 0;
  }
  h1 {
    font-size: 34px;
    line-height: 1.05;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .exe {
    margin: 4px 0 0;
    color: var(--text-3);
  }
  .rename {
    display: flex;
    gap: 8px;
  }
  .rename .input {
    flex: 1;
    font-size: 18px;
  }
  .head-actions {
    display: flex;
    gap: 10px;
  }
  .danger-outline:hover {
    border-color: var(--alert);
    color: var(--alert-2);
  }
  .live {
    margin-bottom: 16px;
  }
  .tiles {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: 16px;
    margin-bottom: 20px;
  }
  .tile-accent :global(.tile) {
    background: var(--accent-bg);
    border-color: var(--accent);
    height: 100%;
  }
  .tile-accent :global(.value) {
    color: var(--accent);
  }
  .tile-edit {
    position: relative;
  }
  .tile-edit :global(.tile) {
    height: 100%;
  }
  .pencil {
    position: absolute;
    top: 6px;
    right: 6px;
  }
  .tiles > :global(.tile) {
    height: 100%;
  }
  .panel {
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    padding: 18px 20px 20px;
    margin-bottom: 20px;
  }
  .panel-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    margin-bottom: 16px;
  }
  h2 {
    font-size: 18px;
    letter-spacing: 0.06em;
  }
  .limit-info {
    display: flex;
    align-items: center;
    gap: 12px;
    color: var(--text-3);
    font-size: 13px;
  }
  .small-btn {
    min-height: 36px;
    padding: 0 12px;
    font-size: 13px;
  }
  .sessions {
    margin: 0 -20px -20px;
    padding: 0;
    border-top: 1px solid var(--border);
  }
  .pending {
    border: 1px dashed var(--border-strong);
    border-radius: var(--radius);
    padding: 10px 14px;
    margin-bottom: 12px;
    display: grid;
    grid-template-columns: 1fr auto;
    gap: 2px 12px;
  }
  .pending strong::first-letter {
    text-transform: uppercase;
  }
  .pending .num {
    font-size: 20px;
  }
  .pending p {
    grid-column: 1 / -1;
    margin: 0;
    color: var(--text-3);
    font-size: 12px;
  }
  .muted {
    color: var(--text-3);
  }
  .empty {
    margin: 0;
  }
  .error {
    color: var(--alert-2);
    margin: 6px 0 0;
  }
  .time {
    display: flex;
    gap: 16px;
    margin-top: 14px;
  }
  .time label {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .time .input {
    width: 96px;
  }
</style>
