use rusqlite::{params, Connection};

use crate::error::AppError;

pub fn upsert(
    conn: &Connection,
    title: &str,
    album_artist_id: i64,
    year: Option<i64>,
) -> Result<i64, AppError> {
    let id: i64 = conn.query_row(
        "INSERT INTO albums (title, album_artist_id, year) VALUES (?1, ?2, ?3)
         ON CONFLICT(title, album_artist_id, year) DO UPDATE SET title = excluded.title
         RETURNING id",
        params![title, album_artist_id, year],
        |row| row.get(0),
    )?;
    Ok(id)
}
