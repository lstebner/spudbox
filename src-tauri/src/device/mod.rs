pub mod detection;
pub mod sync;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize)]
pub struct DeviceStatus {
    pub connected: bool,
    /// Display name derived from the first directory inside the gvfs mount.
    pub device_name: String,
    /// Absolute path to the gvfs MTP mount root (e.g. /run/user/1000/gvfs/mtp:host=...).
    pub mount_path: String,
    /// Music subfolder relative to mount root, either from saved settings or
    /// auto-detected. `None` if no device is connected or no folder was found.
    pub detected_music_subfolder: Option<String>,
}

impl DeviceStatus {
    pub fn disconnected() -> Self {
        DeviceStatus {
            connected: false,
            device_name: String::new(),
            mount_path: String::new(),
            detected_music_subfolder: None,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct SyncEntry {
    pub relative_path: String,
    pub size_bytes: u64,
    pub artist: String,
    pub album: String,
    pub title: String,
}

#[derive(Debug, Serialize)]
pub struct SyncPreview {
    pub to_add: Vec<SyncEntry>,
    pub to_delete: Vec<SyncEntry>,
    pub device_free_bytes: u64,
    /// Total bytes of all files in `to_add`.
    pub required_bytes: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct SyncPreviewProgress {
    /// Running count of audio files found on the device so far.
    pub device_tracks_found: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct SyncProgress {
    pub current: usize,
    pub total: usize,
    pub current_file: String,
    pub phase: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SyncMode {
    AdditionsOnly,
    All,
}
