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

pub fn sample_path_for_album(conn: &Connection, album_id: i64) -> Result<Option<String>, AppError> {
    conn.query_row("SELECT path FROM tracks WHERE album_id = ?1 LIMIT 1", [album_id], |row| {
        row.get(0)
    })
    .map(Some)
    .or_else(|e| {
        if e == rusqlite::Error::QueryReturnedNoRows {
            Ok(None)
        } else {
            Err(e)
        }
    })
    .map_err(AppError::from)
}

pub struct PlayableTrack {
    pub id: i64,
    pub path: String,
    pub duration_ms: i64,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub art_path: Option<String>,
}

fn playable_track_from(row: &rusqlite::Row) -> rusqlite::Result<PlayableTrack> {
    Ok(PlayableTrack {
        id: row.get(0)?,
        path: row.get(1)?,
        duration_ms: row.get(2)?,
        title: row.get(3)?,
        artist: row.get::<_, Option<String>>(4)?.unwrap_or_default(),
        album: row.get::<_, Option<String>>(5)?.unwrap_or_default(),
        art_path: row.get(6)?,
    })
}

const PLAYABLE_TRACK_COLUMNS: &str =
    "t.id, t.path, t.duration_ms, t.title, ar.name, al.title, al.art_path";

/// Resolves a batch of track ids to their playable info, preserving the
/// order of `track_ids` (a plain `WHERE id IN (...)` does not) since that
/// order is the playback queue order.
pub fn get_playable_batch(conn: &Connection, track_ids: &[i64]) -> Result<Vec<PlayableTrack>, AppError> {
    if track_ids.is_empty() {
        return Ok(Vec::new());
    }
    let placeholders = track_ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
    let sql = format!(
        "SELECT {PLAYABLE_TRACK_COLUMNS}
         FROM tracks t
         LEFT JOIN artists ar ON ar.id = t.track_artist_id
         LEFT JOIN albums al ON al.id = t.album_id
         WHERE t.id IN ({placeholders})"
    );
    let mut stmt = conn.prepare(&sql)?;
    let params = rusqlite::params_from_iter(track_ids.iter());
    let rows = stmt.query_map(params, playable_track_from)?;

    let mut by_id: HashMap<i64, PlayableTrack> = HashMap::new();
    for row in rows {
        let track = row?;
        by_id.insert(track.id, track);
    }

    Ok(track_ids.iter().filter_map(|id| by_id.remove(id)).collect())
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
    pub album_id: Option<i64>,
    pub duration_ms: i64,
    pub sample_rate: Option<i64>,
    pub bit_depth: Option<i64>,
    pub channels: Option<i64>,
    pub codec: Option<String>,
    pub disc_no: Option<i64>,
    pub track_no: Option<i64>,
}

const TRACK_ROW_COLUMNS: &str = "t.id, t.title, ar.name, al.title, t.album_id, t.duration_ms, \
    t.sample_rate, t.bit_depth, t.channels, t.codec, t.disc_no, t.track_no";

fn track_row_from(row: &rusqlite::Row) -> rusqlite::Result<TrackRow> {
    Ok(TrackRow {
        id: row.get(0)?,
        title: row.get(1)?,
        artist: row.get::<_, Option<String>>(2)?.unwrap_or_default(),
        album: row.get::<_, Option<String>>(3)?.unwrap_or_default(),
        album_id: row.get(4)?,
        duration_ms: row.get(5)?,
        sample_rate: row.get(6)?,
        bit_depth: row.get(7)?,
        channels: row.get(8)?,
        codec: row.get(9)?,
        disc_no: row.get(10)?,
        track_no: row.get(11)?,
    })
}

pub fn list_all(conn: &Connection) -> Result<Vec<TrackRow>, AppError> {
    let sql = format!(
        "SELECT {TRACK_ROW_COLUMNS}
         FROM tracks t
         LEFT JOIN artists ar ON ar.id = t.track_artist_id
         LEFT JOIN albums al ON al.id = t.album_id
         ORDER BY ar.sort_name, ar.name, al.title, t.disc_no, t.track_no"
    );
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map([], track_row_from)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(AppError::from)
}

pub fn list_by_album(conn: &Connection, album_id: i64) -> Result<Vec<TrackRow>, AppError> {
    let sql = format!(
        "SELECT {TRACK_ROW_COLUMNS}
         FROM tracks t
         LEFT JOIN artists ar ON ar.id = t.track_artist_id
         LEFT JOIN albums al ON al.id = t.album_id
         WHERE t.album_id = ?1
         ORDER BY t.disc_no, t.track_no"
    );
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map([album_id], track_row_from)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(AppError::from)
}
