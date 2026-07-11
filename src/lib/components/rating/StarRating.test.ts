import { describe, expect, it, vi, beforeEach, afterEach } from "vitest";
import { render, fireEvent } from "@testing-library/svelte";
import StarRating from "./StarRating.svelte";

// happy-dom stubs getBoundingClientRect to all zeros. We give star slots a
// realistic width so the left/right half-star split produces predictable
// results: clientX < 8 → left half → half-star; clientX ≥ 8 → right half →
// full star.
const STAR_SLOT_RECT: DOMRect = {
  left: 0, top: 0, right: 16, bottom: 16, width: 16, height: 16,
  x: 0, y: 0, toJSON: () => ({}),
};

beforeEach(() => {
  vi.spyOn(HTMLElement.prototype, "getBoundingClientRect").mockReturnValue(STAR_SLOT_RECT);
});

afterEach(() => {
  vi.restoreAllMocks();
});

function fullCount(container: HTMLElement) {
  return container.querySelectorAll(".star-icon.full").length;
}

function halfCount(container: HTMLElement) {
  return container.querySelectorAll(".star-icon.half-fill").length;
}

describe("StarRating — display", () => {
  it("renders 10 star buttons in interactive mode", () => {
    const { getAllByRole } = render(StarRating, {
      props: { rating: null, onRate: vi.fn() },
    });
    expect(getAllByRole("button")).toHaveLength(10);
  });

  it("renders no buttons in readonly mode", () => {
    const { queryAllByRole } = render(StarRating, {
      props: { rating: 5, readonly: true },
    });
    expect(queryAllByRole("button")).toHaveLength(0);
  });

  it("includes the rating value in the readonly aria-label", () => {
    const { container } = render(StarRating, {
      props: { rating: 7.5, readonly: true },
    });
    expect(container.querySelector("[aria-label]")).toHaveAttribute(
      "aria-label",
      "Rating: 7.5 of 10",
    );
  });

  it("uses 'unrated' in the readonly aria-label when rating is null", () => {
    const { container } = render(StarRating, {
      props: { rating: null, readonly: true },
    });
    expect(container.querySelector("[aria-label]")).toHaveAttribute(
      "aria-label",
      "Rating: unrated of 10",
    );
  });

  it("shows no filled stars when rating is null", () => {
    const { container } = render(StarRating, {
      props: { rating: null, onRate: vi.fn() },
    });
    expect(fullCount(container)).toBe(0);
    expect(halfCount(container)).toBe(0);
  });

  it("shows all 10 filled stars for a perfect rating", () => {
    const { container } = render(StarRating, {
      props: { rating: 10, onRate: vi.fn() },
    });
    expect(fullCount(container)).toBe(10);
    expect(halfCount(container)).toBe(0);
  });

  it("shows the correct number of full stars for a whole-number rating", () => {
    const { container } = render(StarRating, {
      props: { rating: 6, onRate: vi.fn() },
    });
    expect(fullCount(container)).toBe(6);
    expect(halfCount(container)).toBe(0);
  });

  it("shows a half-star for a 0.5-increment rating", () => {
    const { container } = render(StarRating, {
      props: { rating: 4.5, onRate: vi.fn() },
    });
    expect(fullCount(container)).toBe(4);
    expect(halfCount(container)).toBe(1);
  });
});

describe("StarRating — interaction", () => {
  it("calls onRate with the star index when the right half of a star is clicked", () => {
    const onRate = vi.fn();
    const { getAllByRole } = render(StarRating, {
      props: { rating: null, onRate },
    });
    // clientX: 12 → 12 ≥ 8 (rect.width/2) → right half → full star at index 3
    fireEvent.click(getAllByRole("button")[2], { clientX: 12 });
    expect(onRate).toHaveBeenCalledWith(3);
  });

  it("calls onRate with index - 0.5 when the left half of a star is clicked", () => {
    const onRate = vi.fn();
    const { getAllByRole } = render(StarRating, {
      props: { rating: null, onRate },
    });
    // clientX: 4 → 4 < 8 (rect.width/2) → left half → half-star at index 3
    fireEvent.click(getAllByRole("button")[2], { clientX: 4 });
    expect(onRate).toHaveBeenCalledWith(2.5);
  });

  it("calls onRate with null when clicking the slot matching the current rating", () => {
    const onRate = vi.fn();
    const { getAllByRole } = render(StarRating, {
      props: { rating: 3, onRate },
    });
    // Clicking right half of star 3 with current rating 3 → toggle off
    fireEvent.click(getAllByRole("button")[2], { clientX: 12 });
    expect(onRate).toHaveBeenCalledWith(null);
  });

  it("does not call onRate when readonly", () => {
    const onRate = vi.fn();
    const { container } = render(StarRating, {
      props: { rating: 5, readonly: true, onRate },
    });
    // Readonly renders spans, not buttons — clicking them has no handler
    const slots = container.querySelectorAll(".star-slot");
    fireEvent.click(slots[2]);
    expect(onRate).not.toHaveBeenCalled();
  });
});
