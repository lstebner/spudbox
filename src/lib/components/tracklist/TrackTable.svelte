<script lang="ts">
  import { createVirtualizer } from "@tanstack/svelte-virtual";
  import { convertFileSrc } from "@tauri-apps/api/core";
  import { get } from "svelte/store";
  import { library } from "$lib/stores/library.svelte";
  import { player } from "$lib/stores/player.svelte";
  import { formatDuration } from "$lib/format";

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
</script>

<div class="track-view">
  <div class="header">
    <button class="back" onclick={() => library.backToAlbums()}>&larr; Albums</button>
    {#if album}
      <div class="art">
        {#if album.art_path}
          <img src={convertFileSrc(album.art_path)} alt={album.title} />
        {:else}
          <div class="art-placeholder"></div>
        {/if}
      </div>
      <div>
        <div class="title">{album.title}</div>
        <div class="subtitle">{album.album_artist}{album.year ? ` · ${album.year}` : ""}</div>
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
          onclick={() => player.playTrack(t.id)}
        >
          <span class="col-no">{t.track_no ?? ""}</span>
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
