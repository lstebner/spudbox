use rusqlite::{params, Connection};
use serde::Serialize;

use crate::error::AppError;

#[derive(Debug, Serialize)]
pub struct AlbumRow {
    pub id: i64,
    pub title: String,
    pub album_artist: String,
    pub album_artist_id: i64,
    pub year: Option<i64>,
    pub art_path: Option<String>,
}

pub fn list_all(conn: &Connection, artist_id: Option<i64>) -> Result<Vec<AlbumRow>, AppError> {
    let sql = "SELECT al.id, al.title, ar.name, al.album_artist_id, al.year, al.art_path
         FROM albums al
         LEFT JOIN artists ar ON ar.id = al.album_artist_id
         WHERE ?1 IS NULL OR al.album_artist_id = ?1
         ORDER BY ar.sort_name, ar.name, al.year, al.title";
    let mut stmt = conn.prepare(sql)?;
    let rows = stmt.query_map([artist_id], |row| {
        Ok(AlbumRow {
            id: row.get(0)?,
            title: row.get(1)?,
            album_artist: row.get::<_, Option<String>>(2)?.unwrap_or_default(),
            album_artist_id: row.get(3)?,
            year: row.get(4)?,
            art_path: row.get(5)?,
        })
    })?;
    rows.collect::<Result<Vec<_>, _>>().map_err(AppError::from)
}

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

/// Albums that have never had art extraction attempted (`art_source` is
/// only ever NULL before the first attempt, then 'embedded'/'folder'/'none').
pub fn list_missing_art(conn: &Connection) -> Result<Vec<i64>, AppError> {
    let mut stmt = conn.prepare("SELECT id FROM albums WHERE art_source IS NULL")?;
    let rows = stmt.query_map([], |row| row.get::<_, i64>(0))?;
    rows.collect::<Result<Vec<_>, _>>().map_err(AppError::from)
}

pub fn set_art(conn: &Connection, album_id: i64, art_path: Option<&str>, art_source: &str) -> Result<(), AppError> {
    conn.execute(
        "UPDATE albums SET art_path = ?1, art_source = ?2 WHERE id = ?3",
        params![art_path, art_source, album_id],
    )?;
    Ok(())
}
