<script lang="ts">
  import { createVirtualizer } from "@tanstack/svelte-virtual";
  import { convertFileSrc } from "@tauri-apps/api/core";
  import { get } from "svelte/store";
  import { Eye, EyeOff } from "@lucide/svelte";
  import { library } from "$lib/stores/library.svelte";
  import StarRating from "$lib/components/rating/StarRating.svelte";

  const MIN_CARD_WIDTH = 170;
  const CARD_GAP = 20;
  // Sized for 3 text lines (title + subtitle + rating row); always reserve
  // the rating row's height even for unrated albums so every card in the
  // virtualizer is the same height regardless of rating state.
  const TEXT_HEIGHT = 62;

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

  // Pick the column count that fits at the minimum card width, then stretch
  // cards (and their square art) to fill whatever space is left over, so a
  // window size between two exact column-fits doesn't just leave a dead gap
  // on the right of every row.
  const columnsPerRow = $derived(
    Math.max(1, Math.floor((containerWidth + CARD_GAP) / (MIN_CARD_WIDTH + CARD_GAP))),
  );
  const cardWidth = $derived(
    containerWidth > 0
      ? (containerWidth - (columnsPerRow - 1) * CARD_GAP) / columnsPerRow
      : MIN_CARD_WIDTH,
  );
  const rowHeight = $derived(cardWidth + TEXT_HEIGHT + CARD_GAP);
  const rowCount = $derived(Math.ceil(library.albums.length / columnsPerRow));

  const virtualizer = createVirtualizer<HTMLDivElement, HTMLDivElement>({
    count: 0,
    getScrollElement: () => scrollEl ?? null,
    estimateSize: () => rowHeight,
    overscan: 4,
  });

  $effect(() => {
    get(virtualizer).setOptions({
      count: rowCount,
      getScrollElement: () => scrollEl ?? null,
      estimateSize: () => rowHeight,
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
    <p>Use "Add Music Folder" in Settings to get started.</p>
  </div>
{:else}
<div bind:this={scrollEl} class="grid-scroll">
  <div class="grid-inner" style="height: {$virtualizer.getTotalSize()}px;">
    {#each $virtualizer.getVirtualItems() as row (row.key)}
      <div class="grid-row" style="transform: translateY({row.start}px);">
        {#each albumsForRow(row.index) as album (album.id)}
          <div class="album-wrap" style="width: {cardWidth}px;">
            <button
              class="album-card"
              onclick={() => library.selectAlbum(album.id)}
            >
              <div class="art" style="width: {cardWidth}px; height: {cardWidth}px;">
                {#if album.art_path}
                  <img src={convertFileSrc(album.art_path)} alt={album.title} loading="lazy" />
                {:else}
                  <div class="art-placeholder"></div>
                {/if}
              </div>
              <div class="title">{album.title}</div>
              <div class="subtitle">{album.album_artist}{album.year ? ` · ${album.year}` : ""}</div>
              <div class="rating-row">
                <StarRating rating={album.rating} readonly size={12} />
              </div>
            </button>
            <button
              class="hide-toggle"
              aria-label={library.isViewingHidden ? "Show album" : "Hide album"}
              onclick={() => library.setAlbumHidden(album.id, !library.isViewingHidden)}
            >
              {#if library.isViewingHidden}
                <EyeOff size={15} />
              {:else}
                <Eye size={15} />
              {/if}
            </button>
          </div>
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

  .album-wrap {
    position: relative;
  }

  .album-card {
    width: 100%;
    background: none;
    border: none;
    color: inherit;
    text-align: left;
    cursor: pointer;
    padding: 0;
    border-radius: var(--radius);
  }

  .art {
    border-radius: var(--radius);
    overflow: hidden;
    background: var(--bg-hover);
    margin-bottom: 0.5em;
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.4);
  }

  .hide-toggle {
    position: absolute;
    top: 6px;
    right: 6px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: rgba(0, 0, 0, 0.55);
    border: none;
    border-radius: var(--radius-sm);
    color: rgba(255, 255, 255, 0.9);
    cursor: pointer;
    padding: 4px;
    opacity: 0;
    transition: opacity 0.15s;
  }

  .hide-toggle:hover {
    background: rgba(0, 0, 0, 0.75);
    color: #fff;
  }

  .album-wrap:hover .hide-toggle {
    opacity: 1;
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
    font-size: 0.9em;
    margin-top: 2px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .rating-row {
    margin-top: 0.35em;
  }

  .album-wrap:hover .title {
    color: var(--accent-hover);
  }
</style>
