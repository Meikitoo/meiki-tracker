<script lang="ts">
  import { page } from "$app/state";
  import { setTrackingPaused } from "$lib/api";
  import { t } from "$lib/i18n";
  import { library } from "$lib/stores/library.svelte";

  const links = [
    { href: "/", label: t.nav.library, icon: "grid" },
    { href: "/statistiques", label: t.nav.stats, icon: "chart" },
    { href: "/parametres", label: t.nav.settings, icon: "gear" },
  ] as const;

  function isActive(href: string) {
    const path = page.url.pathname;
    return href === "/" ? path === "/" || path.startsWith("/jeu") : path.startsWith(href);
  }

  const playing = $derived(library.live[0]);

  async function togglePause() {
    const next = !library.paused;
    library.paused = next; // affichage immédiat ; confirmé par le rechargement
    try {
      await setTrackingPaused(next);
    } catch {
      library.paused = !next;
    }
  }
</script>

<aside>
  <div class="brand">
    <div class="logo" aria-hidden="true">M</div>
    <div>
      <div class="name title">{t.app.name}</div>
      <div class="by">{t.app.by}</div>
    </div>
  </div>

  <nav>
    {#each links as l (l.href)}
      <a href={l.href} class:active={isActive(l.href)} aria-current={isActive(l.href) ? "page" : undefined}>
        <svg viewBox="0 0 24 24" aria-hidden="true">
          {#if l.icon === "grid"}
            <rect x="3" y="3" width="7" height="7" rx="1.5" /><rect x="14" y="3" width="7" height="7" rx="1.5" />
            <rect x="3" y="14" width="7" height="7" rx="1.5" /><rect x="14" y="14" width="7" height="7" rx="1.5" />
          {:else if l.icon === "chart"}
            <path d="M4 20V10M10 20V4M16 20v-7M22 20H2" />
          {:else}
            <circle cx="12" cy="12" r="3" />
            <path d="M12 2v3M12 19v3M2 12h3M19 12h3M4.9 4.9l2.1 2.1M17 17l2.1 2.1M4.9 19.1 7 17M17 7l2.1-2.1" />
          {/if}
        </svg>
        {l.label}
      </a>
    {/each}
  </nav>

  <div class="status" class:paused={library.paused}>
    <div class="status-head">
      <span class="dot" class:pulse={!library.paused}></span>
      <span class="title">{library.paused ? t.tracking.paused : t.tracking.active}</span>
    </div>
    <div class="status-sub">
      {#if playing && !library.paused}
        {t.tracking.playing(playing.name)}
      {:else}
        {t.tracking.every(library.data?.checkIntervalSecs ?? 5)}
      {/if}
    </div>
    <button class="pause" type="button" onclick={togglePause}>
      {library.paused ? t.tracking.resume : t.tracking.pause}
    </button>
  </div>
</aside>

<style>
  aside {
    width: 240px;
    flex-shrink: 0;
    background: var(--sidebar);
    border-right: 1px solid var(--border);
    display: flex;
    flex-direction: column;
    padding: 20px 14px;
    gap: 28px;
  }
  .brand {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 0 6px;
  }
  .logo {
    width: 40px;
    height: 40px;
    border: 2px solid var(--accent);
    border-radius: var(--radius);
    color: var(--accent);
    display: grid;
    place-items: center;
    font-family: var(--font-title);
    font-weight: 700;
    font-size: 24px;
  }
  .name {
    font-size: 17px;
    line-height: 1.1;
    letter-spacing: 0.06em;
  }
  .by {
    font-size: 11px;
    color: var(--text-3);
    letter-spacing: 0.14em;
    font-weight: 600;
  }
  nav {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  nav a {
    min-height: 44px;
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 0 12px;
    border-radius: var(--radius);
    color: var(--text-3);
    font-weight: 500;
    border: 1px solid transparent;
  }
  nav a:hover {
    color: var(--text);
    background: var(--surface);
  }
  nav a.active {
    color: var(--accent);
    background: var(--accent-bg);
    border-color: #164453;
  }
  svg {
    width: 18px;
    height: 18px;
    fill: none;
    stroke: currentColor;
    stroke-width: 2;
    stroke-linecap: round;
    stroke-linejoin: round;
  }
  .status {
    margin-top: auto;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    padding: 12px 14px;
  }
  .status-head {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 14px;
    color: var(--accent);
  }
  .paused .status-head {
    color: var(--text-3);
  }
  .dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: currentColor;
  }
  .pulse {
    animation: pulse 2s ease-in-out infinite;
  }
  @keyframes pulse {
    50% {
      opacity: 0.35;
    }
  }
  .status-sub {
    margin-top: 4px;
    color: var(--text-3);
    font-size: 12px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .pause {
    margin-top: 10px;
    width: 100%;
    min-height: 36px;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--text-2);
    font-size: 13px;
    font-weight: 600;
    cursor: pointer;
  }
  .pause:hover {
    border-color: var(--text-3);
    color: var(--text);
  }
  @media (prefers-reduced-motion: reduce) {
    .pulse {
      animation: none;
    }
  }
</style>
