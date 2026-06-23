use rusqlite::Connection;
use serde::Serialize;

use crate::error::AppError;

#[derive(Debug, Serialize)]
pub struct ArtistRow {
    pub id: i64,
    pub name: String,
    pub album_count: i64,
}

/// Only artists who appear as an album artist (not every track-level
/// artist, e.g. a one-off featured artist), since this list drives the
/// sidebar's album-browsing navigation.
pub fn list_album_artists(conn: &Connection) -> Result<Vec<ArtistRow>, AppError> {
    let mut stmt = conn.prepare(
        "SELECT ar.id, ar.name, COUNT(al.id) as album_count
         FROM artists ar
         JOIN albums al ON al.album_artist_id = ar.id
         GROUP BY ar.id
         ORDER BY ar.sort_name, ar.name",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(ArtistRow {
            id: row.get(0)?,
            name: row.get(1)?,
            album_count: row.get(2)?,
        })
    })?;
    rows.collect::<Result<Vec<_>, _>>().map_err(AppError::from)
}

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
