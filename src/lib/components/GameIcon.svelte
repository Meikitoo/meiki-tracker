<script lang="ts">
  // Icône d'un jeu : l'image extraite du .exe, sinon l'initiale du nom.
  let { src = null, name, size = 44 }: { src?: string | null; name: string; size?: number } = $props();
  const initial = $derived(name.trim().charAt(0).toUpperCase() || "?");
</script>

<div class="icon" style:width="{size}px" style:height="{size}px" style:font-size="{Math.round(size / 2)}px" aria-hidden="true">
  {#if src}
    <img {src} alt="" width={size - 8} height={size - 8} />
  {:else}
    <span class="title">{initial}</span>
  {/if}
</div>

<style>
  .icon {
    flex-shrink: 0;
    border-radius: var(--radius);
    background: var(--accent-bg);
    color: var(--accent);
    display: grid;
    place-items: center;
    overflow: hidden;
  }
  img {
    object-fit: contain;
    image-rendering: auto;
  }
</style>
