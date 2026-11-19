<script lang="ts">
  import { addGame, browseExe, listRunningPrograms, type Category, type ProgramCandidate } from "$lib/api";
  import { t } from "$lib/i18n";
  import GameIcon from "./GameIcon.svelte";

  let { open = $bindable(false), onadded }: { open?: boolean; onadded?: (id: number) => void } = $props();

  const T = t.add;
  const REFRESH_MS = 5000;

  let dialog = $state<HTMLDialogElement>();
  let step = $state<"pick" | "details">("pick");

  // Étape 1
  let programs = $state<ProgramCandidate[]>([]);
  let loading = $state(false);
  let loaded = $state(false);
  let includeBackground = $state(false);
  let search = $state("");

  // Étape 2
  let selected = $state<ProgramCandidate | null>(null);
  let name = $state("");
  let category = $state<Category>("game");
  let hours = $state<number | null>(null);
  let minutes = $state<number | null>(null);
  let saving = $state(false);
  let error = $state<string | null>(null);

  const filtered = $derived.by(() => {
    const q = search.trim().toLowerCase();
    if (!q) return programs;
    return programs.filter((p) => p.displayName.toLowerCase().includes(q) || p.processName.toLowerCase().includes(q));
  });

  async function loadPrograms() {
    loading = true;
    try {
      programs = await listRunningPrograms(includeBackground);
      error = null;
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
      loaded = true;
    }
  }

  function reset() {
    step = "pick";
    search = "";
    selected = null;
    error = null;
    saving = false;
    loaded = false;
    programs = [];
  }

  // Ouverture / fermeture de la fenêtre.
  $effect(() => {
    if (!dialog) return;
    if (open && !dialog.open) {
      reset();
      dialog.showModal();
      loadPrograms();
    } else if (!open && dialog.open) {
      dialog.close();
    }
  });

  // Rafraîchissement automatique tant qu'on est sur la liste : un jeu lancé apparaît tout seul.
  $effect(() => {
    if (!open || step !== "pick") return;
    const id = setInterval(() => {
      if (!loading) loadPrograms();
    }, REFRESH_MS);
    return () => clearInterval(id);
  });

  function choose(p: ProgramCandidate) {
    if (p.alreadyTracked) return;
    selected = p;
    name = p.displayName;
    category = "game";
    hours = null;
    minutes = null;
    error = null;
    step = "details";
  }

  async function browse() {
    try {
      const p = await browseExe(T.pick.browseTitle);
      if (!p) return;
      if (p.alreadyTracked) {
        error = T.details.alreadyTracked;
        return;
      }
      choose(p);
    } catch (e) {
      error = String(e);
    }
  }

  async function submit(withBaseline: boolean) {
    if (!selected || saving) return;
    const trimmed = name.trim();
    if (!trimmed) {
      error = T.details.nameRequired;
      return;
    }
    const h = withBaseline ? (hours ?? 0) : 0;
    const m = withBaseline ? (minutes ?? 0) : 0;
    if (!Number.isInteger(h) || !Number.isInteger(m) || h < 0 || m < 0 || m > 59) {
      error = T.details.invalidTime;
      return;
    }
    saving = true;
    error = null;
    try {
      const res = await addGame({
        processName: selected.processName,
        name: trimmed,
        category,
        baselineSecs: h * 3600 + m * 60,
        exePath: selected.exePath,
      });
      open = false;
      onadded?.(res.id);
    } catch (e) {
      error = String(e);
    } finally {
      saving = false;
    }
  }
</script>

<dialog bind:this={dialog} onclose={() => (open = false)} aria-labelledby="add-title">
  <div class="head">
    <h2 id="add-title">{T.title}</h2>
    <button class="close" type="button" aria-label={T.close} onclick={() => (open = false)}>✕</button>
  </div>

  {#if step === "pick"}
    <p class="instruction">{T.pick.instruction}</p>

    <div class="toolbar">
      <input class="search" type="search" placeholder={T.pick.search} bind:value={search} aria-label={T.pick.search} />
      <button class="btn" type="button" onclick={loadPrograms} disabled={loading}>{T.pick.refresh}</button>
    </div>

    <div class="list" aria-busy={loading}>
      {#if !loaded}
        <p class="muted">{T.pick.loading}</p>
      {:else if programs.length === 0}
        <p class="muted">{T.pick.empty}</p>
      {:else if filtered.length === 0}
        <p class="muted">{T.pick.noMatch}</p>
      {:else}
        {#each filtered as p (p.processName)}
          <button class="item" type="button" disabled={p.alreadyTracked} onclick={() => choose(p)}>
            <GameIcon src={p.icon} name={p.displayName} size={40} />
            <span class="item-text">
              <span class="item-name">{p.displayName}</span>
              <span class="item-exe">{p.processName}</span>
            </span>
            {#if p.alreadyTracked}<span class="tag">{T.pick.tracked}</span>{/if}
          </button>
        {/each}
      {/if}
    </div>

    <label class="check">
      <input type="checkbox" bind:checked={includeBackground} onchange={loadPrograms} />
      {T.pick.background}
    </label>

    {#if error}<p class="error" role="alert">{error}</p>{/if}

    <div class="foot">
      <span class="muted small">{T.pick.autoRefresh}</span>
      <button class="btn" type="button" onclick={browse}>{T.pick.browse}</button>
    </div>
  {:else if selected}
    <form
      novalidate
      onsubmit={(e) => {
        e.preventDefault();
        submit(true);
      }}
    >
      <button class="back" type="button" onclick={() => ((step = "pick"), (error = null))}>← {T.details.back}</button>

      <div class="picked">
        <GameIcon src={selected.icon} name={name || selected.displayName} size={52} />
        <div>
          <div class="item-name">{name || selected.displayName}</div>
          <div class="item-exe">{selected.processName}</div>
        </div>
      </div>

      <label class="field">
        <span class="label">{T.details.name}</span>
        <input type="text" bind:value={name} maxlength="80" required />
      </label>

      <div class="field">
        <span class="label" id="cat-label">{T.details.category}</span>
        <div class="segmented" role="radiogroup" aria-labelledby="cat-label">
          {#each ["game", "creation"] as const as c (c)}
            <button type="button" role="radio" aria-checked={category === c} class:on={category === c} onclick={() => (category = c)}>
              {T.details.categories[c]}
            </button>
          {/each}
        </div>
      </div>

      <fieldset class="field">
        <legend class="label">{T.details.already}</legend>
        <p class="muted small hint">{T.details.alreadyHint}</p>
        <div class="time">
          <label><input type="number" min="0" max="100000" step="1" inputmode="numeric" placeholder="0" bind:value={hours} /> {T.details.hours}</label>
          <label><input type="number" min="0" max="59" step="1" inputmode="numeric" placeholder="0" bind:value={minutes} /> {T.details.minutes}</label>
        </div>
      </fieldset>

      <aside class="help">
        <div class="help-title title">{T.details.helpTitle}</div>
        <ul>
          {#each T.details.help as line (line)}<li>{line}</li>{/each}
        </ul>
      </aside>

      {#if error}<p class="error" role="alert">{error}</p>{/if}

      <div class="foot end">
        <button class="btn" type="button" onclick={() => submit(false)} disabled={saving}>{T.details.skip}</button>
        <button class="btn btn-primary" type="submit" disabled={saving}>{saving ? T.details.saving : T.details.submit}</button>
      </div>
    </form>
  {/if}
</dialog>

<style>
  dialog {
    width: min(560px, calc(100vw - 32px));
    max-height: calc(100vh - 48px);
    padding: 22px 24px 20px;
    background: var(--surface);
    color: var(--text);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-lg);
    overflow-y: auto;
  }
  dialog::backdrop {
    background: rgba(5, 6, 8, 0.72);
  }
  .head {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }
  h2 {
    font-size: 24px;
  }
  .close {
    width: 44px;
    height: 44px;
    border: 0;
    background: transparent;
    color: var(--text-3);
    font-size: 18px;
    border-radius: var(--radius);
    cursor: pointer;
  }
  .close:hover {
    color: var(--text);
    background: var(--border);
  }
  .instruction {
    margin: 4px 0 16px;
    color: var(--text-2);
    font-size: 15px;
  }
  .toolbar {
    display: flex;
    gap: 8px;
    margin-bottom: 10px;
  }
  input[type="search"],
  input[type="text"],
  input[type="number"] {
    min-height: 44px;
    background: var(--bg);
    color: var(--text);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
    padding: 0 12px;
    font: inherit;
  }
  input:focus {
    outline: none;
    border-color: var(--accent);
  }
  .search {
    flex: 1;
  }
  .list {
    border: 1px solid var(--border);
    border-radius: var(--radius);
    height: 300px;
    overflow-y: auto;
    padding: 4px;
    background: var(--bg);
  }
  .item {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 12px;
    min-height: 56px;
    padding: 6px 10px;
    border: 1px solid transparent;
    border-radius: var(--radius);
    background: transparent;
    text-align: left;
    cursor: pointer;
  }
  .item:hover:not(:disabled) {
    background: var(--surface);
    border-color: var(--border-strong);
  }
  .item:disabled {
    opacity: 0.5;
    cursor: default;
  }
  .item-text {
    display: flex;
    flex-direction: column;
    min-width: 0;
    flex: 1;
  }
  .item-name {
    font-weight: 600;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .item-exe {
    font-size: 12px;
    color: var(--text-3);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .tag {
    font-size: 12px;
    color: var(--text-3);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    padding: 2px 8px;
  }
  .muted {
    color: var(--text-3);
  }
  .list .muted {
    padding: 16px 12px;
    margin: 0;
  }
  .small {
    font-size: 12px;
  }
  .check {
    display: flex;
    align-items: center;
    gap: 8px;
    min-height: 44px;
    color: var(--text-2);
    cursor: pointer;
  }
  .check input {
    width: 18px;
    height: 18px;
    accent-color: var(--accent);
  }
  .foot {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    margin-top: 8px;
  }
  .foot.end {
    justify-content: flex-end;
    margin-top: 18px;
  }
  .error {
    color: var(--alert-2);
    margin: 8px 0 0;
  }
  .back {
    border: 0;
    background: none;
    color: var(--text-3);
    min-height: 44px;
    padding: 0;
    cursor: pointer;
  }
  .back:hover {
    color: var(--text);
  }
  .picked {
    display: flex;
    align-items: center;
    gap: 14px;
    margin: 4px 0 18px;
  }
  .picked .item-name {
    font-size: 17px;
  }
  .field {
    display: flex;
    flex-direction: column;
    gap: 6px;
    margin: 0 0 16px;
    border: 0;
    padding: 0;
  }
  .label {
    font-size: 13px;
    font-weight: 600;
    color: var(--text-2);
    padding: 0;
  }
  .hint {
    margin: 0 0 4px;
  }
  .segmented {
    display: inline-flex;
    align-self: flex-start;
    background: var(--bg);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
    padding: 3px;
  }
  .segmented button {
    min-height: 38px;
    padding: 0 18px;
    border: 0;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--text-3);
    font-weight: 600;
    cursor: pointer;
  }
  .segmented button.on {
    background: var(--accent-bg);
    color: var(--accent);
  }
  .time {
    display: flex;
    gap: 16px;
  }
  .time label {
    display: flex;
    align-items: center;
    gap: 8px;
    color: var(--text-2);
  }
  .time input {
    width: 96px;
  }
  .help {
    background: var(--accent-bg);
    border: 1px solid #164453;
    border-radius: var(--radius);
    padding: 12px 14px;
  }
  .help-title {
    color: var(--accent);
    font-size: 14px;
    letter-spacing: 0.06em;
  }
  .help ul {
    margin: 6px 0 0;
    padding-left: 18px;
    color: var(--text-2);
    font-size: 13px;
  }
</style>
