<script lang="ts">
  import { createVirtualizer } from "@tanstack/svelte-virtual";
  import { convertFileSrc } from "@tauri-apps/api/core";
  import { get } from "svelte/store";
  import { library } from "$lib/stores/library.svelte";

  const CARD_WIDTH = 170;
  const CARD_GAP = 20;
  const ROW_HEIGHT = 170 + 44 + CARD_GAP;

  let scrollEl: HTMLDivElement | undefined = $state();
  let containerWidth = $state(0);

  $effect(() => {
    if (!scrollEl) return;
    const observer = new ResizeObserver((entries) => {
      containerWidth = entries[0].contentRect.width;
    });
    observer.observe(scrollEl);
    return () => observer.disconnect();
  });

  const columnsPerRow = $derived(
    Math.max(1, Math.floor((containerWidth + CARD_GAP) / (CARD_WIDTH + CARD_GAP))),
  );
  const rowCount = $derived(Math.ceil(library.albums.length / columnsPerRow));

  const virtualizer = createVirtualizer<HTMLDivElement, HTMLDivElement>({
    count: 0,
    getScrollElement: () => scrollEl ?? null,
    estimateSize: () => ROW_HEIGHT,
    overscan: 4,
  });

  $effect(() => {
    get(virtualizer).setOptions({
      count: rowCount,
      getScrollElement: () => scrollEl ?? null,
      estimateSize: () => ROW_HEIGHT,
      overscan: 4,
    });
  });

  function albumsForRow(rowIndex: number) {
    const start = rowIndex * columnsPerRow;
    return library.albums.slice(start, start + columnsPerRow);
  }
</script>

{#if !library.hasRoots}
  <div class="empty-state">
    <p>No music folder added yet.</p>
    <p>Use "Add Music Folder" in the sidebar to get started.</p>
  </div>
{:else}
<div bind:this={scrollEl} class="grid-scroll">
  <div class="grid-inner" style="height: {$virtualizer.getTotalSize()}px;">
    {#each $virtualizer.getVirtualItems() as row (row.key)}
      <div class="grid-row" style="transform: translateY({row.start}px);">
        {#each albumsForRow(row.index) as album (album.id)}
          <button class="album-card" onclick={() => library.selectAlbum(album.id)}>
            <div class="art">
              {#if album.art_path}
                <img src={convertFileSrc(album.art_path)} alt={album.title} loading="lazy" />
              {:else}
                <div class="art-placeholder"></div>
              {/if}
            </div>
            <div class="title">{album.title}</div>
            <div class="subtitle">{album.album_artist}{album.year ? ` · ${album.year}` : ""}</div>
          </button>
        {/each}
      </div>
    {/each}
  </div>
</div>
{/if}

<style>
  .empty-state {
    position: absolute;
    inset: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 0.25em;
    color: var(--text-secondary);
    text-align: center;
  }

  .grid-scroll {
    position: absolute;
    inset: 0;
    overflow-y: auto;
    padding: 1.5em;
  }

  .grid-inner {
    position: relative;
    width: 100%;
  }

  .grid-row {
    position: absolute;
    top: 0;
    left: 0;
    display: flex;
    gap: 20px;
    width: 100%;
  }

  .album-card {
    width: 170px;
    background: none;
    border: none;
    color: inherit;
    text-align: left;
    cursor: pointer;
    padding: 0;
    border-radius: var(--radius);
  }

  .art {
    width: 170px;
    height: 170px;
    border-radius: var(--radius);
    overflow: hidden;
    background: var(--bg-hover);
    margin-bottom: 0.5em;
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.4);
  }

  .art img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    display: block;
  }

  .art-placeholder {
    width: 100%;
    height: 100%;
    background: linear-gradient(135deg, var(--bg-hover), var(--bg-selected));
  }

  .title {
    font-weight: 600;
    font-size: 0.9em;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .subtitle {
    color: var(--text-secondary);
    font-size: 0.8em;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .album-card:hover .title {
    color: var(--accent-hover);
  }
</style>
