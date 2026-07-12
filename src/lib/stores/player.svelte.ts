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
    album_id: null,
    art_path: null,
    rms_amplitude: 0,
  });

  let eqGains = $state<number[]>(new Array(8).fill(0));
  let eqEnabled = $state<boolean>(true);

  onPlaybackProgress((payload) => {
    snapshot = payload;
  });

  commands.playbackGetSnapshot().then((s) => {
    snapshot = s;
  });

  commands.playbackGetEq().then((eq) => {
    eqGains = eq.gains_db;
    eqEnabled = eq.enabled;
  });

  return {
    get snapshot() {
      return snapshot;
    },
    get eqGains() {
      return eqGains;
    },
    get eqEnabled() {
      return eqEnabled;
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
    setEq(gains: number[], enabled: boolean) {
      eqGains = gains;
      eqEnabled = enabled;
      return commands.playbackSetEq(gains, enabled);
    },
  };
}

export const player = createPlayerStore();
