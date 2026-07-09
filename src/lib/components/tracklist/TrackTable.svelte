<script lang="ts">
  import { createVirtualizer } from "@tanstack/svelte-virtual";
  import { ArrowLeft } from "@lucide/svelte";
  import { get } from "svelte/store";
  import { library } from "$lib/stores/library.svelte";
  import { player } from "$lib/stores/player.svelte";
  import AlbumHeader from "$lib/components/album/AlbumHeader.svelte";
  import TrackRow from "$lib/components/album/TrackRow.svelte";

  const ROW_HEIGHT = 36;

  let scrollEl: HTMLDivElement | undefined = $state();

  const album = $derived(library.albums.find((a) => a.id === library.selectedAlbumId) ?? null);

  const totalDurationMs = $derived(library.tracks.reduce((sum, t) => sum + t.duration_ms, 0));

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

<div class="track-view">
  <AlbumHeader {album} {totalDurationMs} onRate={(r) => { if (album) library.setAlbumRating(album.id, r); }} bind:artModalOpen>
    {#snippet leading()}
      <button class="back" onclick={() => library.backToAlbums()}>
        <ArrowLeft size={16} />
        Albums
      </button>
    {/snippet}
  </AlbumHeader>

  <div bind:this={scrollEl} class="track-scroll">
    <div class="track-inner" style="height: {$virtualizer.getTotalSize()}px;">
      {#each $virtualizer.getVirtualItems() as row (row.key)}
        {@const t = library.tracks[row.index]}
        <TrackRow
          track={t}
          isPlaying={t.id === player.snapshot.track_id}
          onclick={() => playFrom(row.index)}
          style="position: absolute; top: 0; left: 0; width: 100%; transform: translateY({row.start}px); height: {ROW_HEIGHT}px;"
        />
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

  .track-scroll {
    flex: 1;
    overflow-y: auto;
    padding: 0.5em 1.5em;
  }

  .track-inner {
    position: relative;
    width: 100%;
  }
</style>
