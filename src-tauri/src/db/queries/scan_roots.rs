use rusqlite::Connection;

use crate::error::AppError;

pub fn add(conn: &Connection, path: &str) -> Result<(), AppError> {
    conn.execute(
        "INSERT INTO scan_roots (path, enabled) VALUES (?1, 1)
         ON CONFLICT(path) DO UPDATE SET enabled = 1",
        [path],
    )?;
    Ok(())
}

pub fn list_enabled(conn: &Connection) -> Result<Vec<String>, AppError> {
    let mut stmt = conn.prepare("SELECT path FROM scan_roots WHERE enabled = 1")?;
    let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
    rows.collect::<Result<Vec<_>, _>>().map_err(AppError::from)
}

pub fn has_enabled(conn: &Connection) -> Result<bool, AppError> {
    let count: i64 = conn.query_row("SELECT COUNT(*) FROM scan_roots WHERE enabled = 1", [], |row| row.get(0))?;
    Ok(count > 0)
}
