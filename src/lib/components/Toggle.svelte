<script lang="ts">
  // Interrupteur on/off avec libellé et explication (ligne de réglage).
  let {
    label,
    hint = "",
    checked,
    disabled = false,
    onchange,
  }: { label: string; hint?: string; checked: boolean; disabled?: boolean; onchange: (v: boolean) => void } = $props();
</script>

<label class="row" class:disabled>
  <span class="text">
    <span class="label">{label}</span>
    {#if hint}<span class="hint">{hint}</span>{/if}
  </span>
  <input type="checkbox" role="switch" {checked} {disabled} onchange={(e) => onchange(e.currentTarget.checked)} />
  <span class="switch" aria-hidden="true"></span>
</label>

<style>
  .row {
    display: flex;
    align-items: center;
    gap: 16px;
    min-height: 60px;
    padding: 10px 0;
    cursor: pointer;
    position: relative;
  }
  .disabled {
    opacity: 0.6;
    cursor: default;
  }
  .text {
    flex: 1;
    display: flex;
    flex-direction: column;
  }
  .label {
    font-weight: 600;
  }
  .hint {
    color: var(--text-3);
    font-size: 13px;
  }
  input {
    position: absolute;
    opacity: 0;
    width: 1px;
    height: 1px;
  }
  .switch {
    width: 46px;
    height: 26px;
    flex-shrink: 0;
    border-radius: 13px;
    background: var(--border-strong);
    position: relative;
    transition: background 0.15s;
  }
  .switch::after {
    content: "";
    position: absolute;
    top: 3px;
    left: 3px;
    width: 20px;
    height: 20px;
    border-radius: 50%;
    background: var(--text-2);
    transition: transform 0.15s;
  }
  input:checked + .switch {
    background: var(--accent);
  }
  input:checked + .switch::after {
    transform: translateX(20px);
    background: #06181d;
  }
  input:focus-visible + .switch {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }
</style>
