<script lang="ts">
  import "$lib/styles/theme.css";
  import ArtistList from "$lib/components/sidebar/ArtistList.svelte";
  import TransportBar from "$lib/components/transport/TransportBar.svelte";
  import { library } from "$lib/stores/library.svelte";

  let { children } = $props();

  library.refresh();
</script>

<div class="app-shell">
  <aside class="sidebar">
    <ArtistList />
  </aside>
  <main class="content">
    {@render children()}
  </main>
  <footer class="transport-bar">
    <TransportBar />
  </footer>
</div>

<style>
  .app-shell {
    position: relative;
    height: 100vh;
  }

  .sidebar {
    position: absolute;
    top: 0;
    left: 0;
    bottom: var(--transport-height);
    width: var(--sidebar-width);
    display: flex;
    flex-direction: column;
    overflow-y: auto;
    border-right: 1px solid var(--border);
    background: var(--bg-elevated);
  }

  .content {
    position: absolute;
    top: 0;
    left: var(--sidebar-width);
    right: 0;
    bottom: var(--transport-height);
    overflow: hidden;
  }

  .transport-bar {
    position: absolute;
    left: 0;
    right: 0;
    bottom: 0;
    height: var(--transport-height);
    border-top: 1px solid var(--border);
    background: var(--bg-elevated);
  }
</style>
