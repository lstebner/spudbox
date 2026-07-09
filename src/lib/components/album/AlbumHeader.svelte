<script lang="ts">
  import type { Snippet } from "svelte";
  import { convertFileSrc } from "@tauri-apps/api/core";
  import { X } from "@lucide/svelte";
  import type { AlbumRow } from "$lib/types";
  import { formatDuration } from "$lib/format";
  import StarRating from "$lib/components/rating/StarRating.svelte";

  let {
    album,
    totalDurationMs,
    onRate,
    artModalOpen = $bindable(false),
    leading,
    trailing,
  }: {
    album: AlbumRow | null;
    totalDurationMs: number;
    onRate: (rating: number | null) => void;
    artModalOpen?: boolean;
    leading?: Snippet;
    trailing?: Snippet;
  } = $props();
</script>

{#if artModalOpen && album?.art_path}
  <div
    class="art-modal"
    role="dialog"
    aria-modal="true"
    tabindex="-1"
    onclick={() => (artModalOpen = false)}
    onkeydown={(e) => {
      if (e.key === "Escape") artModalOpen = false;
    }}
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
  {@render leading?.()}
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
        <StarRating rating={album.rating} size={16} onRate={(r) => onRate(r)} />
      </div>
    </div>
  {/if}
  {@render trailing?.()}
</div>

<style>
  .header {
    display: flex;
    align-items: center;
    gap: 1em;
    padding: 1.5em;
    border-bottom: 1px solid var(--border);
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
    background: var(--scrim-heavy);
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: zoom-out;
  }

  .art-modal-close {
    position: absolute;
    top: 1em;
    right: 1em;
    background: var(--scrim-medium);
    border: none;
    border-radius: 50%;
    width: 36px;
    height: 36px;
    color: var(--on-scrim);
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .art-modal-close:hover {
    background: var(--scrim-strong);
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

  .text {
    min-width: 0;
    flex: 1;
  }

  .title {
    font-weight: 500;
    font-size: 1em;
    overflow-wrap: break-word;
  }

  .subtitle {
    color: var(--text-secondary);
    font-size: 1em;
  }

  .rating-row {
    margin-top: 0.4em;
  }
</style>
