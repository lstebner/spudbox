import type { AlbumRow } from "$lib/types";
import type { AlbumSort } from "$lib/stores/ui.svelte";

export function sortAlbums(albums: AlbumRow[], sort: AlbumSort): AlbumRow[] {
  if (sort === "artist_name") {
    return [...albums].sort((a, b) => {
      const artistComparison = a.album_artist.localeCompare(b.album_artist);
      if (artistComparison !== 0) return artistComparison;
      return a.title.localeCompare(b.title);
    });
  }
  if (sort === "album_name") {
    return [...albums].sort((a, b) => a.title.localeCompare(b.title));
  }
  return [...albums].sort((a, b) => (b.date_added ?? 0) - (a.date_added ?? 0));
}
