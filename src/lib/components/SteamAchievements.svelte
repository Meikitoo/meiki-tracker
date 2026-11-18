<script lang="ts">
  // Succès Steam d'un jeu (fiche du jeu). Rien n'est affiché tant que l'option
  // est désactivée et qu'aucun succès n'a été récupéré.
  import {
    getAchievements,
    linkSteamGame,
    steamLinkCandidates,
    steamSyncGame,
    type Achievement,
    type Achievements,
  } from "$lib/api";
  import Modal from "$lib/components/Modal.svelte";
  import { fmtDate } from "$lib/format";
  import { t } from "$lib/i18n";
  import { library } from "$lib/stores/library.svelte";

  let { gameId }: { gameId: number } = $props();
  const T = t.steam;

  let data = $state<Achievements | null>(null);
  let busy = $state(false);
  let error = $state<string | null>(null);
  let showLocked = $state(false);

  async function load() {
    try {
      data = await getAchievements(gameId);
    } catch (e) {
      error = String(e);
    }
  }
  $effect(() => {
    void gameId;
    void library.data;
    load();
  });

  const unlocked = $derived(data?.items.filter((a) => a.achieved) ?? []);
  const locked = $derived(data?.items.filter((a) => !a.achieved) ?? []);
  const pct = $derived(data && data.total > 0 ? Math.round((data.unlocked / data.total) * 100) : 0);
  const visible = $derived(!!data && (data.enabled || data.total > 0));

  async function refresh() {
    busy = true;
    error = null;
    try {
      await steamSyncGame(gameId);
      await load();
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  // --- Lier / délier
  let linkOpen = $state(false);
  let candidates = $state<{ appid: number; name: string }[]>([]);
  let detected = $state<number | null>(null);
  let choice = $state<number | null>(null);
  let manual = $state("");
  let linkError = $state<string | null>(null);
  let loadingCandidates = $state(false);

  async function openLink() {
    linkError = null;
    manual = "";
    linkOpen = true;
    loadingCandidates = true;
    try {
      const r = await steamLinkCandidates(gameId);
      detected = r.detected;
      // Le jeu détecté en premier.
      candidates = [...r.installed].sort((a, b) => Number(b.appid === r.detected) - Number(a.appid === r.detected));
      choice = r.detected;
    } catch (e) {
      linkError = String(e);
    } finally {
      loadingCandidates = false;
    }
  }

  async function doLink() {
    let appid = choice;
    if (manual.trim()) {
      if (!/^\d{1,10}$/.test(manual.trim())) {
        linkError = T.manualInvalid;
        return;
      }
      appid = Number(manual.trim());
    }
    if (!appid) {
      linkError = T.manualInvalid;
      return;
    }
    busy = true;
    linkError = null;
    try {
      await linkSteamGame(gameId, appid);
      linkOpen = false;
      error = null;
    } catch (e) {
      // Le lien est enregistré même si la première synchro échoue : on ferme et on montre l'erreur.
      linkOpen = false;
      error = String(e);
    } finally {
      busy = false;
      await load();
    }
  }

  let confirmUnlink = $state(false);
  async function unlink() {
    confirmUnlink = false;
    try {
      await linkSteamGame(gameId, null);
      error = null;
      await load();
    } catch (e) {
      error = String(e);
    }
  }

  const fmtPct = (p: number) => (p < 1 ? p.toFixed(1) : String(Math.round(p))).replace(".", ",");
</script>

{#snippet row(a: Achievement)}
  {@const secret = a.hidden && !a.achieved}
  <li class:locked={!a.achieved}>
    {#if a.icon}
      <img src={a.icon} alt="" width="48" height="48" />
    {:else}
      <span class="no-icon" aria-hidden="true">★</span>
    {/if}
    <div class="info">
      <strong>{secret ? T.hidden : a.name}</strong>
      <span>{secret ? T.hiddenText : (a.description ?? "")}</span>
    </div>
    <div class="meta">
      {#if a.achieved && a.unlockedAt}<span>{T.unlockedOn(fmtDate(a.unlockedAt))}</span>{/if}
      {#if a.globalPercent !== null}
        <span class="rarity" class:rare={a.globalPercent < 10}>
          {#if a.globalPercent < 10}<b>{T.rare}</b> ·{/if}
          {T.rarity(fmtPct(a.globalPercent))}
        </span>
      {/if}
    </div>
  </li>
{/snippet}

{#if visible && data}
  <section class="panel">
    <div class="head">
      <h2>{T.title}</h2>
      {#if data.appid}
        <div class="head-actions">
          <span class="sub">{data.syncedAt ? T.syncedAt(fmtDate(data.syncedAt)) : T.neverFetched}</span>
          {#if data.enabled}
            <button class="btn small" type="button" disabled={busy} onclick={refresh}>{busy ? T.syncing : T.refresh}</button>
          {/if}
          <button class="btn small" type="button" onclick={() => (confirmUnlink = true)}>{T.unlink}</button>
        </div>
      {/if}
    </div>

    {#if error}<p class="msg error" role="alert">{error}</p>{/if}

    {#if !data.appid}
      <div class="empty">
        <p>{T.notLinked}</p>
        <button class="btn btn-primary" type="button" onclick={openLink}>{T.link}</button>
      </div>
    {:else}
      {#if !data.enabled}<p class="sub">{T.disabledHint}</p>{/if}
      {#if data.total > 0}
        <div class="progress">
          <span class="count num">{T.progress(data.unlocked, data.total)}</span>
          <div class="bar" role="progressbar" aria-valuenow={pct} aria-valuemin="0" aria-valuemax="100">
            <div class="fill" style:width="{pct}%"></div>
          </div>
          <span class="pct num">{pct} %</span>
        </div>

        {#if unlocked.length}
          <ul>
            {#each unlocked as a (a.apiName)}{@render row(a)}{/each}
          </ul>
        {/if}
        {#if locked.length === 0}
          <p class="done">{T.allDone}</p>
        {:else}
          <button class="btn toggle-locked" type="button" onclick={() => (showLocked = !showLocked)}>
            {showLocked ? T.hideLocked : T.showLocked(locked.length)}
          </button>
          {#if showLocked}
            <ul>
              {#each locked as a (a.apiName)}{@render row(a)}{/each}
            </ul>
          {/if}
        {/if}
      {/if}
    {/if}
  </section>
{/if}

<Modal bind:open={linkOpen} title={T.linkTitle}>
  <p>{T.linkText}</p>
  {#if loadingCandidates}
    <p class="sub">{t.app.loading}</p>
  {:else if candidates.length === 0}
    <p class="sub">{T.noInstalled}</p>
  {:else}
    <div class="choices" role="radiogroup" aria-label={T.linkTitle}>
      {#each candidates as c (c.appid)}
        <label class="choice" class:selected={choice === c.appid && !manual.trim()}>
          <input type="radio" name="steam-app" value={c.appid} bind:group={choice} onchange={() => (manual = "")} />
          <span class="c-name">{c.name}</span>
          {#if c.appid === detected}<span class="badge">{T.detected}</span>{/if}
        </label>
      {/each}
    </div>
  {/if}
  <form
    id="steam-link-form"
    class="manual"
    novalidate
    onsubmit={(e) => {
      e.preventDefault();
      doLink();
    }}
  >
    <label for="steam-appid">{T.manualLabel}</label>
    <input id="steam-appid" class="input" type="text" inputmode="numeric" maxlength="10" placeholder="1938090" bind:value={manual} />
    <span class="sub">{T.manualHint}</span>
  </form>
  {#if linkError}<p class="msg error" role="alert">{linkError}</p>{/if}
  {#snippet actions()}
    <button class="btn" type="button" onclick={() => (linkOpen = false)}>{t.game.cancel}</button>
    <button class="btn btn-primary" type="submit" form="steam-link-form" disabled={busy}>{busy ? T.linking : T.linkConfirm}</button>
  {/snippet}
</Modal>

<Modal bind:open={confirmUnlink} title={T.unlinkTitle}>
  <p>{T.unlinkText}</p>
  {#snippet actions()}
    <button class="btn" type="button" onclick={() => (confirmUnlink = false)}>{t.game.cancel}</button>
    <button class="btn btn-danger" type="button" onclick={unlink}>{T.unlink}</button>
  {/snippet}
</Modal>

<style>
  .panel {
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    padding: 18px 20px 20px;
    margin-bottom: 20px;
  }
  .head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    margin-bottom: 14px;
  }
  h2 {
    font-size: 18px;
    letter-spacing: 0.06em;
  }
  .head-actions {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .small {
    min-height: 36px;
    padding: 0 12px;
    font-size: 13px;
  }
  .sub {
    color: var(--text-3);
    font-size: 13px;
  }
  .empty {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    color: var(--text-2);
  }
  .empty p {
    margin: 0;
  }
  .progress {
    display: grid;
    grid-template-columns: auto 1fr auto;
    align-items: center;
    gap: 14px;
    margin-bottom: 14px;
  }
  .count {
    font-family: "Rajdhani", sans-serif;
    font-weight: 700;
    font-size: 26px;
    color: var(--accent);
  }
  .pct {
    color: var(--text-2);
    font-weight: 600;
  }
  .bar {
    height: 10px;
    border-radius: 5px;
    background: var(--border);
    overflow: hidden;
  }
  .fill {
    height: 100%;
    background: var(--accent);
    border-radius: 5px;
  }
  ul {
    list-style: none;
    margin: 0 -20px;
    padding: 0;
    border-top: 1px solid var(--border);
  }
  li {
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 10px 20px;
    border-bottom: 1px solid var(--border);
  }
  li img,
  .no-icon {
    width: 48px;
    height: 48px;
    border-radius: 6px;
    flex-shrink: 0;
  }
  .no-icon {
    display: grid;
    place-items: center;
    background: var(--border);
    color: var(--text-3);
    font-size: 20px;
  }
  li.locked img {
    filter: grayscale(1);
    opacity: 0.45;
  }
  li.locked strong {
    color: var(--text-2);
  }
  .info {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }
  .info span {
    color: var(--text-3);
    font-size: 13px;
  }
  .meta {
    display: flex;
    flex-direction: column;
    align-items: flex-end;
    gap: 2px;
    color: var(--text-3);
    font-size: 12px;
    text-align: right;
    flex-shrink: 0;
  }
  .rarity.rare {
    color: #f5b94a;
  }
  .toggle-locked {
    margin-top: 14px;
    width: 100%;
  }
  .done {
    margin: 14px 0 0;
    color: var(--accent);
    font-weight: 600;
  }
  .msg.error {
    margin: 0 0 12px;
    padding: 10px 14px;
    border-radius: var(--radius);
    border: 1px solid var(--alert);
    color: var(--alert-2);
    background: #2a0f14;
    word-break: break-word;
  }
  .choices {
    display: flex;
    flex-direction: column;
    gap: 6px;
    max-height: 240px;
    overflow-y: auto;
    margin: 12px 0;
  }
  .choice {
    display: flex;
    align-items: center;
    gap: 10px;
    min-height: 44px;
    padding: 0 12px;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    cursor: pointer;
  }
  .choice.selected {
    border-color: var(--accent);
    background: var(--accent-bg);
  }
  .c-name {
    flex: 1;
  }
  .badge {
    font-size: 11px;
    color: var(--accent);
    border: 1px solid var(--accent);
    border-radius: 4px;
    padding: 2px 6px;
  }
  .manual {
    display: flex;
    flex-direction: column;
    gap: 6px;
    margin-top: 12px;
  }
  .manual label {
    font-weight: 600;
  }
  .manual .input {
    width: 180px;
  }
</style>
