use std::path::PathBuf;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};

use crate::db::queries::{album_ratings, albums, artists, hidden_albums, scan_roots, tracks};
use crate::sync::{SyncConfig, SyncManager};
use crate::error::AppError;
use crate::events::{ScanProgress, SCAN_PROGRESS};
use crate::scanner::art::ArtStats;
use crate::scanner::{self, ScanStats};
use crate::state::AppState;

#[derive(Debug, Serialize)]
pub struct ScanResult {
    pub library: ScanStats,
    pub art: ArtStats,
}

#[tauri::command]
pub fn ping(state: State<AppState>) -> Result<String, AppError> {
    let conn = state.db.get()?;
    let value: i64 = conn.query_row("SELECT 1", [], |row| row.get(0))?;
    Ok(format!("pong ({value})"))
}

#[tauri::command]
pub fn library_add_root(state: State<AppState>, path: String) -> Result<(), AppError> {
    let conn = state.db.get()?;
    scan_roots::add(&conn, &path)?;
    Ok(())
}

#[tauri::command]
pub fn library_has_roots(state: State<AppState>) -> Result<bool, AppError> {
    let conn = state.db.get()?;
    scan_roots::has_enabled(&conn)
}

#[tauri::command]
pub fn library_list_roots(state: State<AppState>) -> Result<Vec<String>, AppError> {
    let conn = state.db.get()?;
    scan_roots::list_enabled(&conn)
}

#[tauri::command]
pub fn library_remove_root(state: State<AppState>, path: String, keep_stats: bool) -> Result<(), AppError> {
    let conn = state.db.get()?;
    scan_roots::remove(&conn, &path, keep_stats)
}

/// Async so the genuinely slow work (a fresh scan of thousands of files, or
/// first-time art backfill doing hundreds of image decode/resize/encode
/// round trips) runs on Tauri's dedicated blocking-task pool rather than
/// inline on whatever thread received the IPC call — on Linux that's the
/// GTK main thread, and a plain (non-async) `#[tauri::command]` runs there
/// directly, freezing the whole window for as long as the work takes.
#[tauri::command]
pub async fn library_scan(state: State<'_, AppState>, app: AppHandle) -> Result<ScanResult, AppError> {
    let db = state.db.clone();

    tauri::async_runtime::spawn_blocking(move || {
        let roots: Vec<PathBuf> = {
            let conn = db.get()?;
            scan_roots::list_enabled(&conn)?
                .into_iter()
                .map(PathBuf::from)
                .collect()
        };

        eprintln!("[scan] starting full_scan over {} root(s)", roots.len());
        let progress_app = app.clone();
        let library = scanner::full_scan(&db, &roots, move |scanned, total| {
            let _ = progress_app.emit(SCAN_PROGRESS, ScanProgress { scanned, total });
        })?;
        eprintln!("[scan] full_scan done: {library:?}");

        let cache_dir = app.path().app_cache_dir()?.join("art");
        eprintln!("[scan] starting art backfill into {cache_dir:?}");
        let art = scanner::art::backfill_album_art(&db, &cache_dir)?;
        eprintln!("[scan] art backfill done: {art:?}");

        Ok(ScanResult { library, art })
    })
    .await
    .expect("scan task panicked")
}

#[tauri::command]
pub fn library_get_tracks(state: State<AppState>) -> Result<Vec<tracks::TrackRow>, AppError> {
    let conn = state.db.get()?;
    tracks::list_all(&conn)
}

#[tauri::command]
pub fn library_get_tracks_by_album(
    state: State<AppState>,
    album_id: i64,
) -> Result<Vec<tracks::TrackRow>, AppError> {
    let conn = state.db.get()?;
    tracks::list_by_album(&conn, album_id)
}

#[tauri::command]
pub fn library_get_artists(state: State<AppState>) -> Result<Vec<artists::ArtistRow>, AppError> {
    let conn = state.db.get()?;
    artists::list_album_artists(&conn)
}

#[tauri::command]
pub fn library_get_albums(
    state: State<AppState>,
    artist_id: Option<i64>,
    hidden_only: Option<bool>,
) -> Result<Vec<albums::AlbumRow>, AppError> {
    let conn = state.db.get()?;
    albums::list_all(&conn, artist_id, hidden_only.unwrap_or(false))
}

#[tauri::command]
pub fn library_set_album_hidden(
    state: State<AppState>,
    album_id: i64,
    hidden: bool,
) -> Result<(), AppError> {
    let conn = state.db.get()?;
    if hidden {
        hidden_albums::hide(&conn, album_id)
    } else {
        hidden_albums::unhide(&conn, album_id)
    }
}

#[tauri::command]
pub fn library_set_album_rating(
    state: State<AppState>,
    album_id: i64,
    rating: Option<f64>,
) -> Result<(), AppError> {
    let conn = state.db.get()?;
    let updated_at = album_ratings::set_rating(&conn, album_id, rating)?;

    // Fire-and-forget push to cloud — if it fails (offline, unconfigured),
    // the next startup sync will catch it via the updated_at timestamp.
    if let (Ok(Some(key)), Ok(Some(config))) = (
        albums::get_natural_key(&conn, album_id),
        SyncConfig::from_db(&conn),
    ) {
        let db = state.db.clone();
        tauri::async_runtime::spawn(async move {
            let manager = SyncManager::new(config, db);
            if let Err(e) = manager.push_one_album_rating(
                &key.title,
                &key.artist,
                key.year,
                rating,
                updated_at,
            )
            .await
            {
                eprintln!("[sync] push_one_album_rating failed: {e}");
            }
        });
    }
    Ok(())
}
