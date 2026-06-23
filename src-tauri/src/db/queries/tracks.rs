use std::collections::HashMap;

use rusqlite::{params, Connection};
use serde::Serialize;

use crate::error::AppError;

pub struct NewTrack<'a> {
    pub path: &'a str,
    pub folder_path: &'a str,
    pub title: &'a str,
    pub track_artist_id: i64,
    pub album_id: i64,
    pub genre_id: Option<i64>,
    pub track_no: Option<u32>,
    pub disc_no: Option<u32>,
    pub duration_ms: i64,
    pub sample_rate: Option<u32>,
    pub bit_depth: Option<u8>,
    pub channels: Option<u8>,
    pub codec: &'a str,
    pub bitrate_kbps: Option<u32>,
    pub file_size: i64,
    pub file_mtime: i64,
    pub now: i64,
}

pub fn upsert(conn: &Connection, t: &NewTrack) -> Result<i64, AppError> {
    let id: i64 = conn.query_row(
        "INSERT INTO tracks (
            path, folder_path, title, track_artist_id, album_id, genre_id,
            track_no, disc_no, duration_ms, sample_rate, bit_depth, channels,
            codec, bitrate_kbps, file_size, file_mtime, date_added, date_modified_db
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        ON CONFLICT(path) DO UPDATE SET
            folder_path = excluded.folder_path,
            title = excluded.title,
            track_artist_id = excluded.track_artist_id,
            album_id = excluded.album_id,
            genre_id = excluded.genre_id,
            track_no = excluded.track_no,
            disc_no = excluded.disc_no,
            duration_ms = excluded.duration_ms,
            sample_rate = excluded.sample_rate,
            bit_depth = excluded.bit_depth,
            channels = excluded.channels,
            codec = excluded.codec,
            bitrate_kbps = excluded.bitrate_kbps,
            file_size = excluded.file_size,
            file_mtime = excluded.file_mtime,
            date_modified_db = excluded.date_modified_db
        RETURNING id",
        params![
            t.path,
            t.folder_path,
            t.title,
            t.track_artist_id,
            t.album_id,
            t.genre_id,
            t.track_no,
            t.disc_no,
            t.duration_ms,
            t.sample_rate,
            t.bit_depth,
            t.channels,
            t.codec,
            t.bitrate_kbps,
            t.file_size,
            t.file_mtime,
            t.now,
            t.now
        ],
        |row| row.get(0),
    )?;
    Ok(id)
}

/// Loads `path -> (file_mtime, file_size)` for every known track, used to skip
/// re-parsing tags for files that haven't changed since the last scan.
pub fn fingerprints(conn: &Connection) -> Result<HashMap<String, (i64, i64)>, AppError> {
    let mut stmt = conn.prepare("SELECT path, file_mtime, file_size FROM tracks")?;
    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            (row.get::<_, i64>(1)?, row.get::<_, i64>(2)?),
        ))
    })?;
    let mut map = HashMap::new();
    for row in rows {
        let (path, fp) = row?;
        map.insert(path, fp);
    }
    Ok(map)
}

pub fn delete_by_paths(conn: &Connection, paths: &[String]) -> Result<usize, AppError> {
    let mut deleted = 0;
    for path in paths {
        deleted += conn.execute("DELETE FROM tracks WHERE path = ?1", [path])?;
    }
    Ok(deleted)
}

#[derive(Debug, Serialize)]
pub struct TrackRow {
    pub id: i64,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub duration_ms: i64,
    pub sample_rate: Option<i64>,
    pub bit_depth: Option<i64>,
    pub channels: Option<i64>,
    pub codec: Option<String>,
    pub track_no: Option<i64>,
}

pub fn list_all(conn: &Connection) -> Result<Vec<TrackRow>, AppError> {
    let mut stmt = conn.prepare(
        "SELECT t.id, t.title, ar.name, al.title, t.duration_ms, t.sample_rate, t.bit_depth, t.channels, t.codec, t.track_no
         FROM tracks t
         LEFT JOIN artists ar ON ar.id = t.track_artist_id
         LEFT JOIN albums al ON al.id = t.album_id
         ORDER BY ar.sort_name, ar.name, al.title, t.disc_no, t.track_no",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(TrackRow {
            id: row.get(0)?,
            title: row.get(1)?,
            artist: row.get::<_, Option<String>>(2)?.unwrap_or_default(),
            album: row.get::<_, Option<String>>(3)?.unwrap_or_default(),
            duration_ms: row.get(4)?,
            sample_rate: row.get(5)?,
            bit_depth: row.get(6)?,
            channels: row.get(7)?,
            codec: row.get(8)?,
            track_no: row.get(9)?,
        })
    })?;
    rows.collect::<Result<Vec<_>, _>>().map_err(AppError::from)
}
