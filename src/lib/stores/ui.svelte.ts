export type AlbumSort = 'date_added' | 'artist_name' | 'album_name';

function createUiStore() {
  let showSettings = $state(false);
  let showDeviceSync = $state(false);
  let showVisualizer = $state(false);
  let albumSort = $state<AlbumSort>('date_added');
  let nowPlayingDrawerOpen = $state(false);

  return {
    get showSettings() {
      return showSettings;
    },
    get showDeviceSync() {
      return showDeviceSync;
    },
    get showVisualizer() {
      return showVisualizer;
    },
    get albumSort() {
      return albumSort;
    },
    get nowPlayingDrawerOpen() {
      return nowPlayingDrawerOpen;
    },
    openSettings() {
      showSettings = true;
      showDeviceSync = false;
      nowPlayingDrawerOpen = false;
    },
    closeSettings() {
      showSettings = false;
    },
    openDeviceSync() {
      showDeviceSync = true;
      showSettings = false;
      nowPlayingDrawerOpen = false;
    },
    closeDeviceSync() {
      showDeviceSync = false;
    },
    openVisualizer() {
      showVisualizer = true;
    },
    closeVisualizer() {
      showVisualizer = false;
    },
    setAlbumSort(sort: AlbumSort) {
      albumSort = sort;
    },
    // Deliberately mutually exclusive with Settings/Device Sync: the drawer
    // is a fixed-position overlay that would otherwise visually stack on
    // top of those panels rather than replacing them.
    openNowPlayingDrawer() {
      nowPlayingDrawerOpen = true;
      showSettings = false;
      showDeviceSync = false;
    },
    closeNowPlayingDrawer() {
      nowPlayingDrawerOpen = false;
    },
  };
}

export { createUiStore };
export const ui = createUiStore();
