<script lang="ts">
  import { open } from "@tauri-apps/plugin-dialog";
  import { library } from "$lib/stores/library.svelte";

  async function addFolder() {
    const path = await open({ directory: true, multiple: false, title: "Choose a music folder" });
    if (typeof path === "string") {
      await library.addFolder(path);
    }
  }
</script>

<nav class="artist-list">
  <button
    class="artist-item"
    class:active={library.selectedArtistId === null}
    onclick={() => library.selectArtist(null)}
  >
    All Albums
  </button>
  {#each library.artists as artist (artist.id)}
    <button
      class="artist-item"
      class:active={library.selectedArtistId === artist.id}
      onclick={() => library.selectArtist(artist.id)}
    >
      <span class="name">{artist.name}</span>
      <span class="count">{artist.album_count}</span>
    </button>
  {/each}
</nav>

<div class="rescan">
  <button onclick={addFolder} disabled={library.loading}>Add Music Folder</button>
  <button onclick={() => library.rescan()} disabled={library.loading}>
    {library.loading ? "Scanning…" : "Rescan Library"}
  </button>
</div>

<style>
  .artist-list {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 0.75em 0.5em;
  }

  .artist-item {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 0.5em;
    width: 100%;
    text-align: left;
    background: none;
    border: none;
    padding: 0.5em 0.75em;
    border-radius: var(--radius);
    cursor: pointer;
    color: var(--text-secondary);
  }

  .artist-item:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
  }

  .artist-item.active {
    background: var(--bg-selected);
    color: var(--text-primary);
  }

  .name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .count {
    color: var(--text-tertiary);
    font-size: 0.85em;
    flex-shrink: 0;
  }

  .rescan {
    margin-top: auto;
    padding: 0.75em 0.5em;
    display: flex;
    flex-direction: column;
    gap: 0.4em;
  }

  .rescan button {
    width: 100%;
    background: none;
    border: 1px solid var(--border);
    color: var(--text-secondary);
    padding: 0.5em;
    border-radius: var(--radius);
    cursor: pointer;
  }

  .rescan button:hover:not(:disabled) {
    background: var(--bg-hover);
    color: var(--text-primary);
  }

  .rescan button:disabled {
    cursor: default;
    opacity: 0.6;
  }
</style>
