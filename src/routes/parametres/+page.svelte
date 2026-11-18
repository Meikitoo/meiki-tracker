<script lang="ts">
  import { onMount } from "svelte";
  import { getStartupSettings, setAutostart, setNotifications } from "$lib/api";
  import Toggle from "$lib/components/Toggle.svelte";
  import { t } from "$lib/i18n";

  const T = t.settings;
  let autostart = $state(false);
  let notifications = $state(true);
  let loaded = $state(false);
  let error = $state<string | null>(null);

  onMount(async () => {
    try {
      const s = await getStartupSettings();
      autostart = s.autostart;
      notifications = s.notifications;
    } catch (e) {
      error = String(e);
    } finally {
      loaded = true;
    }
  });

  async function change(apply: () => Promise<void>, rollback: () => void) {
    try {
      await apply();
      error = null;
    } catch (e) {
      rollback();
      error = String(e);
    }
  }
</script>

<h1>{T.title}</h1>

<section class="panel">
  <h2>{T.startup}</h2>
  <Toggle
    label={T.autostart}
    hint={T.autostartHint}
    checked={autostart}
    disabled={!loaded}
    onchange={(v) => {
      const before = autostart;
      autostart = v;
      change(() => setAutostart(v), () => (autostart = before));
    }}
  />
</section>

<section class="panel">
  <h2>{T.notifications}</h2>
  <Toggle
    label={T.notifyEnd}
    hint={T.notifyEndHint}
    checked={notifications}
    disabled={!loaded}
    onchange={(v) => {
      const before = notifications;
      notifications = v;
      change(() => setNotifications(v), () => (notifications = before));
    }}
  />
</section>

{#if error}<p class="error" role="alert">{error}</p>{/if}
<p class="more">{T.more}</p>

<style>
  h1 {
    font-size: 34px;
    margin-bottom: 24px;
  }
  .panel {
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    padding: 16px 20px 6px;
    margin-bottom: 16px;
    max-width: 720px;
  }
  h2 {
    font-size: 16px;
    letter-spacing: 0.08em;
    color: var(--text-3);
  }
  .error {
    color: var(--alert-2);
  }
  .more {
    color: var(--text-3);
    font-size: 13px;
  }
</style>
