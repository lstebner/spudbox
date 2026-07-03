export type AlbumSort = 'date_added' | 'artist_name' | 'album_name';

function createUiStore() {
  let showSettings = $state(false);
  let showDeviceSync = $state(false);
  let albumSort = $state<AlbumSort>('date_added');

  return {
    get showSettings() {
      return showSettings;
    },
    get showDeviceSync() {
      return showDeviceSync;
    },
    get albumSort() {
      return albumSort;
    },
    openSettings() {
      showSettings = true;
      showDeviceSync = false;
    },
    closeSettings() {
      showSettings = false;
    },
    openDeviceSync() {
      showDeviceSync = true;
      showSettings = false;
    },
    closeDeviceSync() {
      showDeviceSync = false;
    },
    setAlbumSort(sort: AlbumSort) {
      albumSort = sort;
    },
  };
}

export const ui = createUiStore();
