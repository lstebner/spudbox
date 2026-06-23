use rusqlite::Connection;

use crate::error::AppError;

pub fn upsert(conn: &Connection, name: &str) -> Result<i64, AppError> {
    let id: i64 = conn.query_row(
        "INSERT INTO artists (name) VALUES (?1)
         ON CONFLICT(name) DO UPDATE SET name = excluded.name
         RETURNING id",
        [name],
        |row| row.get(0),
    )?;
    Ok(id)
}
