<script lang="ts">
  import { Play } from "@lucide/svelte";
  import type { TrackRow as TrackRowData } from "$lib/types";
  import { formatDuration } from "$lib/format";

  let {
    track,
    isPlaying,
    onclick,
    style = "",
  }: {
    track: TrackRowData;
    isPlaying: boolean;
    onclick: () => void;
    style?: string;
  } = $props();
</script>

<button class="track-row" class:playing={isPlaying} {style} {onclick}>
  <span class="col-no">
    {#if isPlaying}
      <Play size={12} fill="currentColor" />
    {:else}
      {track.track_no ?? ""}
    {/if}
  </span>
  <span class="col-title" title={track.title}>{track.title}</span>
  <span class="col-duration">{formatDuration(track.duration_ms)}</span>
</button>

<style>
  .track-row {
    display: flex;
    align-items: center;
    gap: 1em;
    width: 100%;
    min-height: 36px;
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
