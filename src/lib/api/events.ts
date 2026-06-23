import { listen } from "@tauri-apps/api/event";
import type { PlaybackSnapshot } from "$lib/types";

export type ScanProgressPayload = { scanned: number; total: number };

export const onPlaybackProgress = (cb: (payload: PlaybackSnapshot) => void) =>
  listen<PlaybackSnapshot>("playback-progress", (event) => cb(event.payload));

export const onScanProgress = (cb: (payload: ScanProgressPayload) => void) =>
  listen<ScanProgressPayload>("scan-progress", (event) => cb(event.payload));
