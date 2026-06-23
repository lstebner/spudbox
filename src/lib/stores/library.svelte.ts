import { commands } from "$lib/api/commands";
import type { AlbumRow, ArtistRow, TrackRow } from "$lib/types";

function createLibraryStore() {
  let artists = $state<ArtistRow[]>([]);
  let albums = $state<AlbumRow[]>([]);
  let tracks = $state<TrackRow[]>([]);
  let selectedArtistId = $state<number | null>(null);
  let selectedAlbumId = $state<number | null>(null);
  let loading = $state(false);

  async function loadAlbums() {
    albums = await commands.libraryGetAlbums(selectedArtistId);
  }

  async function selectArtist(artistId: number | null) {
    selectedArtistId = artistId;
    selectedAlbumId = null;
    await loadAlbums();
  }

  async function selectAlbum(albumId: number) {
    selectedAlbumId = albumId;
    tracks = await commands.libraryGetTracksByAlbum(albumId);
  }

  function backToAlbums() {
    selectedAlbumId = null;
  }

  async function refresh() {
    loading = true;
    try {
      artists = await commands.libraryGetArtists();
      await loadAlbums();
    } finally {
      loading = false;
    }
  }

  async function rescan() {
    loading = true;
    try {
      await commands.libraryScan();
      await refresh();
    } finally {
      loading = false;
    }
  }

  return {
    get artists() {
      return artists;
    },
    get albums() {
      return albums;
    },
    get tracks() {
      return tracks;
    },
    get selectedArtistId() {
      return selectedArtistId;
    },
    get selectedAlbumId() {
      return selectedAlbumId;
    },
    get loading() {
      return loading;
    },
    selectArtist,
    selectAlbum,
    backToAlbums,
    refresh,
    rescan,
  };
}

export const library = createLibraryStore();
