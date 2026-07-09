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
  let hiddenAlbums = $state<AlbumRow[]>([]);
  let tracks = $state<TrackRow[]>([]);
  let selectedArtistId = $state<number | null>(null);
  let selectedAlbumId = $state<number | null>(null);
  let isViewingHidden = $state(false);
  let loading = $state(false);
  let hasRoots = $state(true);

  async function loadAlbums() {
    if (isViewingHidden) {
      albums = await commands.libraryGetAlbums(null, true);
    } else {
      albums = await commands.libraryGetAlbums(selectedArtistId);
    }
  }

  async function loadAllAlbums() {
    allAlbums = await commands.libraryGetAlbums(null);
  }

  async function loadHiddenAlbums() {
    hiddenAlbums = await commands.libraryGetAlbums(null, true);
  }

  async function selectArtist(artistId: number | null) {
    isViewingHidden = false;
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
    isViewingHidden = false;
    selectedArtistId = artistId;
    await loadAlbums();
    await selectAlbum(albumId);
  }

  // Checks `hiddenAlbums` too, unlike a plain `allAlbums.find(...)` — needed
  // anywhere an album is looked up by id outside of the main browsing flow
  // (e.g. the now-playing bar/drawer), since playback can point at an
  // album the user has since hidden, and `allAlbums` excludes those.
  function findAlbumById(albumId: number): AlbumRow | null {
    return (
      allAlbums.find((a) => a.id === albumId) ??
      hiddenAlbums.find((a) => a.id === albumId) ??
      null
    );
  }

  async function selectHidden() {
    isViewingHidden = true;
    selectedArtistId = null;
    selectedAlbumId = null;
    albums = await commands.libraryGetAlbums(null, true);
  }

  async function setAlbumHidden(albumId: number, hidden: boolean) {
    await commands.librarySetAlbumHidden(albumId, hidden);
    if (selectedAlbumId === albumId) selectedAlbumId = null;
    await refresh();
  }

  async function setAlbumRating(albumId: number, rating: number | null) {
    await commands.librarySetAlbumRating(albumId, rating);
    const patch = (list: AlbumRow[]) =>
      list.map((a) => (a.id === albumId ? { ...a, rating } : a));
    albums = patch(albums);
    allAlbums = patch(allAlbums);
    hiddenAlbums = patch(hiddenAlbums);
  }

  function markAlbumPlayed(albumId: number) {
    const clear = (list: AlbumRow[]) =>
      list.map((a) => (a.id === albumId ? { ...a, is_new: false } : a));
    albums = clear(albums);
    allAlbums = clear(allAlbums);
    hiddenAlbums = clear(hiddenAlbums);
  }

  async function refresh() {
    loading = true;
    try {
      hasRoots = await commands.libraryHasRoots();
      if (!hasRoots) {
        await commands.playbackStop();
      }
      artists = await commands.libraryGetArtists();
      await loadAlbums();
      await loadAllAlbums();
      await loadHiddenAlbums();
      // Clear the selected album only if it no longer exists in any list.
      if (
        selectedAlbumId !== null &&
        !allAlbums.some((a) => a.id === selectedAlbumId) &&
        !hiddenAlbums.some((a) => a.id === selectedAlbumId)
      ) {
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
    get hiddenAlbums() {
      return hiddenAlbums;
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
    get isViewingHidden() {
      return isViewingHidden;
    },
    get loading() {
      return loading;
    },
    get hasRoots() {
      return hasRoots;
    },
    findAlbumById,
    selectArtist,
    selectAlbum,
    selectArtistAndAlbum,
    selectHidden,
    backToAlbums,
    setAlbumRating,
    setAlbumHidden,
    markAlbumPlayed,
    refresh,
    rescan,
    addFolder,
  };
}

export const library = createLibraryStore();
