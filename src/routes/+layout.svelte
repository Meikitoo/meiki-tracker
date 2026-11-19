<script lang="ts">
  import "$lib/styles/theme.css";
  import { onMount } from "svelte";
  import Sidebar from "$lib/components/Sidebar.svelte";
  import { library } from "$lib/stores/library.svelte";
  import { getSettings, inTauri, setLanguage } from "$lib/api";
  import { lang, rememberLang } from "$lib/i18n";
  import { updates } from "$lib/stores/update.svelte";

  // La langue enregistrée dans la base fait foi (ex. : après un import ou une
  // réinstallation). Premier lancement : on y enregistre la langue détectée.
  async function syncLanguage() {
    if (!inTauri) return;
    try {
      const s = await getSettings();
      if (s.updateCheck) updates.check();
      const saved = s.language;
      if (!saved) await setLanguage(lang);
      else if (saved !== lang) {
        rememberLang(saved);
        location.reload();
      }
    } catch {
      // sans gravité : on garde la langue actuelle
    }
  }

  let { children } = $props();

  onMount(() => {
    document.documentElement.lang = lang;
    syncLanguage();
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
