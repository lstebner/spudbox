import { describe, expect, it, vi, beforeEach } from "vitest";
import { render, fireEvent } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { tick } from "svelte";
import NowPlayingDrawer from "./NowPlayingDrawer.svelte";

const uiState = vi.hoisted(() => ({ drawerOpen: false }));
const mockCloseNowPlayingDrawer = vi.hoisted(() => vi.fn(() => { uiState.drawerOpen = false; }));

vi.mock("$lib/stores/ui.svelte", () => ({
  ui: {
    get nowPlayingDrawerOpen() { return uiState.drawerOpen; },
    closeNowPlayingDrawer: mockCloseNowPlayingDrawer,
    openNowPlayingDrawer: vi.fn(),
    openSettings: vi.fn(),
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

vi.mock("$lib/stores/library.svelte", () => ({
  library: {
    findAlbumById: vi.fn().mockReturnValue(null),
    setAlbumRating: vi.fn(),
  },
}));

// Svelte transitions use requestAnimationFrame which doesn't animate in
// happy-dom; stub them so elements appear immediately on condition change.
vi.mock("svelte/transition", () => ({
  fade: () => ({ duration: 0, css: () => "" }),
  fly: () => ({ duration: 0, css: () => "" }),
}));

beforeEach(() => {
  uiState.drawerOpen = false;
  mockCloseNowPlayingDrawer.mockClear();
});

describe("NowPlayingDrawer — closed", () => {
  it("renders nothing when the drawer is closed", () => {
    const { queryByRole } = render(NowPlayingDrawer);
    expect(queryByRole("dialog")).not.toBeInTheDocument();
  });
});

describe("NowPlayingDrawer — open", () => {
  beforeEach(() => {
    uiState.drawerOpen = true;
  });

  it("renders the dialog when open", async () => {
    const { getByRole } = render(NowPlayingDrawer);
    await tick();
    expect(getByRole("dialog")).toBeInTheDocument();
  });

  it("dialog has aria-label 'Now playing'", async () => {
    const { getByRole } = render(NowPlayingDrawer);
    await tick();
    expect(getByRole("dialog")).toHaveAttribute("aria-label", "Now playing");
  });

  it("renders the close button", async () => {
    const { getByRole } = render(NowPlayingDrawer);
    await tick();
    expect(getByRole("button", { name: "Close now playing panel" })).toBeInTheDocument();
  });

  it("focus moves to the close button when the drawer opens", async () => {
    const { getByRole } = render(NowPlayingDrawer);
    await tick();
    expect(document.activeElement).toBe(getByRole("button", { name: "Close now playing panel" }));
  });

  it("clicking the close button calls ui.closeNowPlayingDrawer()", async () => {
    const user = userEvent.setup();
    const { getByRole } = render(NowPlayingDrawer);
    await tick();
    await user.click(getByRole("button", { name: "Close now playing panel" }));
    expect(mockCloseNowPlayingDrawer).toHaveBeenCalledTimes(1);
  });

  it("pressing Escape calls ui.closeNowPlayingDrawer()", async () => {
    render(NowPlayingDrawer);
    await tick();
    window.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true }));
    expect(mockCloseNowPlayingDrawer).toHaveBeenCalledTimes(1);
  });

  it("clicking the backdrop calls ui.closeNowPlayingDrawer()", async () => {
    const { container } = render(NowPlayingDrawer);
    await tick();
    const backdrop = container.querySelector(".backdrop")!;
    fireEvent.click(backdrop);
    expect(mockCloseNowPlayingDrawer).toHaveBeenCalledTimes(1);
  });
});
