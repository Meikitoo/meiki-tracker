<script lang="ts">
  import { onMount } from "svelte";
  import {
    exportBackup,
    getSettings,
    importBackup,
    pickBackupFile,
    pickFolder,
    setAutostart,
    setBackupDir,
    setLanguage,
    setNotifications,
    updateTrackingSettings,
    type SettingsView,
  } from "$lib/api";
  import Modal from "$lib/components/Modal.svelte";
  import SteamSettings from "$lib/components/SteamSettings.svelte";
  import AboutSettings from "$lib/components/AboutSettings.svelte";
  import Toggle from "$lib/components/Toggle.svelte";
  import { fmtDate, fmtTime } from "$lib/format";
  import { LANGS, lang, rememberLang, t, type Lang } from "$lib/i18n";
  import { library } from "$lib/stores/library.svelte";

  const T = t.settings;

  let view = $state<SettingsView | null>(null);
  let error = $state<string | null>(null);
  let message = $state<string | null>(null);

  // Réglages de suivi (en minutes dans l'interface)
  let minSession = $state(2);
  let afkOn = $state(true);
  let afkMinutes = $state(10);
  let merge = $state(5);
  let savingTracking = $state(false);
  let savedTracking = $state(false);

  async function load() {
    try {
      view = await getSettings();
      const s = view.settings;
      minSession = Math.round(s.minSessionSecs / 60);
      afkOn = s.afkThresholdSecs > 0;
      afkMinutes = s.afkThresholdSecs > 0 ? Math.round(s.afkThresholdSecs / 60) : 10;
      merge = Math.round(s.mergeWindowSecs / 60);
    } catch (e) {
      error = String(e);
    }
  }
  onMount(load);

  function report(e: unknown) {
    message = null;
    error = String(e);
  }

  async function saveTracking() {
    const ints = [minSession, afkMinutes, merge];
    if (ints.some((v) => !Number.isInteger(v) || v < 0) || minSession > 60 || merge > 60 || (afkOn && (afkMinutes < 1 || afkMinutes > 120))) {
      error = T.invalidValues;
      return;
    }
    savingTracking = true;
    try {
      await updateTrackingSettings({
        minSessionSecs: minSession * 60,
        afkThresholdSecs: afkOn ? afkMinutes * 60 : 0,
        mergeWindowSecs: merge * 60,
      });
      error = null;
      savedTracking = true;
      setTimeout(() => (savedTracking = false), 2500);
    } catch (e) {
      report(e);
    } finally {
      savingTracking = false;
    }
  }

  async function toggleAutostart(v: boolean) {
    try {
      await setAutostart(v);
      await load();
    } catch (e) {
      report(e);
    }
  }

  async function chooseLanguage(l: Lang) {
    if (l === lang) return;
    try {
      await setLanguage(l);
      rememberLang(l);
      location.reload();
    } catch (e) {
      report(e);
    }
  }

  async function toggleNotifications(v: boolean) {
    try {
      await setNotifications(v);
      await load();
    } catch (e) {
      report(e);
    }
  }

  async function doExport() {
    try {
      const path = await exportBackup(T.exportTitle, view?.suggestedBackupName ?? "meiki-tracker.meikibackup");
      if (path) {
        error = null;
        message = T.exported(path);
      }
    } catch (e) {
      report(e);
    }
  }

  let importPath = $state<string | null>(null);
  let confirmImport = $state(false);
  async function chooseImport() {
    try {
      importPath = await pickBackupFile(T.importTitle);
      if (importPath) confirmImport = true;
    } catch (e) {
      report(e);
    }
  }
  async function doImport() {
    if (!importPath) return;
    confirmImport = false;
    try {
      const r = await importBackup(importPath);
      error = null;
      message = T.imported(r.games, r.sessions, r.safetyBackup);
      await library.refresh();
      await load();
    } catch (e) {
      report(e);
    }
  }

  async function chooseFolder() {
    try {
      const dir = await pickFolder(T.autoTitle);
      if (!dir) return;
      await setBackupDir(dir);
      error = null;
      await load();
    } catch (e) {
      report(e);
    }
  }
  async function disableAuto() {
    try {
      await setBackupDir(null);
      await load();
    } catch (e) {
      report(e);
    }
  }
</script>

<h1>{T.title}</h1>

{#if error}<p class="banner error" role="alert">{error}</p>{/if}
{#if message}<p class="banner ok" role="status">{message}</p>{/if}

{#if view}
  <section class="panel">
    <h2>{T.language}</h2>
    <div class="setting">
      <div class="text">
        <span class="label">{T.languageLabel}</span>
        <span class="hint">{T.languageHint}</span>
      </div>
      <div class="segmented" role="radiogroup" aria-label={T.languageLabel}>
        {#each LANGS as l (l.id)}
          <button
            type="button"
            role="radio"
            aria-checked={l.id === lang}
            class:active={l.id === lang}
            onclick={() => chooseLanguage(l.id)}>{l.label}</button
          >
        {/each}
      </div>
    </div>
  </section>

  <section class="panel">
    <h2>{T.tracking}</h2>
    <form
      novalidate
      onsubmit={(e) => {
        e.preventDefault();
        saveTracking();
      }}
    >
      <div class="setting">
        <div class="text">
          <span class="label">{T.minSession}</span>
          <span class="hint">{T.minSessionHint}</span>
        </div>
        <label class="num-field"><input class="input" type="number" min="0" max="60" bind:value={minSession} aria-label={T.minSession} /> {T.minutes}</label>
      </div>

      <Toggle label={T.afk} hint={T.afkHint} checked={afkOn} onchange={(v) => (afkOn = v)} />
      {#if afkOn}
        <div class="setting sub">
          <div class="text"><span class="label">{T.afkDelay}</span></div>
          <label class="num-field"><input class="input" type="number" min="1" max="120" bind:value={afkMinutes} aria-label={T.afkDelay} /> {T.minutes}</label>
        </div>
      {/if}

      <div class="setting">
        <div class="text">
          <span class="label">{T.merge}</span>
          <span class="hint">{T.mergeHint}</span>
        </div>
        <label class="num-field"><input class="input" type="number" min="0" max="60" bind:value={merge} aria-label={T.merge} /> {T.minutes}</label>
      </div>

      <div class="actions">
        {#if savedTracking}<span class="saved">{T.saved}</span>{/if}
        <button class="btn btn-primary" type="submit" disabled={savingTracking}>{T.save}</button>
      </div>
    </form>
  </section>

  <section class="panel">
    <h2>{T.startup}</h2>
    <Toggle label={T.autostart} hint={T.autostartHint} checked={view.autostart} onchange={toggleAutostart} />
  </section>

  <section class="panel">
    <h2>{T.notifications}</h2>
    <Toggle label={T.notifyEnd} hint={T.notifyEndHint} checked={view.settings.notifications} onchange={toggleNotifications} />
  </section>

  <section class="panel">
    <h2>{T.backups}</h2>
    <div class="setting">
      <div class="text">
        <span class="label">{T.exportLabel}</span>
        <span class="hint">{T.exportHint}</span>
      </div>
      <button class="btn" type="button" onclick={doExport}>{T.exportButton}</button>
    </div>
    <div class="setting">
      <div class="text">
        <span class="label">{T.importLabel}</span>
        <span class="hint">{T.importHint}</span>
      </div>
      <button class="btn" type="button" onclick={chooseImport}>{T.importButton}</button>
    </div>
    <div class="setting">
      <div class="text">
        <span class="label">{T.autoLabel}</span>
        {#if view.settings.backupDir}
          <span class="hint path">{T.autoOn(view.settings.backupDir)}</span>
          {#if view.lastAutoBackupAt}
            <span class="hint">{T.autoLast(t.dateAt(fmtDate(view.lastAutoBackupAt), fmtTime(view.lastAutoBackupAt)))} · {T.autoKeep}</span>
          {/if}
        {:else}
          <span class="hint">{T.autoOff}</span>
        {/if}
      </div>
      <div class="btns">
        {#if view.settings.backupDir}<button class="btn" type="button" onclick={disableAuto}>{T.autoDisable}</button>{/if}
        <button class="btn" type="button" onclick={chooseFolder}>{view.settings.backupDir ? T.autoChange : T.autoChoose}</button>
      </div>
    </div>
  </section>

  <SteamSettings />

  <section class="panel">
    <h2>{T.privacy}</h2>
    <p class="privacy">{T.privacyText}</p>
  </section>

  <AboutSettings updateCheck={view.updateCheck} onchanged={load} />
{/if}

<Modal bind:open={confirmImport} title={T.importConfirmTitle}>
  <p>{T.importConfirmText}</p>
  {#if importPath}<p class="path small">{importPath}</p>{/if}
  {#snippet actions()}
    <button class="btn" type="button" onclick={() => (confirmImport = false)}>{T.cancel}</button>
    <button class="btn btn-danger" type="button" onclick={doImport}>{T.importConfirm}</button>
  {/snippet}
</Modal>

<style>
  h1 {
    font-size: 34px;
    margin-bottom: 24px;
  }
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
  .setting {
    display: flex;
    align-items: center;
    gap: 16px;
    min-height: 60px;
    padding: 10px 0;
  }
  .setting.sub {
    padding-left: 20px;
    border-left: 2px solid var(--border-strong);
    margin-left: 4px;
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
  .path {
    word-break: break-all;
  }
  .num-field {
    display: flex;
    align-items: center;
    gap: 8px;
    color: var(--text-2);
  }
  .num-field .input {
    width: 84px;
  }
  .btns {
    display: flex;
    gap: 8px;
    flex-shrink: 0;
  }
  .actions {
    display: flex;
    justify-content: flex-end;
    align-items: center;
    gap: 12px;
    padding: 6px 0 12px;
  }
  .saved {
    color: var(--accent);
    font-weight: 600;
  }
  .banner {
    max-width: 760px;
    padding: 12px 16px;
    border-radius: var(--radius);
    margin: 0 0 16px;
    word-break: break-word;
  }
  .banner.error {
    color: var(--alert-2);
    border: 1px solid var(--alert);
    background: #2a0f14;
  }
  .banner.ok {
    color: var(--text);
    border: 1px solid var(--accent);
    background: var(--accent-bg);
  }
  .segmented {
    display: flex;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
    overflow: hidden;
    flex-shrink: 0;
  }
  .segmented button {
    min-height: 44px;
    padding: 0 18px;
    background: transparent;
    border: 0;
    color: var(--text-2);
    font: inherit;
    font-weight: 600;
    cursor: pointer;
  }
  .segmented button + button {
    border-left: 1px solid var(--border-strong);
  }
  .segmented button.active {
    background: var(--accent-bg);
    color: var(--accent);
  }
  .privacy {
    color: var(--text-2);
    margin: 4px 0 12px;
  }
  .small {
    font-size: 12px;
    color: var(--text-3);
  }
</style>
