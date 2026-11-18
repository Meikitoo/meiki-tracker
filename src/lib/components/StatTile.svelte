<script lang="ts">
  let {
    label,
    value,
    sub = "",
    tone = "neutral",
  }: { label: string; value: string; sub?: string; tone?: "neutral" | "up" | "down" } = $props();
</script>

<div class="tile">
  <div class="label title">{label}</div>
  <div class="value num">{value}</div>
  {#if sub}<div class="sub {tone}">{sub}</div>{/if}
</div>

<style>
  .tile {
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    padding: 16px 18px;
    min-width: 0;
  }
  .label {
    font-size: 13px;
    color: var(--text-3);
    letter-spacing: 0.08em;
  }
  .value {
    font-size: 36px;
    line-height: 1.1;
    margin-top: 4px;
  }
  .sub {
    margin-top: 4px;
    font-size: 13px;
    color: var(--text-3);
  }
  .sub.up {
    color: var(--accent);
  }
  .sub.down {
    color: var(--alert-2);
  }
</style>
