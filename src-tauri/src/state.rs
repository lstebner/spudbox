use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use r2d2_sqlite::SqliteConnectionManager;

use crate::audio::PlayerHandle;

pub type DbPool = r2d2::Pool<SqliteConnectionManager>;

pub struct AppState {
    pub db: DbPool,
    pub player: PlayerHandle,
    /// Set to `true` for the duration of a device sync. Checked atomically
    /// before starting a new sync so concurrent calls are rejected immediately.
    pub device_sync_running: Arc<AtomicBool>,
    /// Set to `true` to request that the running sync stop between file
    /// operations. Cleared to `false` at the start of every new sync.
    pub device_sync_cancel: Arc<AtomicBool>,
}
