use std::path::{Path, PathBuf};

use tauri::{AppHandle, Emitter};
use walkdir::WalkDir;

use super::{DeviceKind, DeviceStatus};

const DEVICE_STATUS_EVENT: &str = "device-status-changed";
const POLL_INTERVAL: std::time::Duration = std::time::Duration::from_secs(3);

pub struct DeviceMount {
    pub kind: DeviceKind,
    pub mount_path: PathBuf,
    pub device_name: String,
}

/// Returns the gvfs directory root for any connected MTP device, or `None`
/// if no MTP mount is present. Uses `$XDG_RUNTIME_DIR/gvfs/` (the standard
/// location on systemd/GNOME Linux desktops for gvfs auto-mounts).
fn find_mtp_mount() -> Option<DeviceMount> {
    let runtime_dir = std::env::var("XDG_RUNTIME_DIR").ok()?;
    let gvfs_dir = Path::new(&runtime_dir).join("gvfs");

    for entry in std::fs::read_dir(&gvfs_dir).ok()?.flatten() {
        if entry.file_name().to_string_lossy().starts_with("mtp:") {
            let mount_path = entry.path();
            let device_name = first_subdirectory_name(&mount_path)
                .unwrap_or_else(|| "MTP Device".to_string());
            return Some(DeviceMount { kind: DeviceKind::Mtp, mount_path, device_name });
        }
    }
    None
}

/// Returns the first USB mass storage mount found under the udisks2 standard
/// locations (`/run/media/$USER/` and `/media/$USER/`), or `None` if none
/// are present. These paths are only populated with removable media, so there
/// is no risk of accidentally detecting the system disk.
fn find_usb_storage_mount() -> Option<DeviceMount> {
    let username = std::env::var("USER")
        .or_else(|_| std::env::var("LOGNAME"))
        .ok()?;
    for base_dir in [format!("/run/media/{username}"), format!("/media/{username}")] {
        if let Ok(entries) = std::fs::read_dir(&base_dir) {
            for entry in entries.flatten() {
                if entry.file_type().is_ok_and(|file_type| file_type.is_dir()) {
                    let device_name = entry.file_name().to_string_lossy().into_owned();
                    return Some(DeviceMount {
                        kind: DeviceKind::UsbStorage,
                        mount_path: entry.path(),
                        device_name,
                    });
                }
            }
        }
    }
    None
}

/// Returns the first connected device mount, preferring MTP over USB storage
/// when both are present.
pub fn find_device_mount() -> Option<DeviceMount> {
    find_mtp_mount().or_else(find_usb_storage_mount)
}

/// Returns the name of the first directory entry inside `path`, used to
/// derive a human-readable device name from the gvfs mount root.
fn first_subdirectory_name(path: &Path) -> Option<String> {
    std::fs::read_dir(path)
        .ok()?
        .flatten()
        .find(|e| e.file_type().is_ok_and(|t| t.is_dir()))
        .map(|e| e.file_name().to_string_lossy().into_owned())
}

/// Searches the MTP mount for folders named "music" (case-insensitive) using
/// an explicit breadth-first traversal rather than WalkDir.
///
/// WalkDir's `.flatten()` silently drops entire directory subtrees when a
/// `read_dir` call fails — a frequent occurrence over MTP/gvfs, which can
/// cause one storage unit (e.g. internal vs. SD card) to vanish from results
/// on any given scan. Here each `read_dir` is independent: a failure on one
/// directory does not affect its siblings, so both storage units are reliably
/// found even when MTP is intermittently unresponsive.
///
/// The search stops descending into a branch once a qualifying folder is found;
/// it explores up to 4 levels below the mount root to handle devices whose
/// storage units are nested under a device-name directory. Hidden directories
/// (starting with `.`) are skipped throughout.
pub fn find_music_folders(mount_path: &Path) -> Vec<String> {
    let mut results: Vec<String> = Vec::new();
    let mut current_level = list_subdirectories(mount_path);

    for _ in 0..4 {
        let mut next_level: Vec<PathBuf> = Vec::new();

        for directory in &current_level {
            let Ok(entries) = std::fs::read_dir(directory) else { continue };
            for entry in entries.flatten() {
                if !entry.file_type().is_ok_and(|file_type| file_type.is_dir()) {
                    continue;
                }
                let name = entry.file_name().to_string_lossy().to_lowercase();
                if name.starts_with('.') {
                    continue;
                }
                let path = entry.path();
                if name == "music" {
                    if let Ok(relative) = path.strip_prefix(mount_path) {
                        results.push(relative.to_string_lossy().into_owned());
                    }
                } else {
                    next_level.push(path);
                }
            }
        }

        // Found qualifying folders at this depth — no need to descend further.
        if !results.is_empty() {
            return results;
        }

        current_level = next_level;
    }

    results
}

/// Spawns a background thread that walks `music_path` purely to populate
/// gvfs's MTP directory cache. The first `read_dir` on an MTP path sends a
/// USB `GetObjectHandles` request to the device; gvfs caches the result so
/// subsequent reads are instant. Pre-walking on device connect means the
/// user-triggered preview scan hits the cache rather than the device.
fn spawn_gvfs_cache_warmup(music_path: PathBuf) {
    std::thread::spawn(move || {
        for _ in WalkDir::new(&music_path).into_iter().flatten() {}
    });
}

fn list_subdirectories(path: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(path) else { return Vec::new() };
    entries
        .flatten()
        .filter(|entry| {
            entry.file_type().is_ok_and(|file_type| file_type.is_dir())
                && !entry.file_name().to_string_lossy().starts_with('.')
        })
        .map(|entry| entry.path())
        .collect()
}

/// Spawns a background OS thread that polls for MTP device connects/disconnects
/// every 3 seconds and emits `device-status-changed` events on state changes.
pub fn start_detection_loop(app_handle: AppHandle) {
    std::thread::spawn(move || {
        let mut last_connected = false;

        loop {
            let mount = find_device_mount();
            let currently_connected = mount.is_some();

            if currently_connected != last_connected {
                let status = match mount {
                    Some(m) => {
                        let detected_music_subfolder = find_music_folders(&m.mount_path).into_iter().next();
                        if m.kind == DeviceKind::Mtp {
                            if let Some(ref subfolder) = detected_music_subfolder {
                                spawn_gvfs_cache_warmup(m.mount_path.join(subfolder));
                            }
                        }
                        DeviceStatus {
                            connected: true,
                            kind: m.kind,
                            device_name: m.device_name,
                            mount_path: m.mount_path.to_string_lossy().into_owned(),
                            detected_music_subfolder,
                        }
                    }
                    None => DeviceStatus::disconnected(),
                };
                let _ = app_handle.emit(DEVICE_STATUS_EVENT, status);
                last_connected = currently_connected;
            }

            std::thread::sleep(POLL_INTERVAL);
        }
    });
}
