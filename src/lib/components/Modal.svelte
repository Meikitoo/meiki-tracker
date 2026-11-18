<script lang="ts">
  // Fenêtre modale générique (confirmation, saisie…). Échap ou ✕ pour fermer.
  import type { Snippet } from "svelte";

  let {
    open = $bindable(false),
    title,
    children,
    actions,
  }: { open?: boolean; title: string; children?: Snippet; actions?: Snippet } = $props();

  let dialog = $state<HTMLDialogElement>();
  const titleId = `modal-${Math.random().toString(36).slice(2)}`;

  $effect(() => {
    if (!dialog) return;
    if (open && !dialog.open) dialog.showModal();
    else if (!open && dialog.open) dialog.close();
  });
</script>

<dialog bind:this={dialog} onclose={() => (open = false)} aria-labelledby={titleId}>
  {#if open}
    <h2 id={titleId}>{title}</h2>
    <div class="body">{@render children?.()}</div>
    <div class="actions">{@render actions?.()}</div>
  {/if}
</dialog>

<style>
  dialog {
    width: min(440px, calc(100vw - 32px));
    padding: 22px 24px 20px;
    background: var(--surface);
    color: var(--text);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-lg);
  }
  dialog::backdrop {
    background: rgba(5, 6, 8, 0.72);
  }
  h2 {
    font-size: 22px;
    margin-bottom: 8px;
  }
  .body {
    color: var(--text-2);
  }
  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 10px;
    margin-top: 20px;
  }
</style>
