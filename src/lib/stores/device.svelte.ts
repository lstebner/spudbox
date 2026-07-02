import { commands } from "$lib/api/commands";
import { onDeviceStatusChanged } from "$lib/api/events";
import { ui } from "$lib/stores/ui.svelte";
import type { DeviceStatus } from "$lib/types";

function createDeviceStore() {
  let status = $state<DeviceStatus>({
    connected: false,
    device_name: "",
    mount_path: "",
    detected_music_subfolder: null,
  });

  return {
    get connected() {
      return status.connected;
    },
    get deviceName() {
      return status.device_name;
    },
    get mountPath() {
      return status.mount_path;
    },
    get detectedMusicSubfolder() {
      return status.detected_music_subfolder;
    },

    async init() {
      status = await commands.deviceGetStatus();
      const unlisten = await onDeviceStatusChanged((payload) => {
        status = payload;
        if (!payload.connected) {
          commands.deviceCancelSync();
          ui.closeDeviceSync();
        }
      });
      return unlisten;
    },
  };
}

export const device = createDeviceStore();
