use std::panic::AssertUnwindSafe;
use std::sync::atomic::Ordering;

use tauri::{AppHandle, Emitter, State};

use crate::db::queries::settings;
use crate::device::{detection, sync, DeviceStatus, SyncMode, SyncPreview, SyncResult};
use crate::error::AppError;
use crate::state::AppState;

const SETTING_DEVICE_MUSIC_SUBFOLDER: &str = "device_music_subfolder";

/// Returns the current device connection state. Runs in a blocking task so
/// that the gvfs `read_dir` calls do not execute on the main GTK/webview thread,
/// which would freeze the window while waiting for a sleeping MTP device.
#[tauri::command]
pub async fn device_get_status(state: State<'_, AppState>) -> Result<DeviceStatus, AppError> {
    let db = state.db.clone();
    tauri::async_runtime::spawn_blocking(move || {
        match detection::find_device_mount() {
            Some(mount) => {
                let conn = db.get()?;
                let saved_subfolder = settings::get(&conn, SETTING_DEVICE_MUSIC_SUBFOLDER)?;
                let detected_music_subfolder = saved_subfolder
                    .or_else(|| detection::find_music_folders(&mount.mount_path).into_iter().next());
                Ok(DeviceStatus {
                    connected: true,
                    kind: mount.kind,
                    device_name: mount.device_name,
                    mount_path: mount.mount_path.to_string_lossy().into_owned(),
                    detected_music_subfolder,
                })
            }
            None => Ok(DeviceStatus::disconnected()),
        }
    })
    .await
    .unwrap_or_else(|_| Err(AppError::Device("device status check failed".to_string())))
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
        let mount = detection::find_device_mount()
            .ok_or_else(|| AppError::Device("no device connected".to_string()))?;
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
    .unwrap_or_else(|_| Err(AppError::Device("folder scan task failed".to_string())))
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
/// list of files to add and remove. Returns `AppError::Cancelled` if
/// `device_cancel_preview` is called while the device walk is in progress.
/// Returns an error immediately if a preview is already running.
#[tauri::command]
pub async fn device_preview_sync(
    state: State<'_, AppState>,
    app: AppHandle,
    music_subfolder: String,
) -> Result<SyncPreview, AppError> {
    let preview_running = state.device_preview_running.clone();
    if preview_running
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        return Err(AppError::Device("a preview is already in progress".to_string()));
    }
    let cancel = state.device_preview_cancel.clone();
    cancel.store(false, Ordering::SeqCst);
    let db = state.db.clone();

    let result = tauri::async_runtime::spawn_blocking(move || {
        std::panic::catch_unwind(AssertUnwindSafe(|| {
            let mount = detection::find_device_mount()
                .ok_or_else(|| AppError::Device("no device connected".to_string()))?;
            let device_music_path = mount.mount_path.join(&music_subfolder);
            let mount_path = mount.mount_path.to_string_lossy().into_owned();
            sync::preview_sync(&device_music_path, mount_path, &db, &app, &cancel)
        }))
        .unwrap_or_else(|_| Err(AppError::Device("preview task panicked".to_string())))
    })
    .await
    .unwrap_or_else(|_| Err(AppError::Device("preview task failed to complete".to_string())));

    preview_running.store(false, Ordering::SeqCst);
    result
}

/// Signals a running preview scan to stop at the next file boundary.
/// No-op if no preview is in progress.
#[tauri::command]
pub fn device_cancel_preview(state: State<AppState>) {
    state.device_preview_cancel.store(true, Ordering::SeqCst);
}

/// Performs the sync (copy additions, and optionally delete removed tracks)
/// using the preview already computed by the caller, so the slow device walk
/// does not repeat. Emits `device-sync-started` when the guard passes,
/// `device-sync-ended` when done (success, failure, cancellation, or panic),
/// and `device-sync-progress` throughout. Returns an error immediately if a
/// sync is already running. Resets the cancel flag at the start of every run.
#[tauri::command]
pub async fn device_perform_sync(
    state: State<'_, AppState>,
    app: AppHandle,
    music_subfolder: String,
    mode: SyncMode,
    preview: SyncPreview,
) -> Result<SyncResult, AppError> {
    let sync_running = state.device_sync_running.clone();
    if sync_running
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        return Err(AppError::Device("a sync is already in progress".to_string()));
    }
    let _ = app.emit("device-sync-started", ());
    let cancel = state.device_sync_cancel.clone();
    cancel.store(false, Ordering::SeqCst);
    let db = state.db.clone();
    let app_for_sync = app.clone();

    // `catch_unwind` ensures the closure never panics out, so `.await` always
    // resolves to `Ok` and `sync_running` / `device-sync-ended` are guaranteed
    // to fire even if the sync body panics.
    let result = tauri::async_runtime::spawn_blocking(move || {
        std::panic::catch_unwind(AssertUnwindSafe(|| {
            let mount = detection::find_device_mount()
                .ok_or_else(|| AppError::Device("no device connected".to_string()))?;
            if mount.mount_path.to_string_lossy() != preview.device_mount_path {
                return Err(AppError::Device(
                    "connected device changed since preview — re-run preview before syncing"
                        .to_string(),
                ));
            }
            let device_music_path = mount.mount_path.join(&music_subfolder);
            sync::perform_sync(device_music_path, mount.kind, mode, preview, db, app_for_sync, cancel)
        }))
        .unwrap_or_else(|_| Err(AppError::Device("sync task panicked".to_string())))
    })
    .await
    .unwrap_or_else(|_| Err(AppError::Device("sync task failed to complete".to_string())));

    sync_running.store(false, Ordering::SeqCst);
    let _ = app.emit("device-sync-ended", ());
    result
}

/// Signals a running sync to stop cleanly between file operations.
/// No-op if no sync is in progress.
#[tauri::command]
pub fn device_cancel_sync(state: State<AppState>) {
    state.device_sync_cancel.store(true, Ordering::SeqCst);
}
