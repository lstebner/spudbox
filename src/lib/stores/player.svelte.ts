import { commands } from "$lib/api/commands";
import { onPlaybackProgress } from "$lib/api/events";
import type { PlaybackSnapshot } from "$lib/types";

function createPlayerStore() {
  let snapshot = $state<PlaybackSnapshot>({
    state: "stopped",
    track_id: null,
    position_ms: 0,
    duration_ms: 0,
    volume: 1,
    title: null,
    artist: null,
    album: null,
    art_path: null,
  });

  onPlaybackProgress((payload) => {
    snapshot = payload;
  });

  commands.playbackGetSnapshot().then((s) => {
    snapshot = s;
  });

  return {
    get snapshot() {
      return snapshot;
    },
    playQueue(trackIds: number[], startIndex: number) {
      return commands.playbackPlayQueue(trackIds, startIndex);
    },
    togglePlayPause() {
      return snapshot.state === "playing" ? commands.playbackPause() : commands.playbackPlay();
    },
    next() {
      return commands.playbackNext();
    },
    previous() {
      return commands.playbackPrevious();
    },
    seek(positionMs: number) {
      return commands.playbackSeek(positionMs);
    },
    setVolume(volume: number) {
      return commands.playbackSetVolume(volume);
    },
  };
}

export const player = createPlayerStore();
