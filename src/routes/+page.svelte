<script lang="ts">
  import { t } from "$lib/i18n";
</script>

<main>
  <div class="logo">M</div>
  <h1>{t.app.name}</h1>
  <p>{t.app.by}</p>
</main>

<style>
  :global(html, body) {
    margin: 0;
    height: 100%;
    background: #0b0d10;
    color: #ffffff;
    font-family: system-ui, sans-serif;
  }
  main {
    height: 100vh;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 8px;
  }
  .logo {
    width: 64px;
    height: 64px;
    border: 2px solid #22d3ee;
    border-radius: 10px;
    color: #22d3ee;
    display: grid;
    place-items: center;
    font-size: 36px;
    font-weight: 700;
  }
  h1 {
    margin: 12px 0 0;
    letter-spacing: 0.08em;
  }
  p {
    margin: 0;
    color: #9aa4ae;
    letter-spacing: 0.12em;
    font-size: 13px;
  }
</style>
