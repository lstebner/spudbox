import { describe, expect, it, vi, beforeEach } from "vitest";
import { render } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import TransportBar from "./TransportBar.svelte";

// Defined inside vi.hoisted so it is available when the vi.mock() factory runs
// (both are hoisted before top-level const declarations). State is typed as a
// union so tests can assign "playing"/"paused" snapshots without type errors.
const playerState = vi.hoisted(() => {
  const stopped = {
    state: "stopped" as "stopped" | "playing" | "paused",
    track_id: null as number | null,
    position_ms: 0,
    duration_ms: 0,
    volume: 1,
    title: null as string | null,
    artist: null as string | null,
    album: null as string | null,
    album_id: null as number | null,
    art_path: null as string | null,
  };
  return { snapshot: { ...stopped }, stopped };
});
const mockTogglePlayPause = vi.hoisted(() => vi.fn());
const mockNext = vi.hoisted(() => vi.fn());
const mockPrevious = vi.hoisted(() => vi.fn());
const mockOpenNowPlayingDrawer = vi.hoisted(() => vi.fn());

vi.mock("$lib/stores/player.svelte", () => ({
  player: {
    get snapshot() { return playerState.snapshot; },
    get eqGains() { return new Array(8).fill(0); },
    get eqEnabled() { return true; },
    togglePlayPause: mockTogglePlayPause,
    next: mockNext,
    previous: mockPrevious,
    seek: vi.fn(),
    setVolume: vi.fn(),
    setEq: vi.fn(),
  },
}));

vi.mock("$lib/stores/library.svelte", () => ({
  library: {
    findAlbumById: vi.fn().mockReturnValue(null),
    get albums() { return []; },
  },
}));

vi.mock("$lib/stores/ui.svelte", () => ({
  ui: {
    openNowPlayingDrawer: mockOpenNowPlayingDrawer,
    get showSettings() { return false; },
    get nowPlayingDrawerOpen() { return false; },
    get albumSort() { return "date_added"; },
  },
}));

beforeEach(() => {
  playerState.snapshot = { ...playerState.stopped };
  mockTogglePlayPause.mockClear();
  mockNext.mockClear();
  mockPrevious.mockClear();
  mockOpenNowPlayingDrawer.mockClear();
});

describe("TransportBar — stopped state (no track)", () => {
  it("shows 'Nothing playing' when no track is loaded", () => {
    const { getByText } = render(TransportBar);
    expect(getByText("Nothing playing")).toBeInTheDocument();
  });

  it("skip buttons are disabled when no track is loaded", () => {
    const { getByRole } = render(TransportBar);
    expect(getByRole("button", { name: "Previous" })).toBeDisabled();
    expect(getByRole("button", { name: "Next" })).toBeDisabled();
  });

  it("now-playing button is disabled when album_id is null", () => {
    const { getByRole } = render(TransportBar);
    expect(getByRole("button", { name: "Show now playing" })).toBeDisabled();
  });
});

describe("TransportBar — playing state", () => {
  const PLAYING_SNAPSHOT = {
    state: "playing" as const,
    track_id: 1,
    position_ms: 5000,
    duration_ms: 300_000,
    volume: 1,
    title: "Moonlight Sonata",
    artist: "Beethoven",
    album: "Piano Sonatas",
    album_id: 10,
    art_path: null,
  };

  beforeEach(() => {
    playerState.snapshot = { ...PLAYING_SNAPSHOT };
  });

  it("shows the track title when a track is loaded", () => {
    const { getByText } = render(TransportBar);
    expect(getByText("Moonlight Sonata")).toBeInTheDocument();
  });

  it("shows the artist name when a track is loaded", () => {
    const { getByText } = render(TransportBar);
    expect(getByText("Beethoven")).toBeInTheDocument();
  });

  it("skip buttons are enabled when a track is loaded", () => {
    const { getByRole } = render(TransportBar);
    expect(getByRole("button", { name: "Previous" })).not.toBeDisabled();
    expect(getByRole("button", { name: "Next" })).not.toBeDisabled();
  });

  it("now-playing button is enabled when album_id is set", () => {
    const { getByRole } = render(TransportBar);
    expect(getByRole("button", { name: "Show now playing" })).not.toBeDisabled();
  });
});

describe("TransportBar — control interactions", () => {
  beforeEach(() => {
    playerState.snapshot = { ...playerState.stopped, track_id: 1 };
  });

  it("clicking Previous calls player.previous()", async () => {
    const user = userEvent.setup();
    const { getByRole } = render(TransportBar);
    await user.click(getByRole("button", { name: "Previous" }));
    expect(mockPrevious).toHaveBeenCalledTimes(1);
  });

  it("clicking Next calls player.next()", async () => {
    const user = userEvent.setup();
    const { getByRole } = render(TransportBar);
    await user.click(getByRole("button", { name: "Next" }));
    expect(mockNext).toHaveBeenCalledTimes(1);
  });

  it("clicking the now-playing button calls ui.openNowPlayingDrawer()", async () => {
    playerState.snapshot = { ...playerState.stopped, track_id: 1, album_id: 10 };
    const user = userEvent.setup();
    const { getByRole } = render(TransportBar);
    await user.click(getByRole("button", { name: "Show now playing" }));
    expect(mockOpenNowPlayingDrawer).toHaveBeenCalledTimes(1);
  });
});
