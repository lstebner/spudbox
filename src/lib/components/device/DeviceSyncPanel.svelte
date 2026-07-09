<script lang="ts">
  import { X, HardDrive, RefreshCw, Check } from "@lucide/svelte";
  import { confirm } from "@tauri-apps/plugin-dialog";
  import { commands } from "$lib/api/commands";
  import { device } from "$lib/stores/device.svelte";
  import type { DeviceSyncPreview } from "$lib/types";

  let { onclose }: { onclose: () => void } = $props();

  async function handleClose() {
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
    onclose();
  }

  // "scanning" → "found" → "none" (no candidates at all)
  type FolderScanState = "scanning" | "found" | "none";
  let folderScanState = $state<FolderScanState>("scanning");
  let folderCandidates = $state<string[]>([]);
  let selectedFolder = $state(device.detectedMusicSubfolder ?? "");

  let preview = $state<DeviceSyncPreview | null>(null);
  let errorMessage = $state<string | null>(null);
  let successMessage = $state<string | null>(null);

  async function scanFolders() {
    folderScanState = "scanning";
    folderCandidates = [];
    selectedFolder = "";
    preview = null;
    errorMessage = null;
    successMessage = null;
    try {
      const found = await commands.deviceFindMusicFolders();
      folderCandidates = found;
      if (found.length > 0) {
        selectedFolder = found[0];
        folderScanState = "found";
      } else {
        folderScanState = "none";
      }
    } catch {
      folderScanState = "none";
    }
  }

  // Scan as soon as the panel opens.
  scanFolders();

  function formatBytes(bytes: number): string {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
    return `${(bytes / (1024 * 1024 * 1024)).toFixed(2)} GB`;
  }

  function notEnoughSpace(): boolean {
    return !!preview && preview.required_bytes > preview.device_free_bytes;
  }

  async function runPreview() {
    if (!selectedFolder) return;
    // Capture the folder before any awaits. If the user switches folders or
    // triggers a rescan while the preview is in flight, the stale result is
    // discarded rather than overwriting whatever is now selected.
    const capturedFolder = selectedFolder;
    preview = null;
    errorMessage = null;
    successMessage = null;
    try {
      await commands.deviceSaveMusicSubfolder(capturedFolder);
      const result = await device.performPreview(capturedFolder);
      if (result !== null && selectedFolder === capturedFolder) {
        preview = result;
      }
    } catch (error) {
      if (selectedFolder === capturedFolder) {
        errorMessage = String(error);
      }
    }
  }

  async function runSync(mode: "additions_only" | "all") {
    if (!preview || !selectedFolder) return;
    const snapshotPreview = preview;
    errorMessage = null;
    successMessage = null;
    try {
      const result = await device.performSync(selectedFolder, mode, snapshotPreview);
      if (result.cancelled) {
        const planned = snapshotPreview.to_add.length + (mode === "all" ? snapshotPreview.to_delete.length : 0);
        successMessage =
          `Sync cancelled — ${result.copied} of ${planned} file${planned !== 1 ? "s" : ""} copied` +
          (result.deleted > 0 ? `, ${result.deleted} removed` : "") + ".";
      } else {
        successMessage =
          `Sync complete — ${result.copied} file${result.copied !== 1 ? "s" : ""} copied` +
          (result.deleted > 0 ? `, ${result.deleted} removed` : "") + ".";
        preview = null;
      }
    } catch (error) {
      errorMessage = String(error);
    }
  }
</script>

<div class="panel-shell">
  <button class="close-btn" onclick={handleClose} aria-label="Close device sync">
    <X size={18} />
  </button>

  <div class="panel">
    <section>
      <h2>Device</h2>
      <div class="device-row">
        <HardDrive size={16} />
        <span class="device-name">{device.deviceName || "MTP Device"}</span>
        <span class="badge connected">Connected</span>
      </div>
    </section>

    <section>
      <h2>Music folder</h2>

      {#if folderScanState === "scanning"}
        <p class="label scanning">Scanning device…</p>

      {:else if folderScanState === "none"}
        <p class="label">No music folder detected on this device.</p>
        <p class="label hint-text">
          Connect a device that has a folder named "Music", or ensure the device
          is mounted and accessible.
        </p>
        <button class="rescan-btn" onclick={scanFolders}>
          <RefreshCw size={14} />
          Scan again
        </button>

      {:else}
        {#if folderCandidates.length === 1}
          <div class="single-folder">
            <span class="folder-path">{selectedFolder}</span>
            <button class="rescan-btn" onclick={scanFolders} aria-label="Scan again">
              <RefreshCw size={14} />
            </button>
          </div>
        {:else}
          <div class="folder-options" role="radiogroup" aria-label="Select music folder">
            {#each folderCandidates as folder (folder)}
              <label class="folder-option" class:selected={selectedFolder === folder}>
                <input
                  type="radio"
                  name="music-folder"
                  value={folder}
                  checked={selectedFolder === folder}
                  onchange={() => { selectedFolder = folder; preview = null; }}
                />
                <span class="folder-path">{folder}</span>
              </label>
            {/each}
          </div>
          <button class="rescan-btn" onclick={scanFolders}>
            <RefreshCw size={14} />
            Scan again
          </button>
        {/if}
      {/if}
    </section>

    {#if errorMessage}
      <p class="message error">{errorMessage}</p>
    {/if}

    {#if successMessage}
      <p class="message success">{successMessage}</p>
    {/if}

    {#if preview}
      <section>
        <h2>Preview</h2>

        {#if notEnoughSpace()}
          <div class="space-warning">
            Not enough space — need {formatBytes(preview.required_bytes)} but only {formatBytes(preview.device_free_bytes)} free.
            Use "Add Missing Only" to copy only what fits.
          </div>
        {:else}
          <p class="label">
            {formatBytes(preview.device_free_bytes)} free on device
            {#if preview.to_add.length > 0}· {formatBytes(preview.required_bytes)} required{/if}
          </p>
        {/if}

        {#if preview.to_add.length === 0 && preview.to_delete.length === 0}
          <p class="label">Device is already in sync with your library.</p>
        {:else}
          <ul class="change-list">
            {#each preview.to_add as entry (entry.relative_path)}
              {@const done = device.completedPaths.has(entry.relative_path)}
              <li class="change-item" class:done>
                <span class="change-icon" class:add={!done} class:done aria-label={done ? "Copied" : "Add"}>
                  {#if done}
                    <Check size={12} />
                  {:else}
                    +
                  {/if}
                </span>
                <span class="change-text">
                  <span class="change-artist">{entry.artist}</span>
                  <span class="change-sep">·</span>
                  <span class="change-album">{entry.album}</span>
                  <span class="change-sep">·</span>
                  <span class="change-title">{entry.title}</span>
                </span>
              </li>
            {/each}
            {#each preview.to_delete as entry (entry.relative_path)}
              {@const done = device.completedPaths.has(entry.relative_path)}
              <li class="change-item" class:done>
                <span class="change-icon" class:remove={!done} class:done aria-label={done ? "Removed" : "Remove"}>
                  {#if done}
                    <Check size={12} />
                  {:else}
                    −
                  {/if}
                </span>
                <span class="change-text">
                  <span class="change-artist">{entry.artist}</span>
                  <span class="change-sep">·</span>
                  <span class="change-album">{entry.album}</span>
                  <span class="change-sep">·</span>
                  <span class="change-title">{entry.title}</span>
                </span>
              </li>
            {/each}
          </ul>
        {/if}
      </section>
    {/if}
  </div>

  <div class="action-bar">
    {#if device.syncRunning}
      <div
        class="action-bar-progress"
        role="progressbar"
        aria-label="Sync progress"
        aria-valuenow={device.syncProgress?.current ?? 0}
        aria-valuemax={device.syncProgress?.total ?? 0}
      >
        <p class="progress-label">
          {#if device.syncProgress}
            Syncing… {device.syncProgress.current} / {device.syncProgress.total} files
          {:else}
            Scanning device…{device.previewProgressCount > 0 ? ` ${device.previewProgressCount} tracks found` : ""}
          {/if}
        </p>
        <button class="ghost" onclick={() => device.cancelSync()}>Cancel</button>
      </div>
    {:else if device.previewRunning}
      <div class="action-bar-progress">
        <p class="progress-label">
          Scanning device…{device.previewProgressCount > 0 ? ` ${device.previewProgressCount} tracks found` : ""}
        </p>
      </div>
    {:else}
      <div class="action-bar-buttons">
        {#if folderScanState === "found"}
          <button
            class={preview ? "ghost" : "primary"}
            onclick={runPreview}
            disabled={!selectedFolder || device.previewRunning || device.syncRunning}
          >
            {preview ? "Refresh preview" : "Preview Sync"}
          </button>
        {/if}
        {#if preview}
          {#if preview.to_add.length > 0 && preview.to_delete.length === 0}
            <button class="primary" onclick={() => runSync("additions_only")} disabled={device.syncRunning || notEnoughSpace()}>
              Add Music
            </button>
          {:else if preview.to_add.length > 0 && preview.to_delete.length > 0}
            <button class="primary" onclick={() => runSync("additions_only")} disabled={device.syncRunning || notEnoughSpace()}>
              Add Missing Only
            </button>
            <button class="destructive" onclick={() => runSync("all")} disabled={device.syncRunning}>
              Sync All
            </button>
          {:else if preview.to_delete.length > 0}
            <button class="destructive" onclick={() => runSync("all")} disabled={device.syncRunning}>
              Sync All
            </button>
          {/if}
        {/if}
      </div>
    {/if}
  </div>
</div>

<style>
  .panel-shell {
    position: absolute;
    inset: 0;
  }

  .panel {
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    bottom: 60px;
    overflow-y: auto;
    padding: 2em 2.5em;
    display: flex;
    flex-direction: column;
    gap: 2em;
  }

  .action-bar {
    position: absolute;
    bottom: 0;
    left: 0;
    right: 0;
    height: 60px;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 0 2.5em;
    border-top: 1px solid var(--border);
    background: var(--bg-elevated);
  }

  .action-bar-buttons {
    display: flex;
    align-items: center;
    gap: 0.75em;
  }

  .action-bar-progress {
    display: flex;
    flex-direction: row;
    align-items: center;
    gap: 1em;
    width: 100%;
    max-width: 480px;
    justify-content: center;
  }

  section {
    display: flex;
    flex-direction: column;
    gap: 1em;
  }

  h2 {
    margin: 0;
    font-size: 0.75em;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--text-tertiary);
  }

  .close-btn {
    position: absolute;
    top: 0.75em;
    right: 0.75em;
    width: 32px;
    height: 32px;
    padding: 0;
    background: none;
    border: none;
    color: var(--text-tertiary);
    z-index: 1;
    cursor: pointer;
    border-radius: var(--radius-sm);
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .close-btn:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
  }

  .device-row {
    display: flex;
    align-items: center;
    gap: 0.6em;
    color: var(--text-primary);
  }

  .device-name {
    font-weight: 500;
  }

  .badge {
    font-size: 0.8em;
    font-weight: 600;
    padding: 0.2em 0.6em;
    border-radius: var(--radius-sm);
  }

  .badge.connected {
    background: var(--success-bg);
    color: var(--success);
  }

  .label {
    margin: 0;
    font-size: 1em;
    color: var(--text-secondary);
    line-height: 1.5;
  }

  .label.scanning {
    color: var(--text-tertiary);
    font-style: italic;
  }

  .hint-text {
    color: var(--text-tertiary);
    font-size: 0.9em;
  }

  /* Single detected folder */
  .single-folder {
    display: flex;
    align-items: center;
    gap: 0.5em;
    background: var(--bg-hover);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 0.5em 0.5em 0.5em 0.75em;
  }

  /* Multiple folder choices */
  .folder-options {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .folder-option {
    display: flex;
    align-items: center;
    gap: 0.6em;
    background: var(--bg-hover);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 0.5em 0.75em;
    cursor: pointer;
  }

  .folder-option.selected {
    border-color: var(--accent);
    background: var(--accent-tint);
  }

  .folder-option input[type="radio"] {
    accent-color: var(--accent);
    cursor: pointer;
  }

  .folder-path {
    flex: 1;
    font-family: monospace;
    font-size: 0.9em;
    color: var(--text-primary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .rescan-btn {
    display: flex;
    align-items: center;
    gap: 0.4em;
    background: none;
    border: none;
    color: var(--text-tertiary);
    cursor: pointer;
    font-size: 0.85em;
    font-family: inherit;
    padding: 0.2em 0.4em;
    border-radius: var(--radius-sm);
    flex-shrink: 0;
  }

  .rescan-btn:hover {
    color: var(--text-secondary);
    background: var(--bg-hover);
  }

  button {
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--bg-hover);
    border: 1px solid var(--border);
    color: var(--text-secondary);
    padding: 0.5em 1em;
    border-radius: var(--radius);
    cursor: pointer;
    font-size: 1em;
    font-family: inherit;
  }

  button:hover:not(:disabled) {
    background: var(--bg-selected);
    color: var(--text-primary);
  }

  button:disabled {
    opacity: 0.5;
    cursor: default;
  }

  button.primary {
    background: var(--accent);
    border-color: var(--accent);
    color: var(--accent-contrast);
  }

  button.primary:hover:not(:disabled) {
    background: var(--accent-hover);
    border-color: var(--accent-hover);
  }

  button.ghost {
    background: none;
    border-color: transparent;
    color: var(--text-tertiary);
  }

  button.ghost:hover:not(:disabled) {
    background: var(--bg-hover);
    border-color: var(--border);
    color: var(--text-secondary);
  }

  button.destructive {
    border-color: var(--danger-border);
    color: var(--danger);
  }

  button.destructive:hover:not(:disabled) {
    background: var(--danger-bg-strong);
    border-color: var(--danger);
    color: var(--danger);
  }

  .space-warning {
    background: var(--warning-bg);
    border: 1px solid var(--warning-border);
    border-radius: var(--radius);
    color: var(--warning);
    padding: 0.75em 1em;
    line-height: 1.5;
  }


  .change-list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .change-item {
    display: flex;
    align-items: baseline;
    gap: 0.6em;
  }

  .change-icon {
    flex-shrink: 0;
    font-size: 1em;
    font-weight: 700;
    width: 1em;
    text-align: center;
    line-height: 1;
  }

  .change-icon.add { color: var(--success); }
  .change-icon.remove { color: var(--danger); }
  .change-icon.done { color: var(--text-tertiary); display: flex; align-items: center; justify-content: center; }

  .change-item.done {
    opacity: 0.4;
  }

  .change-text {
    display: flex;
    align-items: baseline;
    gap: 0.4em;
    flex-wrap: wrap;
    line-height: 1.4;
  }

  .change-artist {
    color: var(--text-primary);
  }

  .change-sep {
    color: var(--text-tertiary);
  }

  .change-album {
    color: var(--text-secondary);
  }

  .change-title {
    color: var(--text-secondary);
  }

  .message {
    margin: 0;
    font-size: 1em;
  }

  .message.error { color: var(--danger); }
  .message.success { color: var(--success); }

  .progress-label {
    margin: 0;
    font-size: 1em;
    color: var(--text-secondary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
