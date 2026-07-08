<script lang="ts">
  import { fade, fly } from "svelte/transition";
  import { convertFileSrc } from "@tauri-apps/api/core";
  import { Play, X } from "@lucide/svelte";
  import { commands } from "$lib/api/commands";
  import { library } from "$lib/stores/library.svelte";
  import { player } from "$lib/stores/player.svelte";
  import { ui } from "$lib/stores/ui.svelte";
  import { formatDuration } from "$lib/format";
  import type { TrackRow } from "$lib/types";
  import StarRating from "$lib/components/rating/StarRating.svelte";

  const DRAWER_WIDTH_PIXELS = 420;
  const TRANSITION_DURATION_MILLISECONDS = 200;

  let tracks = $state<TrackRow[]>([]);
  let artModalOpen = $state(false);

  const album = $derived(
    library.allAlbums.find((a) => a.id === player.snapshot.album_id) ?? null,
  );

  const totalDurationMs = $derived(tracks.reduce((sum, t) => sum + t.duration_ms, 0));

  $effect(() => {
    const albumId = player.snapshot.album_id;
    if (albumId === null) {
      tracks = [];
      return;
    }
    commands.libraryGetTracksByAlbum(albumId).then((newTracks) => {
      tracks = newTracks;
    });
  });

  function handleKeydown(e: KeyboardEvent) {
    if (e.key !== "Escape") return;
    if (artModalOpen) {
      artModalOpen = false;
    } else {
      ui.closeNowPlayingDrawer();
    }
  }

  function playFrom(index: number) {
    player.playQueue(
      tracks.map((t) => t.id),
      index,
    );
  }
</script>

<svelte:window onkeydown={ui.nowPlayingDrawerOpen ? handleKeydown : undefined} />

{#if ui.nowPlayingDrawerOpen}
  <div
    class="backdrop"
    role="presentation"
    transition:fade={{ duration: TRANSITION_DURATION_MILLISECONDS }}
    onclick={() => ui.closeNowPlayingDrawer()}
  ></div>
  <div
    class="drawer"
    role="dialog"
    aria-modal="true"
    aria-label="Now playing"
    transition:fly={{ x: DRAWER_WIDTH_PIXELS, duration: TRANSITION_DURATION_MILLISECONDS }}
  >
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
          onclick={(e) => {
            e.stopPropagation();
            artModalOpen = false;
          }}
          aria-label="Close"
        >
          <X size={20} />
        </button>
        <img src={convertFileSrc(album.art_path)} alt={album.title} class="art-modal-img" />
      </div>
    {/if}

    <div class="header">
      {#if album}
        <div class="art">
          {#if album.art_path}
            <button
              class="art-btn"
              onclick={() => (artModalOpen = true)}
              aria-label="View full artwork"
            >
              <img src={convertFileSrc(album.art_path)} alt={album.title} />
            </button>
          {:else}
            <div class="art-placeholder"></div>
          {/if}
        </div>
        <div class="text">
          <div class="title">{album.title}</div>
          <div class="subtitle">
            {album.album_artist}{album.year ? ` · ${album.year}` : ""}{totalDurationMs > 0
              ? ` · ${formatDuration(totalDurationMs)}`
              : ""}
          </div>
          <div class="rating-row">
            <StarRating rating={album.rating} size={16} onRate={(r) => library.setAlbumRating(album.id, r)} />
          </div>
        </div>
      {/if}
      <button class="close" onclick={() => ui.closeNowPlayingDrawer()} aria-label="Close now playing panel">
        <X size={18} />
      </button>
    </div>

    <div class="track-scroll">
      {#each tracks as t, index (t.id)}
        <button class="track-row" class:playing={t.id === player.snapshot.track_id} onclick={() => playFrom(index)}>
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
{/if}

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 150;
    background: rgba(0, 0, 0, 0.4);
  }

  .drawer {
    position: fixed;
    top: 0;
    right: 0;
    bottom: 0;
    width: 420px;
    z-index: 151;
    display: flex;
    flex-direction: column;
    background: var(--bg-elevated);
    border-left: 1px solid var(--border);
  }

  .header {
    display: flex;
    align-items: flex-start;
    gap: 1em;
    padding: 1.5em;
    border-bottom: 1px solid var(--border);
  }

  .text {
    min-width: 0;
    flex: 1;
  }

  .close {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 28px;
    height: 28px;
    flex-shrink: 0;
    background: none;
    border: none;
    color: var(--text-secondary);
    cursor: pointer;
    border-radius: var(--radius-sm);
  }

  .close:hover {
    color: var(--text-primary);
    background: var(--bg-hover);
  }

  .art {
    width: 80px;
    height: 80px;
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
    font-weight: 500;
    font-size: 1.1em;
    overflow-wrap: break-word;
  }

  .subtitle {
    color: var(--text-secondary);
    font-size: 1em;
  }

  .rating-row {
    margin-top: 0.4em;
  }

  .track-scroll {
    flex: 1;
    overflow-y: auto;
    padding: 0.5em 1em;
  }

  .track-row {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 1em;
    background: none;
    border: none;
    color: var(--text-primary);
    cursor: pointer;
    padding: 0.5em 0.75em;
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
