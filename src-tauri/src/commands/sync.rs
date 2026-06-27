use serde::Serialize;
use tauri::State;

use crate::db::queries::settings;
use crate::error::AppError;
use crate::state::AppState;
use crate::sync::{SyncConfig, SyncError, SyncManager, SyncStats};

#[derive(Debug, Serialize)]
pub struct SyncStatusResponse {
    pub configured: bool,
    pub machine_id: String,
}

/// One-time setup: persist the Turso database URL and auth token.
#[tauri::command]
pub fn sync_configure(state: State<AppState>, db_url: String, token: String) -> Result<(), AppError> {
    let conn = state.db.get()?;
    settings::set(&conn, "sync_db_url", &db_url)?;
    settings::set(&conn, "sync_token", &token)?;
    Ok(())
}

/// Returns whether sync is configured and this machine's identifier.
#[tauri::command]
pub fn sync_status(state: State<AppState>) -> Result<SyncStatusResponse, AppError> {
    let conn = state.db.get()?;
    let machine_id = settings::ensure_machine_id(&conn)?;
    let db_url = settings::get(&conn, "sync_db_url")?;
    let token = settings::get(&conn, "sync_token")?;
    let configured = matches!((db_url, token), (Some(u), Some(t)) if !u.is_empty() && !t.is_empty());
    Ok(SyncStatusResponse { configured, machine_id })
}

/// Manually triggers a full push+pull sync cycle.
#[tauri::command]
pub async fn sync_now(state: State<'_, AppState>) -> Result<SyncStats, SyncError> {
    let config = {
        let conn = state.db.get().map_err(|e| SyncError::Db(e.to_string()))?;
        SyncConfig::from_db(&conn).map_err(|e| SyncError::Db(e.to_string()))?
    };
    let Some(config) = config else {
        return Err(SyncError::Api("sync is not configured".to_string()));
    };
    SyncManager::new(config, state.db.clone()).sync().await
}
