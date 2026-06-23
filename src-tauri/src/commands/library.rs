use std::path::PathBuf;

use tauri::{AppHandle, Emitter, State};

use crate::db::queries::{scan_roots, tracks};
use crate::error::AppError;
use crate::events::{ScanProgress, SCAN_PROGRESS};
use crate::scanner::{self, ScanStats};
use crate::state::AppState;

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
pub fn library_scan(state: State<AppState>, app: AppHandle) -> Result<ScanStats, AppError> {
    let roots: Vec<PathBuf> = {
        let conn = state.db.get()?;
        scan_roots::list_enabled(&conn)?
            .into_iter()
            .map(PathBuf::from)
            .collect()
    };

    scanner::full_scan(&state.db, &roots, |scanned, total| {
        let _ = app.emit(SCAN_PROGRESS, ScanProgress { scanned, total });
    })
}

#[tauri::command]
pub fn library_get_tracks(state: State<AppState>) -> Result<Vec<tracks::TrackRow>, AppError> {
    let conn = state.db.get()?;
    tracks::list_all(&conn)
}
