<script lang="ts">
  // Section « Succès Steam » des Paramètres : option désactivée par défaut,
  // seule partie de l'appli qui utilise Internet.
  import { onMount } from "svelte";
  import {
    forgetSteamAccount,
    getSteamSettings,
    openWeb,
    saveSteamAccount,
    setSteamEnabled,
    steamSyncAll,
    type SteamSettings,
  } from "$lib/api";
  import Modal from "$lib/components/Modal.svelte";
  import Toggle from "$lib/components/Toggle.svelte";
  import { fmtDate, fmtTime } from "$lib/format";
  import { t } from "$lib/i18n";
  import { library } from "$lib/stores/library.svelte";

  const T = t.steam;
  const KEY_PAGE = "https://steamcommunity.com/dev/apikey";

  let view = $state<SteamSettings | null>(null);
  let apiKey = $state("");
  let steamId = $state("");
  let busy = $state<"save" | "sync" | null>(null);
  let error = $state<string | null>(null);
  let ok = $state<string | null>(null);
  let warning = $state<string | null>(null);
  let confirmForget = $state(false);

  async function load() {
    try {
      view = await getSteamSettings();
      steamId = view.steamId ?? view.detected?.steamId ?? "";
    } catch (e) {
      error = String(e);
    }
  }
  onMount(load);

  function reset() {
    error = null;
    ok = null;
    warning = null;
  }

  async function toggle(v: boolean) {
    reset();
    try {
      await setSteamEnabled(v);
      await load();
    } catch (e) {
      error = String(e);
    }
  }

  async function save() {
    reset();
    const key = apiKey.trim();
    if (!key && !view?.hasKey) {
      error = "Colle ta clé API Steam.";
      return;
    }
    busy = "save";
    try {
      const r = await saveSteamAccount(key || null, steamId.trim());
      apiKey = "";
      ok = T.connected(r.persona);
      if (!r.public) warning = T.privateProfile;
      await load();
    } catch (e) {
      error = String(e);
    } finally {
      busy = null;
    }
  }

  async function syncAll() {
    reset();
    busy = "sync";
    try {
      const r = await steamSyncAll();
      ok = T.syncDone(r.games, r.newUnlocked);
      if (r.errors.length) error = r.errors.join("\n");
      await load();
      await library.refresh();
    } catch (e) {
      error = String(e);
    } finally {
      busy = null;
    }
  }

  async function forget() {
    confirmForget = false;
    reset();
    try {
      await forgetSteamAccount();
      apiKey = "";
      await load();
    } catch (e) {
      error = String(e);
    }
  }

  const configured = $derived(!!view?.hasKey && !!view?.steamId);
</script>

{#if view}
  <section class="panel">
    <h2>{T.section}</h2>
    <Toggle label={T.enable} hint={T.enableHint} checked={view.enabled} onchange={toggle} />

    {#if view.enabled}
      <p class="notice">{T.internetNotice}</p>

      <form
        novalidate
        onsubmit={(e) => {
          e.preventDefault();
          save();
        }}
      >
        <div class="field">
          <div class="text">
            <label class="label" for="steam-key">{T.keyLabel}</label>
            <span class="hint">{view.hasKey && view.keyHint ? T.keySaved(view.keyHint) : T.keyHint}</span>
          </div>
          <div class="inline">
            <input
              id="steam-key"
              class="input"
              type="password"
              autocomplete="off"
              spellcheck="false"
              maxlength="32"
              placeholder={T.keyPlaceholder}
              bind:value={apiKey}
            />
            <button class="btn" type="button" onclick={() => openWeb(KEY_PAGE)}>{T.keyGet}</button>
          </div>
        </div>

        <div class="field">
          <div class="text">
            <label class="label" for="steam-id">{T.idLabel}</label>
            <span class="hint">{T.idHint}</span>
            {#if view.detected && view.detected.steamId !== steamId}
              <span class="hint detected">
                {T.idDetected(view.detected.persona)}
                <button class="link" type="button" onclick={() => (steamId = view!.detected!.steamId)}>{T.idUse}</button>
              </span>
            {/if}
          </div>
          <input id="steam-id" class="input id" type="text" inputmode="numeric" maxlength="17" bind:value={steamId} />
        </div>

        <div class="actions">
          {#if view.hasKey}
            <button class="btn danger-outline" type="button" onclick={() => (confirmForget = true)}>{T.forget}</button>
          {/if}
          <span class="grow"></span>
          <button class="btn btn-primary" type="submit" disabled={busy !== null}>{busy === "save" ? T.checking : T.save}</button>
        </div>
      </form>

      {#if ok}<p class="msg ok" role="status">{ok}</p>{/if}
      {#if warning}<p class="msg warn" role="alert">{warning}</p>{/if}
      {#if error}<p class="msg error" role="alert">{error}</p>{/if}

      {#if configured}
        <div class="field sync">
          <div class="text">
            <span class="label">{T.linkedCount(view.linkedGames)}</span>
            <span class="hint">
              {view.lastSyncAt ? T.lastSync(`${fmtDate(view.lastSyncAt)} à ${fmtTime(view.lastSyncAt)}`) : T.neverSynced}
            </span>
          </div>
          <button class="btn" type="button" disabled={busy !== null || view.linkedGames === 0} onclick={syncAll}>
            {busy === "sync" ? T.syncing : T.syncAll}
          </button>
        </div>
      {/if}
    {:else if error}
      <p class="msg error" role="alert">{error}</p>
    {/if}
  </section>
{/if}

<Modal bind:open={confirmForget} title={T.forgetTitle}>
  <p>{T.forgetText}</p>
  {#snippet actions()}
    <button class="btn" type="button" onclick={() => (confirmForget = false)}>{t.settings.cancel}</button>
    <button class="btn btn-danger" type="button" onclick={forget}>{T.forgetConfirm}</button>
  {/snippet}
</Modal>

<style>
  .panel {
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    padding: 16px 20px 8px;
    margin-bottom: 16px;
    max-width: 760px;
  }
  h2 {
    font-size: 16px;
    letter-spacing: 0.08em;
    color: var(--text-3);
    margin-bottom: 4px;
  }
  .notice {
    margin: 0 0 8px;
    padding: 10px 14px;
    border-left: 3px solid var(--accent);
    background: var(--accent-bg);
    border-radius: 0 var(--radius) var(--radius) 0;
    color: var(--text-2);
    font-size: 13px;
  }
  .field {
    display: flex;
    align-items: center;
    gap: 16px;
    min-height: 60px;
    padding: 10px 0;
  }
  .text {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-width: 0;
  }
  .label {
    font-weight: 600;
  }
  .hint {
    color: var(--text-3);
    font-size: 13px;
  }
  .detected {
    color: var(--accent);
  }
  .link {
    background: none;
    border: 0;
    padding: 0 4px;
    color: var(--accent);
    text-decoration: underline;
    cursor: pointer;
    font: inherit;
  }
  .inline {
    display: flex;
    gap: 8px;
    flex-shrink: 0;
  }
  .inline .input {
    width: 260px;
    font-family: ui-monospace, Consolas, monospace;
  }
  .input.id {
    width: 200px;
    flex-shrink: 0;
    font-family: ui-monospace, Consolas, monospace;
  }
  .actions {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 6px 0 12px;
  }
  .grow {
    flex: 1;
  }
  .danger-outline:hover {
    border-color: var(--alert);
    color: var(--alert-2);
  }
  .sync {
    border-top: 1px solid var(--border);
  }
  .msg {
    margin: 0 0 12px;
    padding: 10px 14px;
    border-radius: var(--radius);
    white-space: pre-line;
    word-break: break-word;
  }
  .msg.ok {
    border: 1px solid var(--accent);
    background: var(--accent-bg);
  }
  .msg.warn {
    border: 1px solid #f5b94a;
    color: #f8d18a;
    background: #2a2210;
  }
  .msg.error {
    border: 1px solid var(--alert);
    color: var(--alert-2);
    background: #2a0f14;
  }
</style>
