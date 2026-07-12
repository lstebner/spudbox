<script lang="ts">
  import "@fontsource/inter/300.css";
  import "@fontsource/inter/400.css";
  import "@fontsource/inter/500.css";
  import "@fontsource/inter/600.css";
  import "@fontsource/inter/700.css";
  import "$lib/styles/theme.css";
  import { confirm } from "@tauri-apps/plugin-dialog";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { HardDrive, Settings, Waves } from "@lucide/svelte";
  import { untrack } from "svelte";
  import ArtistList from "$lib/components/sidebar/ArtistList.svelte";
  import TransportBar from "$lib/components/transport/TransportBar.svelte";
  import SettingsPanel from "$lib/components/settings/SettingsPanel.svelte";
  import DeviceSyncPanel from "$lib/components/device/DeviceSyncPanel.svelte";
  import NowPlayingDrawer from "$lib/components/nowplaying/NowPlayingDrawer.svelte";
  import VisualizerPanel from "$lib/components/visualizer/VisualizerPanel.svelte";
  import ThemeSwitcher from "$lib/components/theme/ThemeSwitcher.svelte";
  import Dropdown from "$lib/components/common/Dropdown.svelte";
  import { library } from "$lib/stores/library.svelte";
  import { player } from "$lib/stores/player.svelte";
  import { ui, type AlbumSort } from "$lib/stores/ui.svelte";
  import { device } from "$lib/stores/device.svelte";
  import { theme } from "$lib/stores/theme.svelte";

  let { children } = $props();

  const ALBUM_SORT_OPTIONS: { value: AlbumSort; label: string }[] = [
    { value: "date_added", label: "Newest first" },
    { value: "artist_name", label: "Artist name" },
    { value: "album_name", label: "Album name" },
  ];

  // Elements that already treat Space as their own native activation
  // (buttons, form controls) must keep that behavior instead of also
  // toggling playback.
  const SPACE_ACTIVATING_TAG_NAMES = new Set(["BUTTON", "INPUT", "TEXTAREA", "SELECT"]);

  function handleGlobalKeydown(event: KeyboardEvent) {
    // Closing the window already fully quits the app (no close-to-tray
    // interception exists), so Ctrl+Q just needs to trigger the same close.
    if (event.ctrlKey && !event.shiftKey && !event.altKey && !event.metaKey && event.code === "KeyQ") {
      event.preventDefault();
      getCurrentWindow().close();
      return;
    }

    if (event.key === "Escape" && ui.showVisualizer) {
      event.preventDefault();
      ui.closeVisualizer();
      return;
    }

    if (event.code !== "Space") return;
    const target = event.target;
    if (
      target instanceof HTMLElement &&
      (SPACE_ACTIVATING_TAG_NAMES.has(target.tagName) || target.isContentEditable)
    ) {
      return;
    }
    event.preventDefault();
    // Ignore OS auto-repeat: player.snapshot.state only updates from the
    // backend's ~4Hz playback-progress event, not synchronously after a
    // command is sent, so a held key would re-read the same stale state
    // and spam the same toggle command on every repeat.
    if (event.repeat) return;
    player.togglePlayPause();
  }

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

<svelte:window onkeydown={handleGlobalKeydown} />

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
            <Dropdown
              options={ALBUM_SORT_OPTIONS}
              value={ui.albumSort}
              onChange={(sort) => ui.setAlbumSort(sort)}
              ariaLabel="Sort albums by"
            />
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
        <button
          class="cog"
          class:active={ui.showVisualizer}
          onclick={() => (ui.showVisualizer ? ui.closeVisualizer() : ui.openVisualizer())}
          aria-label="Visualizer"
        >
          <Waves size={20} />
        </button>
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
  {#if ui.showVisualizer}
    <VisualizerPanel onclose={() => ui.closeVisualizer()} />
  {/if}
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
    border-right: 1px solid var(--chrome-border);
    background: var(--chrome-bg);
    color: var(--chrome-text-primary);
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
    border-top: 1px solid var(--chrome-border);
    background: var(--chrome-bg);
    color: var(--chrome-text-primary);
  }
</style>
