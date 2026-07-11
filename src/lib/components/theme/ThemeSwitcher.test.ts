import { describe, expect, it, vi, beforeEach } from "vitest";
import { render } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { tick } from "svelte";
import ThemeSwitcher from "./ThemeSwitcher.svelte";

const themeState = vi.hoisted(() => ({ current: "dark" }));
const mockSetTheme = vi.hoisted(() => vi.fn((id: string) => { themeState.current = id; }));

vi.mock("$lib/stores/theme.svelte", () => ({
  THEMES: [
    { id: "dark", label: "Dark" },
    { id: "light", label: "Light" },
    { id: "mint", label: "Mint" },
    { id: "grape", label: "Grape" },
    { id: "lemon", label: "Lemon" },
  ],
  theme: {
    get current() { return themeState.current; },
    setTheme: mockSetTheme,
    init: vi.fn(),
  },
}));

async function openAndFlush(user: ReturnType<typeof userEvent.setup>, trigger: HTMLElement) {
  await user.click(trigger);
  await tick();
}

beforeEach(() => {
  themeState.current = "dark";
  mockSetTheme.mockClear();
});

describe("ThemeSwitcher — closed state", () => {
  it("renders the palette button", () => {
    const { getByRole } = render(ThemeSwitcher);
    expect(getByRole("button", { name: "Change theme" })).toBeInTheDocument();
  });

  it("does not show the option list when closed", () => {
    const { queryByRole } = render(ThemeSwitcher);
    expect(queryByRole("listbox")).not.toBeInTheDocument();
  });

  it("button has aria-expanded=false when closed", () => {
    const { getByRole } = render(ThemeSwitcher);
    expect(getByRole("button", { name: "Change theme" })).toHaveAttribute("aria-expanded", "false");
  });
});

describe("ThemeSwitcher — open/close", () => {
  it("shows the listbox after clicking the button", async () => {
    const user = userEvent.setup();
    const { getByRole } = render(ThemeSwitcher);
    await openAndFlush(user, getByRole("button", { name: "Change theme" }));
    expect(getByRole("listbox")).toBeInTheDocument();
  });

  it("button has aria-expanded=true when open", async () => {
    const user = userEvent.setup();
    const { getByRole } = render(ThemeSwitcher);
    await openAndFlush(user, getByRole("button", { name: "Change theme" }));
    expect(getByRole("button", { name: "Change theme" })).toHaveAttribute("aria-expanded", "true");
  });

  it("closes the listbox when clicking the button again", async () => {
    const user = userEvent.setup();
    const { getByRole, queryByRole } = render(ThemeSwitcher);
    const button = getByRole("button", { name: "Change theme" });
    await openAndFlush(user, button);
    await user.click(button);
    expect(queryByRole("listbox")).not.toBeInTheDocument();
  });

  it("closes when clicking outside the component", async () => {
    const user = userEvent.setup();
    const { getByRole, queryByRole } = render(ThemeSwitcher);
    await openAndFlush(user, getByRole("button", { name: "Change theme" }));
    await user.click(document.body);
    expect(queryByRole("listbox")).not.toBeInTheDocument();
  });
});

describe("ThemeSwitcher — theme options", () => {
  it("renders all five themes", async () => {
    const user = userEvent.setup();
    const { getByRole, getAllByRole } = render(ThemeSwitcher);
    await openAndFlush(user, getByRole("button", { name: "Change theme" }));
    expect(getAllByRole("option")).toHaveLength(5);
  });

  it("marks the current theme as selected", async () => {
    themeState.current = "mint";
    const user = userEvent.setup();
    const { getByRole } = render(ThemeSwitcher);
    await openAndFlush(user, getByRole("button", { name: "Change theme" }));
    expect(getByRole("option", { name: /Mint/ })).toHaveAttribute("aria-selected", "true");
  });

  it("calls setTheme with the selected id and closes the list", async () => {
    const user = userEvent.setup();
    const { getByRole, queryByRole } = render(ThemeSwitcher);
    await openAndFlush(user, getByRole("button", { name: "Change theme" }));
    await user.click(getByRole("option", { name: /Light/ }));
    expect(mockSetTheme).toHaveBeenCalledWith("light");
    expect(queryByRole("listbox")).not.toBeInTheDocument();
  });
});

describe("ThemeSwitcher — keyboard navigation", () => {
  it("opens the list on ArrowDown from the button", async () => {
    const user = userEvent.setup();
    const { getByRole } = render(ThemeSwitcher);
    getByRole("button", { name: "Change theme" }).focus();
    await user.keyboard("{ArrowDown}");
    await tick();
    expect(getByRole("listbox")).toBeInTheDocument();
  });

  it("ArrowDown moves focus to the next option", async () => {
    const user = userEvent.setup();
    const { getByRole, getAllByRole } = render(ThemeSwitcher);
    await openAndFlush(user, getByRole("button", { name: "Change theme" }));
    const options = getAllByRole("option");
    options[0].focus();
    await user.keyboard("{ArrowDown}");
    expect(document.activeElement).toBe(options[1]);
  });

  it("ArrowDown wraps from last to first option", async () => {
    const user = userEvent.setup();
    const { getByRole, getAllByRole } = render(ThemeSwitcher);
    await openAndFlush(user, getByRole("button", { name: "Change theme" }));
    const options = getAllByRole("option");
    // Use ArrowUp to wrap 0→last via keyboard (updates focusedOptionIndex),
    // then ArrowDown wraps last→first.
    await user.keyboard("{ArrowUp}");
    expect(document.activeElement).toBe(options[options.length - 1]);
    await user.keyboard("{ArrowDown}");
    expect(document.activeElement).toBe(options[0]);
  });

  it("ArrowUp moves focus to the previous option", async () => {
    const user = userEvent.setup();
    const { getByRole, getAllByRole } = render(ThemeSwitcher);
    await openAndFlush(user, getByRole("button", { name: "Change theme" }));
    const options = getAllByRole("option");
    await user.keyboard("{ArrowDown}"); // 0→1
    await user.keyboard("{ArrowUp}"); // 1→0
    expect(document.activeElement).toBe(options[0]);
  });

  it("ArrowUp wraps from first to last option", async () => {
    const user = userEvent.setup();
    const { getByRole, getAllByRole } = render(ThemeSwitcher);
    await openAndFlush(user, getByRole("button", { name: "Change theme" }));
    const options = getAllByRole("option");
    options[0].focus();
    await user.keyboard("{ArrowUp}");
    expect(document.activeElement).toBe(options[options.length - 1]);
  });

  it("Home moves focus to the first option", async () => {
    const user = userEvent.setup();
    const { getByRole, getAllByRole } = render(ThemeSwitcher);
    await openAndFlush(user, getByRole("button", { name: "Change theme" }));
    const options = getAllByRole("option");
    options[3].focus();
    await user.keyboard("{Home}");
    expect(document.activeElement).toBe(options[0]);
  });

  it("End moves focus to the last option", async () => {
    const user = userEvent.setup();
    const { getByRole, getAllByRole } = render(ThemeSwitcher);
    await openAndFlush(user, getByRole("button", { name: "Change theme" }));
    const options = getAllByRole("option");
    options[0].focus();
    await user.keyboard("{End}");
    expect(document.activeElement).toBe(options[options.length - 1]);
  });

  it("Escape closes the list and returns focus to the button", async () => {
    const user = userEvent.setup();
    const { getByRole, queryByRole } = render(ThemeSwitcher);
    const button = getByRole("button", { name: "Change theme" });
    await openAndFlush(user, button);
    await user.keyboard("{Escape}");
    expect(queryByRole("listbox")).not.toBeInTheDocument();
    expect(document.activeElement).toBe(button);
  });
});
