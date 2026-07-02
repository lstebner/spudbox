use tauri::{AppHandle, State};

use crate::db::queries::settings;
use crate::device::{detection, sync, DeviceStatus, SyncMode, SyncPreview};
use crate::error::AppError;
use crate::state::AppState;

const SETTING_DEVICE_MUSIC_SUBFOLDER: &str = "device_music_subfolder";

/// Returns the current device connection state. Checks the gvfs mount
/// directory synchronously so the result is always up-to-date on first load.
#[tauri::command]
pub fn device_get_status(state: State<AppState>) -> Result<DeviceStatus, AppError> {
    match detection::find_mtp_mount() {
        Some(mount) => {
            let conn = state.db.get()?;
            let saved_subfolder = settings::get(&conn, SETTING_DEVICE_MUSIC_SUBFOLDER)?;
            let detected_music_subfolder = saved_subfolder
                .or_else(|| detection::find_music_folders(&mount.mount_path).into_iter().next());
            Ok(DeviceStatus {
                connected: true,
                device_name: mount.device_name,
                mount_path: mount.mount_path.to_string_lossy().into_owned(),
                detected_music_subfolder,
            })
        }
        None => Ok(DeviceStatus::disconnected()),
    }
}

/// Walks the device and returns all subfolder paths (relative to the mount
/// root) that look like music folders, so the frontend can present them as
/// selectable options instead of asking the user to type a path.
#[tauri::command]
pub async fn device_find_music_folders(
    state: State<'_, AppState>,
) -> Result<Vec<String>, AppError> {
    let db = state.db.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let mount = detection::find_mtp_mount()
            .ok_or_else(|| AppError::Device("no MTP device connected".to_string()))?;
        // Surface any previously saved choice at the top so the UI can
        // pre-select it, then append any additionally detected folders.
        let conn = db.get()?;
        let saved = settings::get(&conn, SETTING_DEVICE_MUSIC_SUBFOLDER)?;
        drop(conn);

        let mut folders = detection::find_music_folders(&mount.mount_path);
        if let Some(saved) = saved {
            let saved_path = mount.mount_path.join(&saved);
            // Only surface the saved path if it is not already in the scan results
            // AND it actually exists on the device right now. Injecting a stale
            // path that the device no longer exposes (e.g. SD card removed, or the
            // path format changed between sessions) would silently cause a wrong
            // folder to be pre-selected and fail the free-space check.
            if !saved.is_empty() && !folders.contains(&saved) && saved_path.exists() {
                folders.insert(0, saved);
            }
        }
        Ok(folders)
    })
    .await
    .expect("folder scan task panicked")
}

/// Persists the user's chosen music subfolder path (relative to the device
/// mount root) so it is remembered across sessions.
#[tauri::command]
pub fn device_save_music_subfolder(
    state: State<AppState>,
    subfolder: String,
) -> Result<(), AppError> {
    let conn = state.db.get()?;
    settings::set(&conn, SETTING_DEVICE_MUSIC_SUBFOLDER, &subfolder)
}

/// Compares the library against the device music folder and returns the
/// list of files to add and remove.
#[tauri::command]
pub async fn device_preview_sync(
    state: State<'_, AppState>,
    app: AppHandle,
    music_subfolder: String,
) -> Result<SyncPreview, AppError> {
    let db = state.db.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let mount = detection::find_mtp_mount()
            .ok_or_else(|| AppError::Device("no MTP device connected".to_string()))?;
        let device_music_path = mount.mount_path.join(&music_subfolder);
        sync::preview_sync(&device_music_path, &db, &app)
    })
    .await
    .expect("preview task panicked")
}

/// Performs the sync (copy additions, and optionally delete removed tracks).
/// Emits `device-sync-progress` events throughout.
#[tauri::command]
pub async fn device_perform_sync(
    state: State<'_, AppState>,
    app: AppHandle,
    music_subfolder: String,
    mode: SyncMode,
) -> Result<(), AppError> {
    let db = state.db.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let mount = detection::find_mtp_mount()
            .ok_or_else(|| AppError::Device("no MTP device connected".to_string()))?;
        let device_music_path = mount.mount_path.join(&music_subfolder);
        sync::perform_sync(device_music_path, mode, db, app)
    })
    .await
    .expect("sync task panicked")
}
