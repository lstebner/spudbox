<script lang="ts">
  import { createVirtualizer } from "@tanstack/svelte-virtual";
  import { convertFileSrc } from "@tauri-apps/api/core";
  import { ArrowLeft, Play, X } from "@lucide/svelte";
  import { get } from "svelte/store";
  import { library } from "$lib/stores/library.svelte";
  import { player } from "$lib/stores/player.svelte";
  import { formatDuration } from "$lib/format";
  import StarRating from "$lib/components/rating/StarRating.svelte";

  const ROW_HEIGHT = 36;

  let scrollEl: HTMLDivElement | undefined = $state();

  const album = $derived(library.albums.find((a) => a.id === library.selectedAlbumId) ?? null);

  const virtualizer = createVirtualizer<HTMLDivElement, HTMLDivElement>({
    count: 0,
    getScrollElement: () => scrollEl ?? null,
    estimateSize: () => ROW_HEIGHT,
    overscan: 10,
  });

  $effect(() => {
    get(virtualizer).setOptions({
      count: library.tracks.length,
      getScrollElement: () => scrollEl ?? null,
      estimateSize: () => ROW_HEIGHT,
      overscan: 10,
    });
  });

  let artModalOpen = $state(false);

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "Escape") artModalOpen = false;
  }

  function playFrom(index: number) {
    player.playQueue(library.tracks.map((t) => t.id), index);
  }
</script>

<svelte:window onkeydown={handleKeydown} />

{#if artModalOpen && album?.art_path}
  <div
    class="art-modal"
    role="dialog"
    aria-modal="true"
    tabindex="-1"
    onclick={() => (artModalOpen = false)}
    onkeydown={handleKeydown}
  >
    <button
      class="art-modal-close"
      onclick={(e) => { e.stopPropagation(); artModalOpen = false; }}
      aria-label="Close"
    >
      <X size={20} />
    </button>
    <img
      src={convertFileSrc(album.art_path)}
      alt={album.title}
      class="art-modal-img"
    />
  </div>
{/if}

<div class="track-view">
  <div class="header">
    <button class="back" onclick={() => library.backToAlbums()}>
      <ArrowLeft size={16} />
      Albums
    </button>
    {#if album}
      <div class="art">
        {#if album.art_path}
          <button class="art-btn" onclick={() => (artModalOpen = true)} aria-label="View full artwork">
            <img src={convertFileSrc(album.art_path)} alt={album.title} />
          </button>
        {:else}
          <div class="art-placeholder"></div>
        {/if}
      </div>
      <div>
        <div class="title">{album.title}</div>
        <div class="subtitle">{album.album_artist}{album.year ? ` · ${album.year}` : ""}</div>
        <div class="rating-row">
          <StarRating rating={album.rating} size={16} onRate={(r) => library.setAlbumRating(album.id, r)} />
        </div>
      </div>
    {/if}
  </div>

  <div bind:this={scrollEl} class="track-scroll">
    <div class="track-inner" style="height: {$virtualizer.getTotalSize()}px;">
      {#each $virtualizer.getVirtualItems() as row (row.key)}
        {@const t = library.tracks[row.index]}
        <button
          class="track-row"
          class:playing={t.id === player.snapshot.track_id}
          style="transform: translateY({row.start}px); height: {ROW_HEIGHT}px;"
          onclick={() => playFrom(row.index)}
        >
          <span class="col-no">
            {#if t.id === player.snapshot.track_id}
              <Play size={12} fill="currentColor" />
            {:else}
              {t.track_no ?? ""}
            {/if}
          </span>
          <span class="col-title">{t.title}</span>
          <span class="col-duration">{formatDuration(t.duration_ms)}</span>
        </button>
      {/each}
    </div>
  </div>
</div>

<style>
  .track-view {
    position: absolute;
    inset: 0;
    display: flex;
    flex-direction: column;
  }

  .header {
    display: flex;
    align-items: center;
    gap: 1em;
    padding: 1.5em;
    border-bottom: 1px solid var(--border);
  }

  .back {
    display: flex;
    align-items: center;
    gap: 0.35em;
    background: none;
    border: none;
    color: var(--text-secondary);
    cursor: pointer;
    align-self: flex-start;
    padding: 0.25em 0.5em;
    margin-right: 0.5em;
  }

  .back:hover {
    color: var(--text-primary);
  }

  .art {
    width: 64px;
    height: 64px;
    border-radius: var(--radius-sm);
    overflow: hidden;
    background: var(--bg-hover);
    flex-shrink: 0;
  }

  .art img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  .art-btn {
    display: block;
    width: 100%;
    height: 100%;
    padding: 0;
    border: none;
    background: none;
    cursor: zoom-in;
  }

  .art-modal {
    position: fixed;
    inset: 0;
    z-index: 200;
    background: rgba(0, 0, 0, 0.85);
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: zoom-out;
  }

  .art-modal-close {
    position: absolute;
    top: 1em;
    right: 1em;
    background: rgba(0, 0, 0, 0.5);
    border: none;
    border-radius: 50%;
    width: 36px;
    height: 36px;
    color: #fff;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .art-modal-close:hover {
    background: rgba(0, 0, 0, 0.8);
  }

  .art-modal-img {
    max-width: 90vw;
    max-height: 90vh;
    object-fit: contain;
    border-radius: var(--radius);
  }

  .art-placeholder {
    width: 100%;
    height: 100%;
    background: linear-gradient(135deg, var(--bg-hover), var(--bg-selected));
  }

  .title {
    font-weight: 600;
  }

  .subtitle {
    color: var(--text-secondary);
    font-size: 0.85em;
  }

  .rating-row {
    margin-top: 0.4em;
  }

  .track-scroll {
    flex: 1;
    overflow-y: auto;
    padding: 0.5em 1.5em;
  }

  .track-inner {
    position: relative;
    width: 100%;
  }

  .track-row {
    position: absolute;
    top: 0;
    left: 0;
    width: 100%;
    display: flex;
    align-items: center;
    gap: 1em;
    background: none;
    border: none;
    color: var(--text-primary);
    cursor: pointer;
    padding: 0 0.75em;
    border-radius: var(--radius-sm);
    text-align: left;
  }

  .track-row:hover {
    background: var(--bg-hover);
  }

  .track-row.playing {
    background: var(--bg-selected);
    color: var(--accent-hover);
  }

  .col-no {
    width: 2em;
    color: var(--text-tertiary);
    text-align: right;
    flex-shrink: 0;
    display: flex;
    align-items: center;
    justify-content: flex-end;
  }

  .col-title {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .col-duration {
    color: var(--text-secondary);
    font-size: 0.85em;
    flex-shrink: 0;
  }
</style>
