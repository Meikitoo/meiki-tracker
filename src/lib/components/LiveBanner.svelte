<script lang="ts">
  import type { LiveInfo } from "$lib/api";
  import { fmtClock } from "$lib/format";
  import { t } from "$lib/i18n";

  let { info, secs }: { info: LiveInfo; secs: number } = $props();
</script>

<a class="banner" href={`/jeu?id=${info.gameId}`} aria-live="off">
  <span class="pill title"><span class="dot"></span>{t.library.live}</span>
  <span class="name">{info.name}</span>
  {#if info.afk}<span class="afk">{t.library.afk}</span>{/if}
  <span class="clock num" class:dim={info.afk}>{fmtClock(secs)}</span>
</a>

<style>
  .banner {
    display: flex;
    align-items: center;
    gap: 16px;
    background: var(--accent-bg);
    border: 1px solid var(--accent);
    border-radius: var(--radius-lg);
    padding: 12px 20px;
    min-height: 64px;
  }
  .pill {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    color: var(--accent);
    font-size: 15px;
    letter-spacing: 0.1em;
  }
  .dot {
    width: 9px;
    height: 9px;
    border-radius: 50%;
    background: var(--accent);
    animation: blink 1.6s ease-in-out infinite;
  }
  @keyframes blink {
    50% {
      opacity: 0.3;
    }
  }
  .name {
    font-weight: 600;
    font-size: 16px;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .afk {
    font-size: 12px;
    color: var(--text-3);
  }
  .clock {
    margin-left: auto;
    font-size: 34px;
    line-height: 1;
    color: var(--text);
  }
  .clock.dim {
    color: var(--text-3);
  }
  @media (prefers-reduced-motion: reduce) {
    .dot {
      animation: none;
    }
  }
</style>
