<script lang="ts">
  import { page } from "$app/state";
  import { t } from "$lib/i18n";
  import { library } from "$lib/stores/library.svelte";

  const id = $derived(Number(page.url.searchParams.get("id")));
  const game = $derived(library.data?.games.find((g) => g.id === id));
</script>

<a class="back" href="/">← {t.game.back}</a>
<h1>{game?.name ?? ""}</h1>
<p class="soon">{t.game.soon}</p>

<style>
  .back {
    display: inline-flex;
    align-items: center;
    min-height: 44px;
    color: var(--text-3);
  }
  .back:hover {
    color: var(--text);
  }
  h1 {
    font-size: 34px;
    margin-top: 8px;
  }
  .soon {
    color: var(--text-3);
  }
</style>
