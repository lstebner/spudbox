<script lang="ts">
  import "$lib/styles/theme.css";
  import { HardDrive, Settings } from "@lucide/svelte";
  import { untrack } from "svelte";
  import ArtistList from "$lib/components/sidebar/ArtistList.svelte";
  import TransportBar from "$lib/components/transport/TransportBar.svelte";
  import SettingsPanel from "$lib/components/settings/SettingsPanel.svelte";
  import DeviceSyncPanel from "$lib/components/device/DeviceSyncPanel.svelte";
  import { library } from "$lib/stores/library.svelte";
  import { player } from "$lib/stores/player.svelte";
  import { ui, type AlbumSort } from "$lib/stores/ui.svelte";
  import { device } from "$lib/stores/device.svelte";

  let { children } = $props();

  $effect(() => {
    if (library.selectedAlbumId !== null) {
      ui.closeSettings();
      ui.closeDeviceSync();
    }
  });

  $effect(() => {
    const albumId = player.snapshot.album_id;
    if (albumId !== null) untrack(() => library.markAlbumPlayed(albumId));
  });

  library.refresh().then(() => library.rescan());
  device.init();
</script>

<div class="app-shell">
  <aside class="sidebar">
    <ArtistList />
  </aside>
  <main class="content">
    <div class="toolbar">
      <div class="toolbar-left">
        {#if library.selectedAlbumId === null && !ui.showSettings}
          <div class="sort-control">
            <span class="sort-label">Sort by</span>
            <select
              class="sort-select"
              aria-label="Sort albums by"
              value={ui.albumSort}
              onchange={(e) => ui.setAlbumSort(e.currentTarget.value as AlbumSort)}
            >
              <option value="date_added">Newest first</option>
              <option value="artist_name">Artist name</option>
              <option value="album_name">Album name</option>
            </select>
          </div>
        {/if}
      </div>
      <div class="toolbar-right">
        {#if device.connected}
          <button
            class="cog"
            class:active={ui.showDeviceSync}
            onclick={() => (ui.showDeviceSync ? ui.closeDeviceSync() : ui.openDeviceSync())}
            aria-label="Device sync"
          >
            <HardDrive size={20} />
          </button>
        {/if}
        <button
          class="cog"
          class:active={ui.showSettings}
          onclick={() => (ui.showSettings ? ui.closeSettings() : ui.openSettings())}
          aria-label="Settings"
        >
          <Settings size={20} />
        </button>
      </div>
    </div>
    <div class="content-body">
      {#if ui.showSettings}
        <SettingsPanel onclose={() => ui.closeSettings()} />
      {:else if ui.showDeviceSync}
        <DeviceSyncPanel onclose={() => ui.closeDeviceSync()} />
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

  .sort-control {
    display: flex;
    align-items: center;
    gap: 0.5em;
  }

  .sort-label {
    color: var(--text-tertiary);
    white-space: nowrap;
  }

  .sort-select {
    appearance: none;
    background-color: var(--bg-hover);
    background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='12' height='12' viewBox='0 0 24 24' fill='none' stroke='%2395959c' stroke-width='2' stroke-linecap='round' stroke-linejoin='round'%3E%3Cpath d='M6 9l6 6 6-6'/%3E%3C/svg%3E");
    background-repeat: no-repeat;
    background-position: right 0.5em center;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    color: var(--text-primary);
    cursor: pointer;
    font-family: inherit;
    font-size: inherit;
    padding: 0.4em 1.8em 0.4em 0.6em;
    outline: none;
  }

  .sort-select:focus {
    border-color: var(--accent);
  }

  .toolbar-right {
    display: flex;
    align-items: center;
    gap: 2px;
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
