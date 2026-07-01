<script lang="ts">
  import "$lib/styles/theme.css";
  import { Settings } from "@lucide/svelte";
  import { untrack } from "svelte";
  import ArtistList from "$lib/components/sidebar/ArtistList.svelte";
  import TransportBar from "$lib/components/transport/TransportBar.svelte";
  import SettingsPanel from "$lib/components/settings/SettingsPanel.svelte";
  import { library } from "$lib/stores/library.svelte";
  import { player } from "$lib/stores/player.svelte";
  import { ui } from "$lib/stores/ui.svelte";

  let { children } = $props();

  $effect(() => {
    if (library.selectedAlbumId !== null) ui.closeSettings();
  });

  $effect(() => {
    const albumId = player.snapshot.album_id;
    if (albumId !== null) untrack(() => library.markAlbumPlayed(albumId));
  });

  library.refresh().then(() => library.rescan());
</script>

<div class="app-shell">
  <aside class="sidebar">
    <ArtistList />
  </aside>
  <main class="content">
    <div class="toolbar">
      <div class="toolbar-left">
        {#if library.selectedAlbumId === null && !ui.showSettings}
          <div class="sort-tabs" role="group" aria-label="Sort albums by">
            <button
              class="sort-tab"
              class:active={ui.albumSort === 'date_added'}
              onclick={() => ui.setAlbumSort('date_added')}
            >
              Newest
            </button>
            <button
              class="sort-tab"
              class:active={ui.albumSort === 'name'}
              onclick={() => ui.setAlbumSort('name')}
            >
              Name
            </button>
          </div>
        {/if}
      </div>
      <button
        class="cog"
        class:active={ui.showSettings}
        onclick={() => (ui.showSettings ? ui.closeSettings() : ui.openSettings())}
        aria-label="Settings"
      >
        <Settings size={20} />
      </button>
    </div>
    <div class="content-body">
      {#if ui.showSettings}
        <SettingsPanel onclose={() => ui.closeSettings()} />
      {:else}
        {@render children()}
      {/if}
    </div>
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
    overflow-x: hidden;
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

  .toolbar {
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    height: var(--toolbar-height);
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 0.75em;
    border-bottom: 1px solid var(--border);
  }

  .toolbar-left {
    display: flex;
    align-items: center;
  }

  .sort-tabs {
    display: flex;
    gap: 2px;
    background: var(--bg-hover);
    border-radius: var(--radius-sm);
    padding: 2px;
  }

  .sort-tab {
    background: none;
    border: none;
    border-radius: var(--radius-sm);
    color: var(--text-tertiary);
    cursor: pointer;
    font-size: 0.8em;
    padding: 3px 10px;
  }

  .sort-tab.active {
    background: var(--bg-selected);
    color: var(--text-primary);
  }

  .sort-tab:hover:not(.active) {
    color: var(--text-secondary);
  }

  .cog {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 32px;
    height: 32px;
    background: none;
    border: none;
    border-radius: var(--radius-sm);
    color: var(--text-tertiary);
    cursor: pointer;
  }

  .cog:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
  }

  .cog.active {
    color: var(--accent);
  }

  .content-body {
    position: absolute;
    top: var(--toolbar-height);
    left: 0;
    right: 0;
    bottom: 0;
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
