<script lang="ts">
  // Paramètres → À propos : version, mises à jour, code source, soutien.
  import { onMount } from "svelte";
  import { inTauri, openWeb, setUpdateCheck } from "$lib/api";
  import Toggle from "$lib/components/Toggle.svelte";
  import { t } from "$lib/i18n";
  import { DONATE_URL, REPO_URL } from "$lib/links";
  import { updates } from "$lib/stores/update.svelte";

  let { updateCheck, onchanged }: { updateCheck: boolean; onchanged: () => void } = $props();
  const T = t.settings;

  let version = $state("");
  onMount(async () => {
    if (!inTauri) return;
    const { getVersion } = await import("@tauri-apps/api/app");
    version = await getVersion();
  });

  async function toggleAuto(v: boolean) {
    await setUpdateCheck(v);
    onchanged();
  }
</script>

<section class="panel" id="a-propos">
  <h2>{T.about}</h2>
  <div class="setting">
    <div class="text">
      <span class="label">{T.version(version || "—")}</span>
      <span class="hint">{T.license}</span>
    </div>
    <div class="btns">
      <button class="btn" type="button" onclick={() => openWeb(REPO_URL)}>{T.sourceCode}</button>
      {#if DONATE_URL}
        <button class="btn support" type="button" onclick={() => openWeb(DONATE_URL)}>♥ {T.support}</button>
      {/if}
    </div>
  </div>

  <div class="setting">
    <div class="text">
      <span class="label">{T.updates}</span>
      {#if updates.status === "none"}
        <span class="hint ok">{T.upToDate}</span>
      {:else if updates.status === "available" && updates.version}
        <span class="hint avail">{T.available(updates.version)}</span>
        {#if updates.notes}<span class="hint notes">{updates.notes}</span>{/if}
      {:else if updates.status === "error" && updates.error}
        <span class="hint err">{updates.error === T.updateUnreachable ? updates.error : T.updateError(updates.error)}</span>
      {/if}
    </div>
    {#if updates.status === "available" || updates.status === "installing"}
      <button class="btn btn-primary" type="button" disabled={updates.status === "installing"} onclick={() => updates.install()}>
        {updates.status === "installing" ? T.installing : T.install}
      </button>
    {:else}
      <button class="btn" type="button" disabled={updates.status === "checking"} onclick={() => updates.check()}>
        {updates.status === "checking" ? T.checking : T.checkNow}
      </button>
    {/if}
  </div>
  <Toggle label={T.updateAuto} hint={T.updateAutoHint} checked={updateCheck} onchange={toggleAuto} />
</section>

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
  .setting {
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
  .hint.ok,
  .hint.avail {
    color: var(--accent);
  }
  .hint.err {
    color: var(--alert-2);
    word-break: break-word;
  }
  .notes {
    white-space: pre-line;
  }
  .btns {
    display: flex;
    gap: 8px;
    flex-shrink: 0;
  }
  .support:hover {
    border-color: #ff8a96;
    color: #ff8a96;
  }
</style>
