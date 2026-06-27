<script lang="ts">
  import { X } from "@lucide/svelte";
  import { open, confirm } from "@tauri-apps/plugin-dialog";
  import { library } from "$lib/stores/library.svelte";
  import { commands } from "$lib/api/commands";
  import type { SyncStatus, SyncStats } from "$lib/types";

  let { onclose }: { onclose: () => void } = $props();

  let roots = $state<string[]>([]);
  let syncStatus = $state<SyncStatus | null>(null);
  let dbUrl = $state("");
  let token = $state("");
  let saving = $state(false);
  let syncing = $state(false);
  let showCredentialForm = $state(false);
  let saveMessage = $state<{ ok: boolean; text: string } | null>(null);
  let syncMessage = $state<{ ok: boolean; text: string } | null>(null);

  $effect(() => {
    commands.libraryListRoots().then((r) => { roots = r; });
    commands.syncStatus().then((s) => { syncStatus = s; });
  });

  async function addFolder() {
    const path = await open({ directory: true, multiple: false, title: "Choose a music folder" });
    if (typeof path === "string") {
      await library.addFolder(path);
      roots = await commands.libraryListRoots();
    }
  }

  async function removeFolder(path: string) {
    const ok = await confirm(
      `Remove "${path}" from your library?\n\nAll tracks, play counts, and ratings for this folder will be deleted. This cannot be undone (unless cloud sync is configured).`,
      { title: "Remove music folder", kind: "warning" }
    );
    if (!ok) return;
    await commands.libraryRemoveRoot(path);
    roots = await commands.libraryListRoots();
    await library.refresh();
  }

  async function saveSync() {
    if (!dbUrl.trim() || !token.trim()) return;
    saving = true;
    saveMessage = null;
    try {
      await commands.syncConfigure(dbUrl.trim(), token.trim());
      syncStatus = await commands.syncStatus();
      saveMessage = { ok: true, text: "Credentials saved." };
      dbUrl = "";
      token = "";
      showCredentialForm = false;
    } catch (e) {
      saveMessage = { ok: false, text: String(e) };
    } finally {
      saving = false;
    }
  }

  async function syncNow() {
    syncing = true;
    syncMessage = null;
    try {
      const stats: SyncStats = await commands.syncNow();
      syncMessage = {
        ok: true,
        text: `Sync complete — ${stats.ratings_pushed} ratings pushed, ${stats.ratings_pulled} pulled, ${stats.plays_pushed} plays pushed, ${stats.plays_merged} merged.`,
      };
    } catch (e) {
      syncMessage = { ok: false, text: String(e) };
    } finally {
      syncing = false;
    }
  }
</script>

<div class="settings-shell">
  <button class="close-btn" onclick={onclose} aria-label="Close settings">
    <X size={18} />
  </button>

  <div class="settings">
  <section>
    <h2>Library</h2>
    {#if roots.length > 0}
      <ul class="folder-list">
        {#each roots as root (root)}
          <li class="folder-item">
            <span class="folder-path">{root}</span>
            <button class="remove-btn" onclick={() => removeFolder(root)} aria-label="Remove folder">
              <X size={14} />
            </button>
          </li>
        {/each}
      </ul>
    {/if}
    <div class="row">
      <button onclick={addFolder} disabled={library.loading}>Add Music Folder</button>
      <button onclick={() => library.rescan()} disabled={library.loading}>
        {library.loading ? "Scanning…" : "Rescan Library"}
      </button>
    </div>
  </section>

  <section>
    <h2>Cloud Sync</h2>

    {#if syncStatus?.configured}
      <div class="status-row">
        <span class="badge ok">Connected</span>
        <button class="inline-action" onclick={syncNow} disabled={syncing}>
          {syncing ? "Syncing…" : "Sync Now"}
        </button>
      </div>
      {#if syncMessage}
        <p class="message" class:error={!syncMessage.ok}>{syncMessage.text}</p>
      {/if}
      {#if syncStatus.machine_id}
        <p class="label">Machine ID: <code>{syncStatus.machine_id}</code></p>
      {/if}
      {#if !showCredentialForm}
        <div class="row">
          <button onclick={() => { showCredentialForm = true; saveMessage = null; }}>Change credentials</button>
        </div>
      {/if}
    {:else if syncStatus}
      <div class="status-row">
        <span class="badge off">Not configured</span>
      </div>
      <p class="label">Create a free database at <strong>turso.tech</strong> and paste the credentials below.</p>
    {/if}

    {#if showCredentialForm || !syncStatus?.configured}
      <label>
        Database URL
        <input
          type="url"
          bind:value={dbUrl}
          placeholder="https://your-db.turso.io"
          autocomplete="off"
          spellcheck={false}
        />
      </label>
      <label>
        Auth Token
        <input
          type="password"
          bind:value={token}
          placeholder="Enter token"
          autocomplete="new-password"
        />
      </label>
      <div class="row">
        <button class="primary" onclick={saveSync} disabled={saving || !dbUrl.trim() || !token.trim()}>
          {saving ? "Saving…" : "Save Credentials"}
        </button>
        {#if showCredentialForm}
          <button onclick={() => { showCredentialForm = false; saveMessage = null; }}>Cancel</button>
        {/if}
      </div>
      {#if saveMessage}
        <p class="message" class:error={!saveMessage.ok}>{saveMessage.text}</p>
      {/if}
    {/if}
  </section>
  </div>
</div>

<style>
  .settings-shell {
    position: absolute;
    inset: 0;
  }

  .settings {
    position: absolute;
    inset: 0;
    overflow-y: auto;
    padding: 2em 2.5em;
    display: flex;
    flex-direction: column;
    gap: 2em;
    max-width: 640px;
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

  .folder-list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .folder-item {
    display: flex;
    align-items: center;
    gap: 0.5em;
    background: var(--bg-hover);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 0.4em 0.4em 0.4em 0.75em;
  }

  .folder-path {
    flex: 1;
    font-family: monospace;
    font-size: 0.9em;
    color: var(--text-secondary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .remove-btn {
    flex-shrink: 0;
    width: 24px;
    height: 24px;
    padding: 0;
    background: none;
    border: none;
    color: var(--text-tertiary);
    border-radius: var(--radius-sm);
  }

  .remove-btn:hover:not(:disabled) {
    background: #3a1a1a;
    color: #e07070;
  }

  .row {
    display: flex;
    gap: 0.75em;
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
  }

  .close-btn:hover:not(:disabled) {
    background: var(--bg-hover);
    color: var(--text-primary);
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
    color: #fff;
  }

  button.primary:hover:not(:disabled) {
    background: var(--accent-hover);
    border-color: var(--accent-hover);
  }

  .status-row {
    display: flex;
    align-items: center;
    gap: 1em;
    flex-wrap: wrap;
  }

  .badge {
    font-size: 1em;
    font-weight: 600;
    padding: 0.2em 0.6em;
    border-radius: var(--radius-sm);
  }

  .badge.ok {
    background: #1a3a1a;
    color: #6fcf6f;
  }

  .badge.off {
    background: var(--bg-hover);
    color: var(--text-tertiary);
  }

  .inline-action {
    margin-left: auto;
  }

  .label {
    margin: 0;
    font-size: 1em;
    color: var(--text-secondary);
    line-height: 1.5;
  }

  .label code {
    font-family: monospace;
    color: var(--text-primary);
  }


  label {
    display: flex;
    flex-direction: column;
    gap: 0.4em;
    font-size: 1em;
    color: var(--text-secondary);
  }

  input[type="url"],
  input[type="password"] {
    background: var(--bg-hover);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    color: var(--text-primary);
    padding: 0.5em 0.75em;
    font-size: 1em;
    font-family: inherit;
  }

  input[type="url"]:focus,
  input[type="password"]:focus {
    outline: 1px solid var(--accent);
    border-color: var(--accent);
  }

  input::placeholder {
    color: var(--text-tertiary);
  }

  .message {
    margin: 0;
    font-size: 1em;
    color: var(--text-secondary);
  }

  .message.error {
    color: #e07070;
  }
</style>
