export type AlbumSort = 'date_added' | 'name';

function createUiStore() {
  let showSettings = $state(false);
  let albumSort = $state<AlbumSort>('date_added');

  return {
    get showSettings() {
      return showSettings;
    },
    get albumSort() {
      return albumSort;
    },
    openSettings() {
      showSettings = true;
    },
    closeSettings() {
      showSettings = false;
    },
    setAlbumSort(sort: AlbumSort) {
      albumSort = sort;
    },
  };
}

export const ui = createUiStore();
