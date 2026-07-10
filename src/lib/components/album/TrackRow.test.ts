import { describe, expect, it, vi } from "vitest";
import { render } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import TrackRow from "./TrackRow.svelte";
import type { TrackRow as TrackRowData } from "$lib/types";

const baseTrack: TrackRowData = {
  id: 1,
  title: "Moonlight Sonata",
  artist: "Beethoven",
  album: "Piano Sonatas",
  album_id: 10,
  duration_ms: 361_000,
  sample_rate: 44100,
  bit_depth: 16,
  channels: 2,
  codec: "flac",
  disc_no: 1,
  track_no: 3,
};

describe("TrackRow", () => {
  it("renders the track title", () => {
    const { getByText } = render(TrackRow, {
      props: { track: baseTrack, isPlaying: false, onclick: vi.fn() },
    });
    expect(getByText("Moonlight Sonata")).toBeInTheDocument();
  });

  it("renders the formatted duration", () => {
    const { getByText } = render(TrackRow, {
      props: { track: baseTrack, isPlaying: false, onclick: vi.fn() },
    });
    expect(getByText("6:01")).toBeInTheDocument();
  });

  it("renders the track number when not playing", () => {
    const { getByText } = render(TrackRow, {
      props: { track: baseTrack, isPlaying: false, onclick: vi.fn() },
    });
    expect(getByText("3")).toBeInTheDocument();
  });

  it("renders an empty track number column when track_no is null", () => {
    const track = { ...baseTrack, track_no: null };
    const { container } = render(TrackRow, {
      props: { track, isPlaying: false, onclick: vi.fn() },
    });
    expect(container.querySelector(".col-no")).toHaveTextContent("");
  });

  it("shows the play icon instead of track number when playing", () => {
    const { container, queryByText } = render(TrackRow, {
      props: { track: baseTrack, isPlaying: true, onclick: vi.fn() },
    });
    expect(queryByText("3")).not.toBeInTheDocument();
    expect(container.querySelector("svg")).toBeInTheDocument();
  });

  it("applies the playing class when isPlaying is true", () => {
    const { container } = render(TrackRow, {
      props: { track: baseTrack, isPlaying: true, onclick: vi.fn() },
    });
    expect(container.querySelector(".track-row")).toHaveClass("playing");
  });

  it("does not apply the playing class when isPlaying is false", () => {
    const { container } = render(TrackRow, {
      props: { track: baseTrack, isPlaying: false, onclick: vi.fn() },
    });
    expect(container.querySelector(".track-row")).not.toHaveClass("playing");
  });

  it("calls onclick when the row is clicked", async () => {
    const user = userEvent.setup();
    const onclick = vi.fn();
    const { getByRole } = render(TrackRow, {
      props: { track: baseTrack, isPlaying: false, onclick },
    });
    await user.click(getByRole("button"));
    expect(onclick).toHaveBeenCalledTimes(1);
  });
});
