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
    background: var(--row-hover-bg);
    color: var(--row-hover-title-ink);
  }

  .track-row.playing {
    background: var(--row-active-bg);
    color: var(--row-active-title-ink);
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

  /* --row-hover-num-ink / --row-active-num-ink default to the same
   * text-tertiary/text-secondary values .col-no/.col-duration already use
   * at rest — a no-op for every theme except lemon, which overrides them
   * to stay legible against its flipped dark hover/selected background.
   * See "The lemon hover flip" in docs/DESIGN_SYSTEM.md. */
  .track-row:hover .col-no {
    color: var(--row-hover-num-ink);
  }

  .track-row.playing .col-no {
    color: var(--row-active-num-ink);
  }

  .col-title {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .col-duration {
    color: var(--text-secondary);
    flex-shrink: 0;
  }

  .track-row:hover .col-duration {
    color: var(--row-hover-dur-ink);
  }

  .track-row.playing .col-duration {
    color: var(--row-active-dur-ink);
  }
</style>
