import { listen } from "@tauri-apps/api/event";
import type { DevicePreviewProgress, DeviceStatus, DeviceSyncProgress, PlaybackSnapshot } from "$lib/types";

export type ScanProgressPayload = { scanned: number; total: number };

export const onPlaybackProgress = (cb: (payload: PlaybackSnapshot) => void) =>
  listen<PlaybackSnapshot>("playback-progress", (event) => cb(event.payload));

export const onScanProgress = (cb: (payload: ScanProgressPayload) => void) =>
  listen<ScanProgressPayload>("scan-progress", (event) => cb(event.payload));

export const onDeviceStatusChanged = (cb: (payload: DeviceStatus) => void) =>
  listen<DeviceStatus>("device-status-changed", (event) => cb(event.payload));

export const onDevicePreviewProgress = (cb: (payload: DevicePreviewProgress) => void) =>
  listen<DevicePreviewProgress>("device-preview-progress", (event) => cb(event.payload));

export const onDeviceSyncProgress = (cb: (payload: DeviceSyncProgress) => void) =>
  listen<DeviceSyncProgress>("device-sync-progress", (event) => cb(event.payload));

export const onDeviceSyncStarted = (cb: () => void) =>
  listen("device-sync-started", () => cb());

export const onDeviceSyncEnded = (cb: () => void) =>
  listen("device-sync-ended", () => cb());
