import { describe, expect, it, vi, beforeEach } from "vitest";
import { render } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import TrackTable from "./TrackTable.svelte";

const mockBackToAlbums = vi.hoisted(() => vi.fn());

vi.mock("$lib/stores/library.svelte", () => ({
  library: {
    get albums() { return []; },
    get tracks() { return []; },
    get selectedAlbumId() { return null; },
    backToAlbums: mockBackToAlbums,
    setAlbumRating: vi.fn(),
  },
}));

vi.mock("$lib/stores/player.svelte", () => ({
  player: {
    get snapshot() {
      return { state: "stopped", track_id: null, position_ms: 0, duration_ms: 0, volume: 1, title: null, artist: null, album: null, album_id: null, art_path: null };
    },
    playQueue: vi.fn(),
  },
}));

beforeEach(() => {
  mockBackToAlbums.mockClear();
});

describe("TrackTable", () => {
  it("renders the Albums back button", () => {
    const { getByRole } = render(TrackTable);
    expect(getByRole("button", { name: /Albums/i })).toBeInTheDocument();
  });

  it("clicking the back button calls library.backToAlbums()", async () => {
    const user = userEvent.setup();
    const { getByRole } = render(TrackTable);
    await user.click(getByRole("button", { name: /Albums/i }));
    expect(mockBackToAlbums).toHaveBeenCalledTimes(1);
  });

  it("renders no track rows when the track list is empty", () => {
    const { queryAllByRole } = render(TrackTable);
    // Only the back button — no track row buttons
    const buttons = queryAllByRole("button");
    expect(buttons.length).toBe(1);
  });
});
