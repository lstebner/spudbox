import { describe, expect, it, vi } from "vitest";
import { render } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { tick } from "svelte";
import Dropdown from "./Dropdown.svelte";

const options = [
  { value: "alpha", label: "Alpha" },
  { value: "beta", label: "Beta" },
  { value: "gamma", label: "Gamma" },
];

// openList() calls `await tick()` before focusing the selected option, so we
// need to flush that microtask before making focus assertions.
async function openAndFlush(user: ReturnType<typeof userEvent.setup>, trigger: HTMLElement) {
  await user.click(trigger);
  await tick();
}

describe("Dropdown", () => {
  it("shows the label of the currently selected value", () => {
    const { getByRole } = render(Dropdown, {
      props: { options, value: "beta", onChange: vi.fn(), ariaLabel: "Sort by" },
    });
    expect(getByRole("button", { name: "Sort by" })).toHaveTextContent(/Beta/);
  });

  it("opens the listbox when the trigger is clicked", async () => {
    const user = userEvent.setup();
    const { getByRole, queryByRole } = render(Dropdown, {
      props: { options, value: "alpha", onChange: vi.fn(), ariaLabel: "Sort by" },
    });
    expect(queryByRole("listbox")).not.toBeInTheDocument();
    await user.click(getByRole("button", { name: "Sort by" }));
    expect(getByRole("listbox")).toBeInTheDocument();
  });

  it("lists all options when open", async () => {
    const user = userEvent.setup();
    const { getByRole, getAllByRole } = render(Dropdown, {
      props: { options, value: "alpha", onChange: vi.fn(), ariaLabel: "Sort by" },
    });
    await user.click(getByRole("button", { name: "Sort by" }));
    const optionElements = getAllByRole("option");
    expect(optionElements).toHaveLength(3);
    expect(optionElements[0]).toHaveTextContent("Alpha");
    expect(optionElements[1]).toHaveTextContent("Beta");
    expect(optionElements[2]).toHaveTextContent("Gamma");
  });

  it("closes the listbox when the trigger is clicked while open", async () => {
    const user = userEvent.setup();
    const { getByRole, queryByRole } = render(Dropdown, {
      props: { options, value: "alpha", onChange: vi.fn(), ariaLabel: "Sort by" },
    });
    const trigger = getByRole("button", { name: "Sort by" });
    await user.click(trigger);
    await user.click(trigger);
    expect(queryByRole("listbox")).not.toBeInTheDocument();
  });

  it("calls onChange with the selected value when an option is clicked", async () => {
    const user = userEvent.setup();
    const onChange = vi.fn();
    const { getByRole } = render(Dropdown, {
      props: { options, value: "alpha", onChange, ariaLabel: "Sort by" },
    });
    await user.click(getByRole("button", { name: "Sort by" }));
    await user.click(getByRole("option", { name: "Beta" }));
    expect(onChange).toHaveBeenCalledWith("beta");
    expect(onChange).toHaveBeenCalledTimes(1);
  });

  it("closes the listbox after an option is selected", async () => {
    const user = userEvent.setup();
    const { getByRole, queryByRole } = render(Dropdown, {
      props: { options, value: "alpha", onChange: vi.fn(), ariaLabel: "Sort by" },
    });
    await user.click(getByRole("button", { name: "Sort by" }));
    await user.click(getByRole("option", { name: "Beta" }));
    expect(queryByRole("listbox")).not.toBeInTheDocument();
  });

  it("marks the current value as selected and others as not selected", async () => {
    const user = userEvent.setup();
    const { getByRole } = render(Dropdown, {
      props: { options, value: "beta", onChange: vi.fn(), ariaLabel: "Sort by" },
    });
    await user.click(getByRole("button", { name: "Sort by" }));
    expect(getByRole("option", { name: "Beta" })).toHaveAttribute("aria-selected", "true");
    expect(getByRole("option", { name: "Alpha" })).toHaveAttribute("aria-selected", "false");
    expect(getByRole("option", { name: "Gamma" })).toHaveAttribute("aria-selected", "false");
  });

  it("reflects open state in aria-expanded on the trigger", async () => {
    const user = userEvent.setup();
    const { getByRole } = render(Dropdown, {
      props: { options, value: "alpha", onChange: vi.fn(), ariaLabel: "Sort by" },
    });
    const trigger = getByRole("button", { name: "Sort by" });
    expect(trigger).toHaveAttribute("aria-expanded", "false");
    await user.click(trigger);
    expect(trigger).toHaveAttribute("aria-expanded", "true");
  });

  it("opens the listbox on ArrowDown from the trigger", async () => {
    const user = userEvent.setup();
    const { getByRole, queryByRole } = render(Dropdown, {
      props: { options, value: "alpha", onChange: vi.fn(), ariaLabel: "Sort by" },
    });
    getByRole("button", { name: "Sort by" }).focus();
    expect(queryByRole("listbox")).not.toBeInTheDocument();
    await user.keyboard("{ArrowDown}");
    expect(getByRole("listbox")).toBeInTheDocument();
  });

  it("opens the listbox on ArrowUp from the trigger", async () => {
    const user = userEvent.setup();
    const { getByRole, queryByRole } = render(Dropdown, {
      props: { options, value: "alpha", onChange: vi.fn(), ariaLabel: "Sort by" },
    });
    getByRole("button", { name: "Sort by" }).focus();
    expect(queryByRole("listbox")).not.toBeInTheDocument();
    await user.keyboard("{ArrowUp}");
    expect(getByRole("listbox")).toBeInTheDocument();
  });

  it("focuses the currently selected option when the listbox opens", async () => {
    const user = userEvent.setup();
    const { getByRole } = render(Dropdown, {
      props: { options, value: "beta", onChange: vi.fn(), ariaLabel: "Sort by" },
    });
    await openAndFlush(user, getByRole("button", { name: "Sort by" }));
    expect(getByRole("option", { name: "Beta" })).toHaveFocus();
  });

  it("moves focus to the next option on ArrowDown", async () => {
    const user = userEvent.setup();
    const { getByRole } = render(Dropdown, {
      props: { options, value: "alpha", onChange: vi.fn(), ariaLabel: "Sort by" },
    });
    await openAndFlush(user, getByRole("button", { name: "Sort by" }));
    await user.keyboard("{ArrowDown}");
    expect(getByRole("option", { name: "Beta" })).toHaveFocus();
  });

  it("moves focus to the previous option on ArrowUp", async () => {
    const user = userEvent.setup();
    const { getByRole } = render(Dropdown, {
      props: { options, value: "beta", onChange: vi.fn(), ariaLabel: "Sort by" },
    });
    await openAndFlush(user, getByRole("button", { name: "Sort by" }));
    await user.keyboard("{ArrowUp}");
    expect(getByRole("option", { name: "Alpha" })).toHaveFocus();
  });

  it("wraps focus from last to first option on ArrowDown", async () => {
    const user = userEvent.setup();
    const { getByRole } = render(Dropdown, {
      props: { options, value: "gamma", onChange: vi.fn(), ariaLabel: "Sort by" },
    });
    await openAndFlush(user, getByRole("button", { name: "Sort by" }));
    await user.keyboard("{ArrowDown}");
    expect(getByRole("option", { name: "Alpha" })).toHaveFocus();
  });

  it("wraps focus from first to last option on ArrowUp", async () => {
    const user = userEvent.setup();
    const { getByRole } = render(Dropdown, {
      props: { options, value: "alpha", onChange: vi.fn(), ariaLabel: "Sort by" },
    });
    await openAndFlush(user, getByRole("button", { name: "Sort by" }));
    await user.keyboard("{ArrowUp}");
    expect(getByRole("option", { name: "Gamma" })).toHaveFocus();
  });

  it("jumps focus to the first option on Home", async () => {
    const user = userEvent.setup();
    const { getByRole } = render(Dropdown, {
      props: { options, value: "gamma", onChange: vi.fn(), ariaLabel: "Sort by" },
    });
    await openAndFlush(user, getByRole("button", { name: "Sort by" }));
    await user.keyboard("{Home}");
    expect(getByRole("option", { name: "Alpha" })).toHaveFocus();
  });

  it("jumps focus to the last option on End", async () => {
    const user = userEvent.setup();
    const { getByRole } = render(Dropdown, {
      props: { options, value: "alpha", onChange: vi.fn(), ariaLabel: "Sort by" },
    });
    await openAndFlush(user, getByRole("button", { name: "Sort by" }));
    await user.keyboard("{End}");
    expect(getByRole("option", { name: "Gamma" })).toHaveFocus();
  });

  it("closes the listbox and returns focus to the trigger on Escape", async () => {
    const user = userEvent.setup();
    const { getByRole, queryByRole } = render(Dropdown, {
      props: { options, value: "alpha", onChange: vi.fn(), ariaLabel: "Sort by" },
    });
    const trigger = getByRole("button", { name: "Sort by" });
    await openAndFlush(user, trigger);
    await user.keyboard("{Escape}");
    expect(queryByRole("listbox")).not.toBeInTheDocument();
    expect(trigger).toHaveFocus();
  });

  it("closes the listbox when focus moves outside the component", async () => {
    const user = userEvent.setup();
    const { getByRole, queryByRole } = render(Dropdown, {
      props: { options, value: "alpha", onChange: vi.fn(), ariaLabel: "Sort by" },
    });
    await user.click(getByRole("button", { name: "Sort by" }));
    expect(getByRole("listbox")).toBeInTheDocument();
    await user.click(document.body);
    expect(queryByRole("listbox")).not.toBeInTheDocument();
  });
});
