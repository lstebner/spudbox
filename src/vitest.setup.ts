import "@testing-library/jest-dom/vitest";
import { afterEach, vi } from "vitest";
import { cleanup } from "@testing-library/svelte";

afterEach(() => cleanup());

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn().mockResolvedValue(null),
  convertFileSrc: vi.fn((path: string) => path),
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn().mockResolvedValue(() => {}),
}));

// Per-command defaults so stores that fetch on init (player, theme) receive
// valid data instead of null, which would otherwise throw on field access.
vi.mock("$lib/api/commands", () => {
  const STOPPED_SNAPSHOT = {
    state: "stopped",
    track_id: null,
    position_ms: 0,
    duration_ms: 0,
    volume: 1,
    title: null,
    artist: null,
    album: null,
    album_id: null,
    art_path: null,
  };

  return {
    commands: {
      ping: vi.fn().mockResolvedValue("pong"),

      libraryAddRoot: vi.fn().mockResolvedValue(null),
      libraryHasRoots: vi.fn().mockResolvedValue(false),
      libraryListRoots: vi.fn().mockResolvedValue([]),
      libraryRemoveRoot: vi.fn().mockResolvedValue(null),
      libraryScan: vi.fn().mockResolvedValue({ scanned: 0, added: 0, updated: 0, unchanged: 0, removed: 0, errors: 0 }),
      libraryGetArtists: vi.fn().mockResolvedValue([]),
      libraryGetAlbums: vi.fn().mockResolvedValue([]),
      librarySetAlbumHidden: vi.fn().mockResolvedValue(null),
      libraryGetTracks: vi.fn().mockResolvedValue([]),
      libraryGetTracksByAlbum: vi.fn().mockResolvedValue([]),
      librarySetAlbumRating: vi.fn().mockResolvedValue(null),

      playbackPlayQueue: vi.fn().mockResolvedValue(null),
      playbackStop: vi.fn().mockResolvedValue(null),
      playbackPlay: vi.fn().mockResolvedValue(null),
      playbackPause: vi.fn().mockResolvedValue(null),
      playbackNext: vi.fn().mockResolvedValue(null),
      playbackPrevious: vi.fn().mockResolvedValue(null),
      playbackSeek: vi.fn().mockResolvedValue(null),
      playbackSetVolume: vi.fn().mockResolvedValue(null),
      playbackGetSnapshot: vi.fn().mockResolvedValue(STOPPED_SNAPSHOT),
      playbackSetEq: vi.fn().mockResolvedValue(null),
      playbackGetEq: vi.fn().mockResolvedValue({ gains_db: new Array(8).fill(0), enabled: true }),

      syncConfigure: vi.fn().mockResolvedValue(null),
      syncStatus: vi.fn().mockResolvedValue(null),
      syncNow: vi.fn().mockResolvedValue(null),

      deviceGetStatus: vi.fn().mockResolvedValue(null),
      deviceFindMusicFolders: vi.fn().mockResolvedValue([]),
      deviceSaveMusicSubfolder: vi.fn().mockResolvedValue(null),
      devicePreviewSync: vi.fn().mockResolvedValue(null),
      devicePerformSync: vi.fn().mockResolvedValue(null),
      deviceCancelSync: vi.fn().mockResolvedValue(null),
      deviceCancelPreview: vi.fn().mockResolvedValue(null),

      appearanceGetTheme: vi.fn().mockResolvedValue("dark"),
      appearanceSetTheme: vi.fn().mockResolvedValue(null),
    },
  };
});
