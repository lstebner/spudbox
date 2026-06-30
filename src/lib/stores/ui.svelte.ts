function createUiStore() {
  let showSettings = $state(false);

  return {
    get showSettings() {
      return showSettings;
    },
    openSettings() {
      showSettings = true;
    },
    closeSettings() {
      showSettings = false;
    },
  };
}

export const ui = createUiStore();
