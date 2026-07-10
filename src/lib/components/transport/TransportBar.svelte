<script lang="ts">
  import { convertFileSrc } from "@tauri-apps/api/core";
  import { Pause, Play, SkipBack, SkipForward, Volume2 } from "@lucide/svelte";
  import { library } from "$lib/stores/library.svelte";
  import { player } from "$lib/stores/player.svelte";
  import { ui } from "$lib/stores/ui.svelte";
  import { formatDuration } from "$lib/format";
  import StarRating from "$lib/components/rating/StarRating.svelte";
  import Equalizer from "$lib/components/transport/Equalizer.svelte";

  // Looked up via findAlbumById (checks hidden albums too, not just the
  // artist-filtered `albums`) since playback can be on an album outside
  // whatever's currently browsed, or one the user has since hidden — same
  // reasoning the now-playing drawer uses.
  const currentAlbum = $derived(
    player.snapshot.album_id !== null ? library.findAlbumById(player.snapshot.album_id) : null,
  );

  function openNowPlayingDrawer() {
    if (player.snapshot.album_id !== null) {
      ui.openNowPlayingDrawer();
    }
  }

  let seeking = $state(false);
  let seekValue = $state(0);

  // Seeking is dispatched fire-and-forget over Tauri IPC, and the backend's
  // position only reaches the frontend via the ~4Hz playback-progress
  // event — so the target isn't confirmed the instant player.seek()
  // returns. Clearing `seeking` immediately on commit raced that event:
  // the bar would flash back to the stale pre-seek position before
  // snapping forward once the real update arrived. Instead, keep showing
  // the optimistic value until an incoming snapshot's position lands
  // within tolerance of the target (or the fallback timeout gives up, in
  // case the seek silently failed on the backend).
  let pendingSeekTargetMs = $state<number | null>(null);
  let seekFallbackTimer: ReturnType<typeof setTimeout> | undefined;
  const SEEK_CONFIRM_TOLERANCE_MS = 750;
  const SEEK_CONFIRM_FALLBACK_MS = 2000;

  function clearPendingSeek() {
    seeking = false;
    pendingSeekTargetMs = null;
    clearTimeout(seekFallbackTimer);
  }

  function onSeekInput(e: Event) {
    seeking = true;
    pendingSeekTargetMs = null;
    clearTimeout(seekFallbackTimer);
    seekValue = Number((e.target as HTMLInputElement).value);
  }

  function onSeekCommit(e: Event) {
    const positionMs = Number((e.target as HTMLInputElement).value);
    seekValue = positionMs;
    pendingSeekTargetMs = positionMs;
    player.seek(positionMs);
    clearTimeout(seekFallbackTimer);
    seekFallbackTimer = setTimeout(clearPendingSeek, SEEK_CONFIRM_FALLBACK_MS);
  }

  $effect(() => {
    if (pendingSeekTargetMs === null) return;
    if (Math.abs(player.snapshot.position_ms - pendingSeekTargetMs) <= SEEK_CONFIRM_TOLERANCE_MS) {
      clearPendingSeek();
    }
  });

  function onVolumeChange(e: Event) {
    player.setVolume(Number((e.target as HTMLInputElement).value));
  }
</script>

<div class="transport">
  <button
    class="now-playing"
    onclick={openNowPlayingDrawer}
    disabled={player.snapshot.album_id === null}
    aria-label="Show now playing"
  >
    <div class="art">
      {#if player.snapshot.art_path}
        <img src={convertFileSrc(player.snapshot.art_path)} alt={player.snapshot.album ?? ""} />
      {:else}
        <div class="art-placeholder"></div>
      {/if}
    </div>
    <div class="text">
      <div class="title">{player.snapshot.title ?? "Nothing playing"}</div>
      <div class="artist">{player.snapshot.artist ?? ""}</div>
      {#if currentAlbum}
        <div class="rating-row">
          <StarRating rating={currentAlbum.rating} readonly size={11} />
        </div>
      {/if}
    </div>
  </button>

  <div class="controls">
    <button
      class="skip"
      disabled={!player.snapshot.track_id}
      onclick={() => player.previous()}
      aria-label="Previous"
    >
      <SkipBack size={18} fill="currentColor" />
    </button>
    <button
      class="play-pause"
      disabled={!player.snapshot.track_id}
      onclick={() => player.togglePlayPause()}
    >
      {#if player.snapshot.state === "playing"}
        <Pause size={18} fill="currentColor" />
      {:else}
        <Play size={18} fill="currentColor" />
      {/if}
    </button>
    <button
      class="skip"
      disabled={!player.snapshot.track_id}
      onclick={() => player.next()}
      aria-label="Next"
    >
      <SkipForward size={18} fill="currentColor" />
    </button>
    <div class="scrubber">
      <span class="time">{formatDuration(seeking ? seekValue : player.snapshot.position_ms)}</span>
      <input
        type="range"
        min="0"
        max={player.snapshot.duration_ms || 1}
        value={seeking ? seekValue : player.snapshot.position_ms}
        oninput={onSeekInput}
        onchange={onSeekCommit}
      />
      <span class="time">{formatDuration(player.snapshot.duration_ms)}</span>
    </div>
  </div>

  <div class="volume">
    <Volume2 size={16} />
    <input
      type="range"
      min="0"
      max="1"
      step="0.01"
      value={player.snapshot.volume}
      oninput={onVolumeChange}
    />
    <Equalizer />
  </div>
</div>

<style>
  .transport {
    display: grid;
    grid-template-columns: 1fr 2fr 1fr;
    align-items: center;
    height: 100%;
    padding: 0 1.5em;
    gap: 1.5em;
  }

  .now-playing {
    display: flex;
    align-items: center;
    gap: 0.75em;
    min-width: 0;
    background: none;
    border: none;
    padding: 0.25em;
    border-radius: var(--radius);
    text-align: left;
    color: inherit;
    font: inherit;
    cursor: pointer;
  }

  .now-playing:hover:not(:disabled) {
    background: var(--chrome-hover-bg);
  }

  .now-playing:disabled {
    cursor: default;
  }

  .art {
    width: 56px;
    height: 56px;
    border-radius: var(--radius-sm);
    overflow: hidden;
    background: var(--chrome-hover-bg);
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
    background: linear-gradient(135deg, var(--chrome-hover-bg), var(--chrome-selected-bg));
  }

  .text {
    overflow: hidden;
  }

  .title {
    font-weight: 600;
    font-size: 0.9em;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .artist {
    color: var(--chrome-text-secondary);
    font-size: 0.8em;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .rating-row {
    margin-top: 0.3em;
  }

  .controls {
    display: flex;
    align-items: center;
    gap: 1em;
  }

  .play-pause {
    width: 36px;
    height: 36px;
    border-radius: 50%;
    border: none;
    background: var(--accent);
    color: var(--bg-base);
    cursor: pointer;
    flex-shrink: 0;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .play-pause:hover {
    background: var(--accent-hover);
  }

  .play-pause:disabled {
    background: var(--chrome-hover-bg);
    color: var(--chrome-text-tertiary);
    cursor: default;
  }

  .skip {
    width: 28px;
    height: 28px;
    border: none;
    background: none;
    color: var(--chrome-text-secondary);
    cursor: pointer;
    flex-shrink: 0;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .skip:hover:not(:disabled) {
    color: var(--chrome-text-primary);
  }

  .skip:disabled {
    color: var(--chrome-text-tertiary);
    cursor: default;
  }

  .scrubber {
    flex: 1;
    display: flex;
    align-items: center;
    gap: 0.5em;
  }

  .scrubber input[type="range"] {
    flex: 1;
  }

  .time {
    font-size: 0.75em;
    color: var(--chrome-text-secondary);
    width: 2.5em;
    text-align: center;
  }

  .volume {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 0.5em;
    color: var(--chrome-text-secondary);
  }

  .volume input[type="range"] {
    width: 100px;
  }
</style>
