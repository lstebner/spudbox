import { commands } from "$lib/api/commands";
import type { AlbumRow, ArtistRow, TrackRow } from "$lib/types";

function createLibraryStore() {
  let artists = $state<ArtistRow[]>([]);
  let albums = $state<AlbumRow[]>([]);
  // Always the complete, unfiltered album list (unlike `albums`, which is
  // scoped to whatever artist is currently selected for the main grid) so
  // the sidebar can group/search across every artist's albums regardless
  // of what's currently browsed in the main view.
  let allAlbums = $state<AlbumRow[]>([]);
  let tracks = $state<TrackRow[]>([]);
  let selectedArtistId = $state<number | null>(null);
  let selectedAlbumId = $state<number | null>(null);
  let loading = $state(false);
  let hasRoots = $state(true);

  async function loadAlbums() {
    albums = await commands.libraryGetAlbums(selectedArtistId);
  }

  async function loadAllAlbums() {
    allAlbums = await commands.libraryGetAlbums(null);
  }

  async function selectArtist(artistId: number | null) {
    selectedArtistId = artistId;
    selectedAlbumId = null;
    await loadAlbums();
  }

  async function selectAlbum(albumId: number) {
    // Fetch first, then assign both together: assigning selectedAlbumId
    // alone would immediately remount the keyed track-list view (see
    // +page.svelte) while `tracks` still held the previous album's rows,
    // so the freshly-mounted virtualizer would briefly size itself off
    // stale data.
    const newTracks = await commands.libraryGetTracksByAlbum(albumId);
    selectedAlbumId = albumId;
    tracks = newTracks;
  }

  function backToAlbums() {
    selectedAlbumId = null;
  }

  // Like selectAlbum, but also sets the artist filter first so "back to
  // albums" lands on that artist's albums rather than resetting to "All
  // Albums" — used when navigating into an album from its artist's
  // expanded sublist in the sidebar, where that context is already known.
  async function selectArtistAndAlbum(artistId: number, albumId: number) {
    selectedArtistId = artistId;
    await loadAlbums();
    await selectAlbum(albumId);
  }

  // Navigates to an album's track list regardless of the currently
  // selected artist filter (e.g. from the now-playing bar, which can
  // point at an album outside whatever's currently browsed) by resetting
  // to "All Albums" first so the target album is guaranteed to be in the
  // loaded list.
  async function goToAlbum(albumId: number) {
    selectedArtistId = null;
    await loadAlbums();
    await selectAlbum(albumId);
  }

  async function setAlbumRating(albumId: number, rating: number | null) {
    await commands.librarySetAlbumRating(albumId, rating);
    const patch = (list: AlbumRow[]) =>
      list.map((a) => (a.id === albumId ? { ...a, rating } : a));
    albums = patch(albums);
    allAlbums = patch(allAlbums);
  }

  async function refresh() {
    loading = true;
    try {
      hasRoots = await commands.libraryHasRoots();
      artists = await commands.libraryGetArtists();
      await loadAlbums();
      await loadAllAlbums();
      // If the selected album was just removed, clear back to the grid
      if (selectedAlbumId !== null && !allAlbums.some((a) => a.id === selectedAlbumId)) {
        selectedAlbumId = null;
      }
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

  async function addFolder(path: string) {
    loading = true;
    try {
      await commands.libraryAddRoot(path);
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
    get allAlbums() {
      return allAlbums;
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
    get hasRoots() {
      return hasRoots;
    },
    selectArtist,
    selectAlbum,
    selectArtistAndAlbum,
    goToAlbum,
    backToAlbums,
    setAlbumRating,
    refresh,
    rescan,
    addFolder,
  };
}

export const library = createLibraryStore();
