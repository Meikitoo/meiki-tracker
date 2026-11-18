<script lang="ts">
  import "$lib/styles/theme.css";
  import { onMount } from "svelte";
  import Sidebar from "$lib/components/Sidebar.svelte";
  import { library } from "$lib/stores/library.svelte";

  let { children } = $props();

  onMount(() => {
    library.start();
  });
</script>

<div class="shell">
  <Sidebar />
  <main>
    {@render children()}
  </main>
</div>

<style>
  .shell {
    display: flex;
    height: 100vh;
    overflow: hidden;
  }
  main {
    flex: 1;
    overflow-y: auto;
    padding: 28px 32px 40px;
  }
</style>
