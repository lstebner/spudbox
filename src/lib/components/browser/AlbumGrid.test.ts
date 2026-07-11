import { describe, expect, it, vi, beforeEach } from "vitest";
import { render } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import AlbumGrid from "./AlbumGrid.svelte";

vi.mock("@tauri-apps/plugin-dialog", () => ({
  open: vi.fn().mockResolvedValue(null),
}));

const libraryState = vi.hoisted(() => ({ hasRoots: false, loading: false }));
const mockOpenSettings = vi.hoisted(() => vi.fn());

vi.mock("$lib/stores/library.svelte", () => ({
  library: {
    get hasRoots() { return libraryState.hasRoots; },
    get loading() { return libraryState.loading; },
    get albums() { return []; },
    get isViewingHidden() { return false; },
    addFolder: vi.fn(),
    setAlbumHidden: vi.fn(),
    selectAlbum: vi.fn(),
  },
}));

vi.mock("$lib/stores/ui.svelte", () => ({
  ui: {
    get albumSort() { return "date_added"; },
    openSettings: mockOpenSettings,
  },
}));

beforeEach(() => {
  libraryState.hasRoots = false;
  libraryState.loading = false;
  mockOpenSettings.mockClear();
});

describe("AlbumGrid — empty state (no music folder)", () => {
  it("shows the empty-state prompt when hasRoots is false", () => {
    const { getByText } = render(AlbumGrid);
    expect(getByText("No music folder added yet.")).toBeInTheDocument();
  });

  it("shows the Add Music Folder button", () => {
    const { getByRole } = render(AlbumGrid);
    expect(getByRole("button", { name: "Add Music Folder" })).toBeInTheDocument();
  });

  it("shows 'Scanning…' on the button while loading", () => {
    libraryState.loading = true;
    const { getByRole } = render(AlbumGrid);
    expect(getByRole("button", { name: "Scanning…" })).toBeInTheDocument();
  });

  it("disables the folder button while scanning", () => {
    libraryState.loading = true;
    const { getByRole } = render(AlbumGrid);
    expect(getByRole("button", { name: "Scanning…" })).toBeDisabled();
  });

  it("includes a Settings link in the hint text", () => {
    const { getByRole } = render(AlbumGrid);
    expect(getByRole("button", { name: "Settings" })).toBeInTheDocument();
  });

  it("clicking Settings calls ui.openSettings()", async () => {
    const user = userEvent.setup();
    const { getByRole } = render(AlbumGrid);
    await user.click(getByRole("button", { name: "Settings" }));
    expect(mockOpenSettings).toHaveBeenCalledTimes(1);
  });
});

describe("AlbumGrid — with roots", () => {
  it("does not show the empty-state prompt when hasRoots is true", () => {
    libraryState.hasRoots = true;
    const { queryByText } = render(AlbumGrid);
    expect(queryByText("No music folder added yet.")).not.toBeInTheDocument();
  });
});
