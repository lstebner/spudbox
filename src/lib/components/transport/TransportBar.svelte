<script lang="ts">
  import { convertFileSrc } from "@tauri-apps/api/core";
  import { player } from "$lib/stores/player.svelte";
  import { formatDuration } from "$lib/format";

  let seeking = $state(false);
  let seekValue = $state(0);

  function onSeekInput(e: Event) {
    seeking = true;
    seekValue = Number((e.target as HTMLInputElement).value);
  }

  function onSeekCommit(e: Event) {
    const positionMs = Number((e.target as HTMLInputElement).value);
    player.seek(positionMs);
    seeking = false;
  }

  function onVolumeChange(e: Event) {
    player.setVolume(Number((e.target as HTMLInputElement).value));
  }
</script>

<div class="transport">
  <div class="now-playing">
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
    </div>
  </div>

  <div class="controls">
    <button
      class="skip"
      disabled={!player.snapshot.track_id}
      onclick={() => player.previous()}
      aria-label="Previous"
    >
      ⏮
    </button>
    <button
      class="play-pause"
      disabled={!player.snapshot.track_id}
      onclick={() => player.togglePlayPause()}
    >
      {player.snapshot.state === "playing" ? "❙❙" : "▶"}
    </button>
    <button
      class="skip"
      disabled={!player.snapshot.track_id}
      onclick={() => player.next()}
      aria-label="Next"
    >
      ⏭
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
    <input
      type="range"
      min="0"
      max="1"
      step="0.01"
      value={player.snapshot.volume}
      oninput={onVolumeChange}
    />
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
  }

  .art {
    width: 56px;
    height: 56px;
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
    color: var(--text-secondary);
    font-size: 0.8em;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
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
    background: var(--bg-hover);
    color: var(--text-tertiary);
    cursor: default;
  }

  .skip {
    width: 28px;
    height: 28px;
    border: none;
    background: none;
    color: var(--text-secondary);
    cursor: pointer;
    flex-shrink: 0;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .skip:hover:not(:disabled) {
    color: var(--text-primary);
  }

  .skip:disabled {
    color: var(--text-tertiary);
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
    color: var(--text-secondary);
    width: 2.5em;
    text-align: center;
  }

  .volume {
    display: flex;
    justify-content: flex-end;
  }

  .volume input[type="range"] {
    width: 100px;
  }
</style>
