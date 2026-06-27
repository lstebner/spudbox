<script lang="ts">
  import { ChevronDown, ChevronRight } from "@lucide/svelte";
  import { library } from "$lib/stores/library.svelte";
  import type { AlbumRow } from "$lib/types";

  let query = $state("");
  let manuallyExpanded = $state(new Set<number>());

  function toggleExpanded(artistId: number) {
    const next = new Set(manuallyExpanded);
    if (next.has(artistId)) {
      next.delete(artistId);
    } else {
      next.add(artistId);
    }
    manuallyExpanded = next;
  }

  // Strips everything but letters/numbers (so punctuation, apostrophes —
  // straight or curly — ellipses, etc. can't cause a mismatch) and
  // lowercases, applied to both the query and the text being searched.
  function normalize(text: string): string {
    return text.toLowerCase().replace(/[^\p{L}\p{N}]+/gu, "");
  }

  const searching = $derived(query.trim().length > 0);
  const normalizedQuery = $derived(normalize(query));

  const albumsByArtist = $derived.by(() => {
    const map = new Map<number, AlbumRow[]>();
    for (const album of library.allAlbums) {
      const list = map.get(album.album_artist_id);
      if (list) {
        list.push(album);
      } else {
        map.set(album.album_artist_id, [album]);
      }
    }
    return map;
  });

  // While searching, an artist is visible if its own name matches or any
  // of its albums do; outside search, every artist is shown (collapsed by
  // default, toggled manually via the caret).
  const visibleArtists = $derived.by(() => {
    if (!searching) return library.artists;
    return library.artists.filter((artist) => {
      if (normalize(artist.name).includes(normalizedQuery)) return true;
      const albums = albumsByArtist.get(artist.id) ?? [];
      return albums.some((album) => normalize(album.title).includes(normalizedQuery));
    });
  });

  function isExpanded(artistId: number): boolean {
    return searching || manuallyExpanded.has(artistId);
  }

  // When the artist itself matched by name, show all its albums; when it
  // only matched because one of its albums did, show just the matches.
  function visibleAlbumsFor(artistId: number, artistName: string): AlbumRow[] {
    const albums = albumsByArtist.get(artistId) ?? [];
    if (!searching || normalize(artistName).includes(normalizedQuery)) {
      return albums;
    }
    return albums.filter((album) => normalize(album.title).includes(normalizedQuery));
  }
</script>

<div class="search">
  <input type="text" placeholder="Search artists or albums…" bind:value={query} />
</div>

<nav class="artist-list">
  <button
    class="artist-item"
    class:active={library.selectedArtistId === null}
    onclick={() => library.selectArtist(null)}
  >
    All Albums
  </button>
  {#each visibleArtists as artist (artist.id)}
    <div class="artist-group">
      <div class="artist-row">
        <button
          class="caret"
          onclick={() => toggleExpanded(artist.id)}
          aria-label={isExpanded(artist.id) ? "Collapse albums" : "Expand albums"}
        >
          {#if isExpanded(artist.id)}
            <ChevronDown size={14} />
          {:else}
            <ChevronRight size={14} />
          {/if}
        </button>
        <button
          class="artist-item"
          class:active={library.selectedArtistId === artist.id && library.selectedAlbumId === null}
          onclick={() => library.selectArtist(artist.id)}
        >
          <span class="name">{artist.name}</span>
          <span class="count">{artist.album_count}</span>
        </button>
      </div>
      {#if isExpanded(artist.id)}
        <div class="album-sublist">
          {#each visibleAlbumsFor(artist.id, artist.name) as album (album.id)}
            <button
              class="album-item"
              class:active={library.selectedAlbumId === album.id}
              onclick={() => library.selectArtistAndAlbum(artist.id, album.id)}
            >
              {album.title}
            </button>
          {/each}
        </div>
      {/if}
    </div>
  {/each}
</nav>


<style>
  .search {
    position: sticky;
    top: 0;
    z-index: 1;
    padding: 0.75em 0.5em 0.25em;
    background: var(--bg-elevated);
  }

  .search input {
    width: 100%;
    background: var(--bg-hover);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    color: var(--text-primary);
    padding: 0.4em 0.6em;
  }

  .search input::placeholder {
    color: var(--text-tertiary);
  }

  .artist-list {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 0.25em 0.5em 0.75em;
  }

  .artist-row {
    display: flex;
    align-items: center;
  }

  .caret {
    display: flex;
    align-items: center;
    justify-content: center;
    background: none;
    border: none;
    color: var(--text-tertiary);
    cursor: pointer;
    width: 1.5em;
    flex-shrink: 0;
    padding: 0.5em 0;
  }

  .caret:hover {
    color: var(--text-primary);
  }

  .artist-item {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 0.5em;
    width: 100%;
    text-align: left;
    background: none;
    border: none;
    padding: 0.5em 0.75em;
    border-radius: var(--radius);
    cursor: pointer;
    color: var(--text-secondary);
  }

  .artist-item:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
  }

  .artist-item.active {
    background: var(--bg-selected);
    color: var(--text-primary);
  }

  .name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .count {
    color: var(--text-tertiary);
    font-size: 0.85em;
    flex-shrink: 0;
  }

  .album-sublist {
    display: flex;
    flex-direction: column;
    gap: 1px;
    padding-left: 1.5em;
  }

  .album-item {
    width: 100%;
    text-align: left;
    background: none;
    border: none;
    padding: 0.35em 0.75em;
    border-radius: var(--radius);
    cursor: pointer;
    color: var(--text-tertiary);
    font-size: 0.9em;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .album-item:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
  }

  .album-item.active {
    background: var(--bg-selected);
    color: var(--text-primary);
  }

</style>
