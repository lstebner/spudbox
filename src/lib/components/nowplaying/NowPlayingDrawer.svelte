<script lang="ts">
  import { fade, fly } from "svelte/transition";
  import { X } from "@lucide/svelte";
  import { commands } from "$lib/api/commands";
  import { library } from "$lib/stores/library.svelte";
  import { player } from "$lib/stores/player.svelte";
  import { ui } from "$lib/stores/ui.svelte";
  import type { TrackRow as TrackRowData } from "$lib/types";
  import AlbumHeader from "$lib/components/album/AlbumHeader.svelte";
  import TrackRow from "$lib/components/album/TrackRow.svelte";

  const DRAWER_WIDTH_PIXELS = 420;
  const TRANSITION_DURATION_MILLISECONDS = 200;

  const FOCUSABLE_SELECTOR =
    'button:not([disabled]), [href], input, select, textarea, [tabindex]:not([tabindex="-1"])';

  let tracks = $state<TrackRowData[]>([]);
  let artModalOpen = $state(false);
  let drawerEl: HTMLDivElement | undefined = $state();
  let closeButtonEl: HTMLButtonElement | undefined = $state();
  let previouslyFocusedElement: HTMLElement | null = null;

  const album = $derived(
    player.snapshot.album_id !== null ? library.findAlbumById(player.snapshot.album_id) : null,
  );

  const totalDurationMs = $derived(tracks.reduce((sum, t) => sum + t.duration_ms, 0));

  // Gated on nowPlayingDrawerOpen so normal playback (album/track changes
  // while the drawer is closed) never fires this IPC call in the
  // background — only re-fetches while the drawer is actually visible.
  $effect(() => {
    if (!ui.nowPlayingDrawerOpen) return;
    const albumId = player.snapshot.album_id;
    if (albumId === null) {
      tracks = [];
      return;
    }
    // Guards against out-of-order resolution if album_id changes again
    // before this fetch completes (e.g. rapid skips across an album
    // boundary) — an older in-flight fetch must not clobber newer tracks.
    commands.libraryGetTracksByAlbum(albumId).then((newTracks) => {
      if (player.snapshot.album_id === albumId) tracks = newTracks;
    });
  });

  $effect(() => {
    if (ui.nowPlayingDrawerOpen) {
      previouslyFocusedElement = document.activeElement as HTMLElement | null;
      closeButtonEl?.focus();
    } else {
      previouslyFocusedElement?.focus();
      previouslyFocusedElement = null;
    }
  });

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "Escape") {
      if (artModalOpen) {
        artModalOpen = false;
      } else {
        ui.closeNowPlayingDrawer();
      }
      return;
    }
    if (e.key === "Tab") trapFocus(e);
  }

  function trapFocus(e: KeyboardEvent) {
    if (!drawerEl) return;
    const focusables = drawerEl.querySelectorAll<HTMLElement>(FOCUSABLE_SELECTOR);
    if (focusables.length === 0) return;
    const first = focusables[0];
    const last = focusables[focusables.length - 1];
    if (e.shiftKey && document.activeElement === first) {
      e.preventDefault();
      last.focus();
    } else if (!e.shiftKey && document.activeElement === last) {
      e.preventDefault();
      first.focus();
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
    bind:this={drawerEl}
    transition:fly={{ x: DRAWER_WIDTH_PIXELS, duration: TRANSITION_DURATION_MILLISECONDS }}
  >
    <AlbumHeader
      {album}
      {totalDurationMs}
      onRate={(r) => {
        if (album) library.setAlbumRating(album.id, r);
      }}
      bind:artModalOpen
    >
      {#snippet trailing()}
        <button
          class="close"
          bind:this={closeButtonEl}
          onclick={() => ui.closeNowPlayingDrawer()}
          aria-label="Close now playing panel"
        >
          <X size={18} />
        </button>
      {/snippet}
    </AlbumHeader>

    <div class="track-scroll">
      {#each tracks as t, index (t.id)}
        <TrackRow track={t} isPlaying={t.id === player.snapshot.track_id} onclick={() => playFrom(index)} />
      {/each}
    </div>
  </div>
{/if}

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 150;
    background: var(--scrim-weak);
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

  .track-scroll {
    flex: 1;
    overflow-y: auto;
    padding: 0.5em 1em;
  }
</style>
