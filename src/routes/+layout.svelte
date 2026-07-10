<script lang="ts">
  import "@fontsource/inter/300.css";
  import "@fontsource/inter/400.css";
  import "@fontsource/inter/500.css";
  import "@fontsource/inter/600.css";
  import "@fontsource/inter/700.css";
  import "$lib/styles/theme.css";
  import { confirm } from "@tauri-apps/plugin-dialog";
  import { HardDrive, Settings } from "@lucide/svelte";
  import { untrack } from "svelte";
  import ArtistList from "$lib/components/sidebar/ArtistList.svelte";
  import TransportBar from "$lib/components/transport/TransportBar.svelte";
  import SettingsPanel from "$lib/components/settings/SettingsPanel.svelte";
  import DeviceSyncPanel from "$lib/components/device/DeviceSyncPanel.svelte";
  import NowPlayingDrawer from "$lib/components/nowplaying/NowPlayingDrawer.svelte";
  import ThemeSwitcher from "$lib/components/theme/ThemeSwitcher.svelte";
  import { library } from "$lib/stores/library.svelte";
  import { player } from "$lib/stores/player.svelte";
  import { ui, type AlbumSort } from "$lib/stores/ui.svelte";
  import { device } from "$lib/stores/device.svelte";
  import { theme } from "$lib/stores/theme.svelte";

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
  theme.init();
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
            <div class="sort-select-wrap">
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
          </div>
        {/if}
      </div>
      <div class="toolbar-right">
        {#if device.connected}
          <button
            class="cog"
            class:active={ui.showDeviceSync}
            onclick={async () => {
              if (ui.showDeviceSync) {
                if (device.syncRunning) {
                  const confirmed = await confirm("A sync is in progress — stop it and close?", {
                    title: "Stop sync?",
                    kind: "warning",
                    okLabel: "Stop & close",
                    cancelLabel: "Keep syncing",
                  });
                  if (!confirmed) return;
                  await device.cancelSync();
                } else if (device.previewRunning) {
                  const confirmed = await confirm("A preview scan is in progress — stop it and close?", {
                    title: "Stop preview?",
                    kind: "warning",
                    okLabel: "Stop & close",
                    cancelLabel: "Keep scanning",
                  });
                  if (!confirmed) return;
                  await device.cancelPreview();
                }
                ui.closeDeviceSync();
              } else {
                ui.openDeviceSync();
              }
            }}
            aria-label="Device sync"
          >
            <HardDrive size={20} />
          </button>
        {/if}
        <ThemeSwitcher />
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
  <NowPlayingDrawer />
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

  .sort-select-wrap {
    position: relative;
  }

  .sort-select-wrap::after {
    content: "";
    position: absolute;
    top: 50%;
    right: 0.5em;
    width: 12px;
    height: 12px;
    transform: translateY(-50%);
    pointer-events: none;
    background-color: var(--text-tertiary);
    mask-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='24' height='24' viewBox='0 0 24 24' fill='none' stroke='%23000' stroke-width='2' stroke-linecap='round' stroke-linejoin='round'%3E%3Cpath d='M6 9l6 6 6-6'/%3E%3C/svg%3E");
    mask-repeat: no-repeat;
    mask-position: center;
    mask-size: contain;
  }

  .sort-select {
    appearance: none;
    background-color: var(--bg-hover);
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
