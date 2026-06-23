use std::path::Path;

use r2d2_sqlite::SqliteConnectionManager;

use crate::error::AppError;
use crate::state::DbPool;

pub fn create_pool(db_path: &Path) -> Result<DbPool, AppError> {
    let manager = SqliteConnectionManager::file(db_path)
        .with_init(|conn| conn.execute_batch("PRAGMA journal_mode = WAL; PRAGMA foreign_keys = ON;"));
    let pool = r2d2::Pool::new(manager)?;
    Ok(pool)
}
