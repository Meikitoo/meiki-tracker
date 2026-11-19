<script lang="ts">
  import { getGameIcon, getRecap, saveRecapImage, type Recap, type RecapPeriod } from "$lib/api";
  import { fmtDuration } from "$lib/format";
  import { t } from "$lib/i18n";
  import { drawCard, loadCardFonts, type CardFormat } from "$lib/recap/card";

  const T = t.recap;
  const PERIODS: RecapPeriod[] = ["week", "month", "year"];
  const FORMATS: CardFormat[] = ["square", "story"];

  function stored<V extends string>(key: string, allowed: V[], fallback: V): V {
    try {
      const v = localStorage.getItem(key) as V | null;
      if (v && allowed.includes(v)) return v;
    } catch {
      // stockage indisponible
    }
    return fallback;
  }
  function store(key: string, v: string) {
    try {
      localStorage.setItem(key, v);
    } catch {
      // sans gravité
    }
  }

  let period = $state<RecapPeriod>(stored("meiki.recap.period", PERIODS, "week"));
  let format = $state<CardFormat>(stored("meiki.recap.format", FORMATS, "square"));
  let recap = $state<Recap | null>(null);
  let icons = $state<Record<number, string | null>>({});
  let canvas = $state<HTMLCanvasElement>();
  let error = $state<string | null>(null);
  let message = $state<string | null>(null);
  let busy = $state(false);

  async function load(p: RecapPeriod) {
    try {
      const r = await getRecap(p);
      const ids = r.topGames.map((g) => g.id).filter((id) => !(id in icons));
      const loaded = await Promise.all(ids.map(async (id) => [id, await getGameIcon(id).catch(() => null)] as const));
      icons = { ...icons, ...Object.fromEntries(loaded) };
      recap = r;
      error = null;
    } catch (e) {
      error = String(e);
    }
  }

  $effect(() => {
    store("meiki.recap.period", period);
    message = null;
    load(period);
  });

  // Redessine dès que les données ou le format changent.
  $effect(() => {
    store("meiki.recap.format", format);
    const r = recap;
    const c = canvas;
    const f = format;
    const ic = icons;
    if (!r || !c) return;
    loadCardFonts().then(() => drawCard(c, { recap: r, period, format: f, icons: ic, T, locale: t.locale, fmtDuration }));
  });

  function fileName() {
    const d = new Date();
    const stamp = `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}-${String(d.getDate()).padStart(2, "0")}`;
    return `meiki-recap-${period}-${stamp}.png`;
  }

  async function save() {
    if (!canvas) return;
    busy = true;
    message = null;
    try {
      const base64 = canvas.toDataURL("image/png").split(",")[1];
      const path = await saveRecapImage(T.saveTitle, fileName(), base64);
      if (path) {
        error = null;
        message = T.saved(path);
      }
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  async function copy() {
    if (!canvas) return;
    message = null;
    try {
      const blob = await new Promise<Blob | null>((res) => canvas!.toBlob(res, "image/png"));
      if (!blob) throw new Error("toBlob");
      await navigator.clipboard.write([new ClipboardItem({ "image/png": blob })]);
      error = null;
      message = T.copied;
    } catch {
      error = T.copyFailed;
    }
  }
</script>

<h1>{T.title}</h1>
<p class="sub">{T.subtitle}</p>

<div class="layout">
  <div class="controls">
    <div class="group">
      <span class="label">{T.periodLabel}</span>
      <div class="segmented" role="radiogroup" aria-label={T.periodLabel}>
        {#each PERIODS as p (p)}
          <button type="button" role="radio" aria-checked={period === p} class:active={period === p} onclick={() => (period = p)}>
            {T.periods[p]}
          </button>
        {/each}
      </div>
    </div>
    <div class="group">
      <span class="label">{T.formatLabel}</span>
      <div class="segmented" role="radiogroup" aria-label={T.formatLabel}>
        {#each FORMATS as f (f)}
          <button type="button" role="radio" aria-checked={format === f} class:active={format === f} onclick={() => (format = f)}>
            {T.formats[f]}
          </button>
        {/each}
      </div>
    </div>

    <div class="actions">
      <button class="btn btn-primary" type="button" disabled={!recap || busy} onclick={save}>{T.save}</button>
      <button class="btn" type="button" disabled={!recap} onclick={copy}>{T.copy}</button>
    </div>

    {#if recap && recap.totalSecs === 0}<p class="hint">{T.empty}</p>{/if}
    {#if message}<p class="msg ok" role="status">{message}</p>{/if}
    {#if error}<p class="msg error" role="alert">{error}</p>{/if}
  </div>

  <div class="preview" class:story={format === "story"}>
    <canvas bind:this={canvas} aria-label={T.title}></canvas>
  </div>
</div>

<style>
  h1 {
    font-size: 34px;
  }
  .sub {
    color: var(--text-3);
    margin: 4px 0 24px;
  }
  .layout {
    display: flex;
    gap: 28px;
    align-items: flex-start;
  }
  .controls {
    width: 320px;
    flex-shrink: 0;
    display: flex;
    flex-direction: column;
    gap: 20px;
  }
  .group {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .label {
    font-weight: 600;
    color: var(--text-2);
  }
  .segmented {
    display: flex;
    flex-direction: column;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
    overflow: hidden;
  }
  .segmented button {
    min-height: 44px;
    padding: 0 16px;
    text-align: left;
    background: transparent;
    border: 0;
    color: var(--text-2);
    font: inherit;
    font-weight: 600;
    cursor: pointer;
  }
  .segmented button + button {
    border-top: 1px solid var(--border-strong);
  }
  .segmented button.active {
    background: var(--accent-bg);
    color: var(--accent);
  }
  .actions {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .hint {
    color: var(--text-3);
    font-size: 13px;
    margin: 0;
  }
  .msg {
    margin: 0;
    padding: 10px 14px;
    border-radius: var(--radius);
    word-break: break-word;
    font-size: 13px;
  }
  .msg.ok {
    border: 1px solid var(--accent);
    background: var(--accent-bg);
  }
  .msg.error {
    border: 1px solid var(--alert);
    color: var(--alert-2);
    background: #2a0f14;
  }
  .preview {
    flex: 1;
    min-width: 0;
    display: flex;
    justify-content: center;
  }
  canvas {
    width: 100%;
    max-width: 640px;
    height: auto;
    border-radius: 12px;
    box-shadow: 0 12px 40px rgba(0, 0, 0, 0.5);
  }
  .preview.story canvas {
    width: auto;
    max-width: 100%;
    max-height: calc(100vh - 160px);
  }
</style>
