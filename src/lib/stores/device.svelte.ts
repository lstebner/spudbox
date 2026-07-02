import { commands } from "$lib/api/commands";
import {
  onDevicePreviewProgress,
  onDeviceStatusChanged,
  onDeviceSyncProgress,
  onDeviceSyncStarted,
  onDeviceSyncEnded,
} from "$lib/api/events";
import { ui } from "$lib/stores/ui.svelte";
import type { DeviceStatus, DeviceSyncMode, DeviceSyncPreview, DeviceSyncProgress, DeviceSyncResult } from "$lib/types";

function createDeviceStore() {
  let status = $state<DeviceStatus>({
    connected: false,
    device_name: "",
    mount_path: "",
    detected_music_subfolder: null,
  });
  let previewRunning = $state(false);
  let previewProgressCount = $state(0);
  let syncRunning = $state(false);
  let syncProgress = $state<DeviceSyncProgress | null>(null);

  return {
    get connected() { return status.connected; },
    get deviceName() { return status.device_name; },
    get mountPath() { return status.mount_path; },
    get detectedMusicSubfolder() { return status.detected_music_subfolder; },
    get previewRunning() { return previewRunning; },
    get previewProgressCount() { return previewProgressCount; },
    get syncRunning() { return syncRunning; },
    get syncProgress() { return syncProgress; },

    async init() {
      status = await commands.deviceGetStatus();

      // These listeners are permanent for the app lifetime — syncRunning reflects
      // actual backend state, not JS execution flow, so closing and re-opening the
      // panel never loses the running state.
      await onDeviceSyncStarted(() => { syncRunning = true; syncProgress = null; });
      await onDeviceSyncEnded(() => { syncRunning = false; syncProgress = null; });
      await onDeviceSyncProgress((payload) => { syncProgress = payload; });
      await onDevicePreviewProgress((payload) => { previewProgressCount = payload.device_tracks_found; });

      const unlisten = await onDeviceStatusChanged((payload) => {
        status = payload;
        if (!payload.connected) {
          commands.deviceCancelPreview();
          commands.deviceCancelSync();
          ui.closeDeviceSync();
          previewRunning = false;
          syncRunning = false;
          syncProgress = null;
        }
      });
      return unlisten;
    },

    // Returns null when the preview was cancelled (not an error worth showing).
    async performPreview(subfolder: string): Promise<DeviceSyncPreview | null> {
      previewRunning = true;
      previewProgressCount = 0;
      try {
        return await commands.devicePreviewSync(subfolder);
      } catch (error) {
        if (String(error) === "cancelled") return null;
        throw error;
      } finally {
        previewRunning = false;
      }
    },

    async cancelPreview(): Promise<void> {
      await commands.deviceCancelPreview();
    },

    async performSync(subfolder: string, mode: DeviceSyncMode, preview: DeviceSyncPreview): Promise<DeviceSyncResult> {
      // Set eagerly so the confirm dialog appears immediately if the user tries to
      // close, without waiting for the device-sync-started event to round-trip.
      // device-sync-ended (always emitted by Rust on completion) drives it back
      // to false. If the call fails with "already in progress", syncRunning was
      // already true from the first sync and stays true until that sync ends.
      syncRunning = true;
      syncProgress = null;
      previewProgressCount = 0;
      return commands.devicePerformSync(subfolder, mode, preview);
    },

    async cancelSync(): Promise<void> {
      await commands.deviceCancelSync();
    },
  };
}

export const device = createDeviceStore();
