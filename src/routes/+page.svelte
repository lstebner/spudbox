<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";

  let pingResult = $state("checking backend...");

  async function checkBackend() {
    try {
      pingResult = await invoke("ping");
    } catch (e) {
      pingResult = `error: ${e}`;
    }
  }

  checkBackend();
</script>

<main class="container">
  <h1>Music Player</h1>
  <p>Backend status: {pingResult}</p>
</main>

<style>
  :root {
    font-family: Inter, Avenir, Helvetica, Arial, sans-serif;
    color: #0f0f0f;
    background-color: #f6f6f6;
  }

  .container {
    margin: 0;
    padding-top: 10vh;
    display: flex;
    flex-direction: column;
    justify-content: center;
    text-align: center;
  }

  @media (prefers-color-scheme: dark) {
    :root {
      color: #f6f6f6;
      background-color: #2f2f2f;
    }
  }
</style>
