import { describe, expect, it } from "vitest";
import { createUiStore } from "./ui.svelte";

// Each test calls createUiStore() for an isolated instance so tests don't
// share state through the module-level singleton.

describe("ui store — initial state", () => {
  it("starts with all panels closed", () => {
    const store = createUiStore();
    expect(store.showSettings).toBe(false);
    expect(store.showDeviceSync).toBe(false);
    expect(store.nowPlayingDrawerOpen).toBe(false);
  });

  it("starts with date_added sort", () => {
    const store = createUiStore();
    expect(store.albumSort).toBe("date_added");
  });
});

describe("ui store — settings panel", () => {
  it("opens settings", () => {
    const store = createUiStore();
    store.openSettings();
    expect(store.showSettings).toBe(true);
  });

  it("closes settings", () => {
    const store = createUiStore();
    store.openSettings();
    store.closeSettings();
    expect(store.showSettings).toBe(false);
  });

  it("opening settings closes device sync", () => {
    const store = createUiStore();
    store.openDeviceSync();
    store.openSettings();
    expect(store.showDeviceSync).toBe(false);
  });

  it("opening settings closes the now-playing drawer", () => {
    const store = createUiStore();
    store.openNowPlayingDrawer();
    store.openSettings();
    expect(store.nowPlayingDrawerOpen).toBe(false);
  });
});

describe("ui store — device sync panel", () => {
  it("opens device sync", () => {
    const store = createUiStore();
    store.openDeviceSync();
    expect(store.showDeviceSync).toBe(true);
  });

  it("closes device sync", () => {
    const store = createUiStore();
    store.openDeviceSync();
    store.closeDeviceSync();
    expect(store.showDeviceSync).toBe(false);
  });

  it("opening device sync closes settings", () => {
    const store = createUiStore();
    store.openSettings();
    store.openDeviceSync();
    expect(store.showSettings).toBe(false);
  });

  it("opening device sync closes the now-playing drawer", () => {
    const store = createUiStore();
    store.openNowPlayingDrawer();
    store.openDeviceSync();
    expect(store.nowPlayingDrawerOpen).toBe(false);
  });
});

describe("ui store — now-playing drawer", () => {
  it("opens the now-playing drawer", () => {
    const store = createUiStore();
    store.openNowPlayingDrawer();
    expect(store.nowPlayingDrawerOpen).toBe(true);
  });

  it("closes the now-playing drawer", () => {
    const store = createUiStore();
    store.openNowPlayingDrawer();
    store.closeNowPlayingDrawer();
    expect(store.nowPlayingDrawerOpen).toBe(false);
  });

  it("opening the now-playing drawer closes settings", () => {
    const store = createUiStore();
    store.openSettings();
    store.openNowPlayingDrawer();
    expect(store.showSettings).toBe(false);
  });

  it("opening the now-playing drawer closes device sync", () => {
    const store = createUiStore();
    store.openDeviceSync();
    store.openNowPlayingDrawer();
    expect(store.showDeviceSync).toBe(false);
  });
});

describe("ui store — album sort", () => {
  it("updates the sort order", () => {
    const store = createUiStore();
    store.setAlbumSort("artist_name");
    expect(store.albumSort).toBe("artist_name");
    store.setAlbumSort("album_name");
    expect(store.albumSort).toBe("album_name");
  });

  it("setting sort does not affect panel visibility", () => {
    const store = createUiStore();
    store.openSettings();
    store.setAlbumSort("artist_name");
    expect(store.showSettings).toBe(true);
  });
});
