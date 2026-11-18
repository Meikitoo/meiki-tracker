<script lang="ts">
  import { updateSession, type SessionRow } from "$lib/api";
  import { fmtDay, fmtDuration, fmtTime } from "$lib/format";
  import { t } from "$lib/i18n";

  let {
    session,
    ondelete,
    onchanged,
  }: { session: SessionRow; ondelete: (s: SessionRow) => void; onchanged: () => void } = $props();

  const T = t.game.sessions;
  const PRESETS: readonly string[] = T.tags;

  let editing = $state(false);
  let tag = $state<string | null>(null);
  let customTag = $state("");
  let note = $state("");
  let saving = $state(false);
  let error = $state<string | null>(null);

  function startEdit() {
    const current = session.tag;
    tag = current && PRESETS.includes(current) ? current : current ? "__custom" : null;
    customTag = current && !PRESETS.includes(current) ? current : "";
    note = session.note ?? "";
    error = null;
    editing = true;
  }

  async function save() {
    saving = true;
    try {
      const finalTag = tag === "__custom" ? customTag.trim() || null : tag;
      await updateSession(session.id, finalTag, note.trim() || null);
      editing = false;
      onchanged();
    } catch (e) {
      error = String(e);
    } finally {
      saving = false;
    }
  }
</script>

<li class="row" class:editing>
  <div class="main">
    <div class="when">
      <span class="day">{fmtDay(session.startedAt)}</span>
      <span class="hours">{fmtTime(session.startedAt)} → {fmtTime(session.endedAt)}</span>
    </div>
    <div class="meta">
      {#if session.tag}<span class="tag">{session.tag}</span>{/if}
      {#if session.note}<span class="note" title={session.note}>{session.note}</span>{/if}
    </div>
    <div class="dur">
      <span class="num">{fmtDuration(session.durationSecs)}</span>
      {#if session.afkSecs > 0}<span class="small">{T.afk(fmtDuration(session.afkSecs))}</span>{/if}
      {#if session.gapSecs > 0}<span class="small">{T.pauses(fmtDuration(session.gapSecs))}</span>{/if}
    </div>
    <div class="actions">
      <button class="icon-btn" type="button" aria-label={t.game.edit} title={t.game.edit} onclick={() => (editing ? (editing = false) : startEdit())}>
        <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M4 20h4L19 9l-4-4L4 16v4zM14 6l4 4" /></svg>
      </button>
      <button class="icon-btn danger" type="button" aria-label={T.delete} title={T.delete} onclick={() => ondelete(session)}>
        <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M4 7h16M10 11v6M14 11v6M6 7l1 13h10l1-13M9 7V4h6v3" /></svg>
      </button>
    </div>
  </div>

  {#if editing}
    <form
      class="edit"
      onsubmit={(e) => {
        e.preventDefault();
        save();
      }}
    >
      <div class="field">
        <span class="label" id="tag-{session.id}">{T.tag}</span>
        <div class="chips" role="radiogroup" aria-labelledby="tag-{session.id}">
          <button type="button" role="radio" aria-checked={tag === null} class:on={tag === null} onclick={() => (tag = null)}>{T.noTag}</button>
          {#each PRESETS as p (p)}
            <button type="button" role="radio" aria-checked={tag === p} class:on={tag === p} onclick={() => (tag = p)}>{p}</button>
          {/each}
          <button type="button" role="radio" aria-checked={tag === "__custom"} class:on={tag === "__custom"} onclick={() => (tag = "__custom")}>
            {T.customTag}
          </button>
        </div>
        {#if tag === "__custom"}
          <input class="input" type="text" maxlength="30" bind:value={customTag} placeholder={T.customTag} aria-label={T.customTag} />
        {/if}
      </div>
      <label class="field">
        <span class="label">{T.note}</span>
        <input class="input" type="text" maxlength="200" bind:value={note} placeholder={T.notePlaceholder} />
      </label>
      {#if error}<p class="error" role="alert">{error}</p>{/if}
      <div class="edit-actions">
        <button class="btn" type="button" onclick={() => (editing = false)}>{t.game.cancel}</button>
        <button class="btn btn-primary" type="submit" disabled={saving}>{t.game.save}</button>
      </div>
    </form>
  {/if}
</li>

<style>
  .row {
    list-style: none;
    border-bottom: 1px solid var(--border);
  }
  .row:last-child {
    border-bottom: 0;
  }
  .main {
    display: grid;
    grid-template-columns: 210px 1fr auto auto;
    align-items: center;
    gap: 16px;
    min-height: 60px;
    padding: 8px 8px 8px 16px;
  }
  .when {
    display: flex;
    flex-direction: column;
  }
  .day {
    font-weight: 600;
  }
  .day::first-letter {
    text-transform: uppercase;
  }
  .hours {
    color: var(--text-3);
    font-size: 13px;
    font-variant-numeric: tabular-nums;
  }
  .meta {
    display: flex;
    align-items: center;
    gap: 10px;
    min-width: 0;
  }
  .tag {
    flex-shrink: 0;
    font-size: 12px;
    font-weight: 600;
    color: var(--accent);
    background: var(--accent-bg);
    border: 1px solid #164453;
    border-radius: var(--radius-sm);
    padding: 2px 8px;
  }
  .note {
    color: var(--text-2);
    font-size: 13px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .dur {
    display: flex;
    flex-direction: column;
    align-items: flex-end;
  }
  .dur .num {
    font-size: 22px;
    line-height: 1.1;
  }
  .small {
    font-size: 11px;
    color: var(--text-3);
  }
  .actions {
    display: flex;
  }
  .edit {
    padding: 4px 16px 16px;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .field {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .label {
    font-size: 13px;
    font-weight: 600;
    color: var(--text-2);
  }
  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  .chips button {
    min-height: 36px;
    padding: 0 12px;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    background: var(--bg);
    color: var(--text-2);
    cursor: pointer;
  }
  .chips button.on {
    background: var(--accent-bg);
    border-color: var(--accent);
    color: var(--accent);
  }
  .edit-actions {
    display: flex;
    justify-content: flex-end;
    gap: 10px;
  }
  .error {
    color: var(--alert-2);
    margin: 0;
  }
</style>
