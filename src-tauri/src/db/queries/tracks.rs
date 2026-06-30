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
            date_modified_db = excluded.date_modified_db,
            is_archived = 0
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

/// Loads `path -> (file_mtime, file_size)` for every active (non-archived) track,
/// used by the scanner to skip re-parsing unchanged files and to detect deleted
/// files. Archived tracks are excluded so the scanner does not treat them as
/// "missing from disk" and delete them.
pub fn fingerprints(conn: &Connection) -> Result<HashMap<String, (i64, i64)>, AppError> {
    let mut stmt = conn.prepare("SELECT path, file_mtime, file_size FROM tracks WHERE is_archived = 0")?;
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
    pub album_id: Option<i64>,
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
        album_id: row.get(6)?,
        art_path: row.get(7)?,
    })
}

const PLAYABLE_TRACK_COLUMNS: &str =
    "t.id, t.path, t.duration_ms, t.title, ar.name, al.title, al.id, al.art_path";

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
         WHERE t.is_archived = 0
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
         WHERE t.album_id = ?1 AND t.is_archived = 0
         ORDER BY t.disc_no, t.track_no"
    );
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map([album_id], track_row_from)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(AppError::from)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::queries::{albums, artists};
    use crate::db::schema::test_connection;

    /// Tracks reference real artists/albums rows: the bundled SQLite build
    /// compiles with `SQLITE_DEFAULT_FOREIGN_KEYS=1`, so FK constraints are
    /// enforced even on a bare test connection that never runs `PRAGMA
    /// foreign_keys`.
    fn setup_artist_and_album(conn: &Connection) -> (i64, i64) {
        let artist_id = artists::upsert(conn, "Thrice").unwrap();
        let album_id = albums::upsert(conn, "Vheissu", artist_id, Some(2005)).unwrap();
        (artist_id, album_id)
    }

    fn sample_track<'a>(path: &'a str, title: &'a str, artist_id: i64, album_id: i64, track_no: u32) -> NewTrack<'a> {
        NewTrack {
            path,
            folder_path: "/music/album",
            title,
            track_artist_id: artist_id,
            album_id,
            genre_id: None,
            track_no: Some(track_no),
            disc_no: Some(1),
            duration_ms: 200_000,
            sample_rate: Some(44100),
            bit_depth: Some(16),
            channels: Some(2),
            codec: "flac",
            bitrate_kbps: None,
            file_size: 1_000_000,
            file_mtime: 1000,
            now: 1000,
        }
    }

    fn track_count(conn: &Connection) -> i64 {
        conn.query_row("SELECT COUNT(*) FROM tracks", [], |row| row.get(0)).unwrap()
    }

    #[test]
    fn upsert_inserts_a_new_track() {
        let conn = test_connection();
        let (artist_id, album_id) = setup_artist_and_album(&conn);
        let id = upsert(&conn, &sample_track("/music/a.flac", "Image of the Invisible", artist_id, album_id, 1)).unwrap();
        assert!(id > 0);
        assert_eq!(track_count(&conn), 1);
    }

    #[test]
    fn upsert_on_the_same_path_updates_in_place_instead_of_duplicating() {
        let conn = test_connection();
        let (artist_id, album_id) = setup_artist_and_album(&conn);
        let mut t = sample_track("/music/a.flac", "Image of the Invisible", artist_id, album_id, 1);
        let first_id = upsert(&conn, &t).unwrap();

        t.duration_ms = 999_000;
        let second_id = upsert(&conn, &t).unwrap();

        assert_eq!(first_id, second_id);
        assert_eq!(track_count(&conn), 1);
        let duration: i64 = conn
            .query_row("SELECT duration_ms FROM tracks WHERE id = ?1", [first_id], |row| row.get(0))
            .unwrap();
        assert_eq!(duration, 999_000);
    }

    #[test]
    fn fingerprints_reflects_inserted_mtime_and_size() {
        let conn = test_connection();
        let (artist_id, album_id) = setup_artist_and_album(&conn);
        let mut t = sample_track("/music/a.flac", "Image of the Invisible", artist_id, album_id, 1);
        t.file_mtime = 12345;
        t.file_size = 67890;
        upsert(&conn, &t).unwrap();

        let fps = fingerprints(&conn).unwrap();
        assert_eq!(fps.get("/music/a.flac"), Some(&(12345, 67890)));
    }

    #[test]
    fn sample_path_for_album_finds_a_track_or_none() {
        let conn = test_connection();
        let (artist_id, album_id) = setup_artist_and_album(&conn);
        upsert(&conn, &sample_track("/music/a.flac", "Image of the Invisible", artist_id, album_id, 1)).unwrap();

        assert_eq!(sample_path_for_album(&conn, album_id).unwrap(), Some("/music/a.flac".to_string()));
        assert_eq!(sample_path_for_album(&conn, 999).unwrap(), None);
    }

    #[test]
    fn get_playable_batch_preserves_requested_order_not_insertion_order() {
        let conn = test_connection();
        let (artist_id, album_id) = setup_artist_and_album(&conn);
        let id_a = upsert(&conn, &sample_track("/music/a.flac", "Track A", artist_id, album_id, 1)).unwrap();
        let id_b = upsert(&conn, &sample_track("/music/b.flac", "Track B", artist_id, album_id, 2)).unwrap();
        let id_c = upsert(&conn, &sample_track("/music/c.flac", "Track C", artist_id, album_id, 3)).unwrap();

        // Deliberately out of insertion order, and skips id_b entirely.
        let playable = get_playable_batch(&conn, &[id_c, id_a]).unwrap();
        let ids: Vec<i64> = playable.iter().map(|t| t.id).collect();
        assert_eq!(ids, vec![id_c, id_a]);
        let _ = id_b;
    }

    #[test]
    fn get_playable_batch_silently_skips_missing_ids() {
        let conn = test_connection();
        let (artist_id, album_id) = setup_artist_and_album(&conn);
        let id_a = upsert(&conn, &sample_track("/music/a.flac", "Track A", artist_id, album_id, 1)).unwrap();

        let playable = get_playable_batch(&conn, &[id_a, 99999]).unwrap();
        assert_eq!(playable.len(), 1);
        assert_eq!(playable[0].id, id_a);
    }

    #[test]
    fn delete_by_paths_removes_only_the_given_paths() {
        let conn = test_connection();
        let (artist_id, album_id) = setup_artist_and_album(&conn);
        upsert(&conn, &sample_track("/music/a.flac", "Track A", artist_id, album_id, 1)).unwrap();
        upsert(&conn, &sample_track("/music/b.flac", "Track B", artist_id, album_id, 2)).unwrap();

        let deleted = delete_by_paths(&conn, &["/music/a.flac".to_string()]).unwrap();
        assert_eq!(deleted, 1);
        assert_eq!(track_count(&conn), 1);
    }

    #[test]
    fn fingerprints_excludes_archived_tracks() {
        let conn = test_connection();
        let (artist_id, album_id) = setup_artist_and_album(&conn);
        upsert(&conn, &sample_track("/music/a.flac", "Track A", artist_id, album_id, 1)).unwrap();
        upsert(&conn, &sample_track("/music/b.flac", "Track B", artist_id, album_id, 2)).unwrap();
        conn.execute("UPDATE tracks SET is_archived = 1 WHERE path = '/music/b.flac'", []).unwrap();

        let fps = fingerprints(&conn).unwrap();
        assert!(fps.contains_key("/music/a.flac"));
        assert!(!fps.contains_key("/music/b.flac"), "archived track must not appear in fingerprints");
    }

    #[test]
    fn upsert_restores_archived_track() {
        let conn = test_connection();
        let (artist_id, album_id) = setup_artist_and_album(&conn);
        let id = upsert(&conn, &sample_track("/music/a.flac", "Track A", artist_id, album_id, 1)).unwrap();
        conn.execute("UPDATE tracks SET is_archived = 1 WHERE id = ?1", [id]).unwrap();

        // Upserting the same path should restore is_archived = 0
        let id2 = upsert(&conn, &sample_track("/music/a.flac", "Track A", artist_id, album_id, 1)).unwrap();
        assert_eq!(id, id2);
        let archived: i64 = conn
            .query_row("SELECT is_archived FROM tracks WHERE id = ?1", [id], |r| r.get(0))
            .unwrap();
        assert_eq!(archived, 0, "upsert must clear is_archived");
    }

    #[test]
    fn list_by_album_excludes_archived_tracks() {
        let conn = test_connection();
        let (artist_id, album_id) = setup_artist_and_album(&conn);
        upsert(&conn, &sample_track("/music/a.flac", "Track A", artist_id, album_id, 1)).unwrap();
        let id_b = upsert(&conn, &sample_track("/music/b.flac", "Track B", artist_id, album_id, 2)).unwrap();
        conn.execute("UPDATE tracks SET is_archived = 1 WHERE id = ?1", [id_b]).unwrap();

        let rows = list_by_album(&conn, album_id).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].title, "Track A");
    }

    #[test]
    fn list_by_album_orders_by_disc_then_track_number() {
        let conn = test_connection();
        let (artist_id, album_id) = setup_artist_and_album(&conn);
        upsert(&conn, &sample_track("/music/3.flac", "Three", artist_id, album_id, 3)).unwrap();
        upsert(&conn, &sample_track("/music/1.flac", "One", artist_id, album_id, 1)).unwrap();
        upsert(&conn, &sample_track("/music/2.flac", "Two", artist_id, album_id, 2)).unwrap();

        let rows = list_by_album(&conn, album_id).unwrap();
        let titles: Vec<&str> = rows.iter().map(|r| r.title.as_str()).collect();
        assert_eq!(titles, vec!["One", "Two", "Three"]);
    }
}
