use rusqlite::Connection;
use serde::Serialize;

use crate::error::AppError;

/// Orders by artist name (falling back to the not-yet-populated `sort_name`
/// column when set), case-insensitively, with any name starting in a digit
/// or symbol sorting before alphabetic names — matches how a media library
/// is conventionally browsed, rather than SQLite's default byte-wise
/// comparison, which sorts all uppercase letters before any lowercase one.
pub const ARTIST_NAME_ORDER_BY: &str = "\
    CASE WHEN COALESCE(ar.sort_name, ar.name) GLOB '[A-Za-z]*' THEN 1 ELSE 0 END, \
    COALESCE(ar.sort_name, ar.name) COLLATE NOCASE";

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
    let sql = format!(
        "SELECT ar.id, ar.name, COUNT(al.id) as album_count
         FROM artists ar
         JOIN albums al ON al.album_artist_id = ar.id
         LEFT JOIN hidden_albums ha ON ha.album_id = al.id
         WHERE ha.album_id IS NULL
           AND EXISTS (SELECT 1 FROM tracks WHERE album_id = al.id AND is_archived = 0)
         GROUP BY ar.id
         HAVING COUNT(al.id) > 0
         ORDER BY {ARTIST_NAME_ORDER_BY}"
    );
    let mut stmt = conn.prepare(&sql)?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::queries::{albums, tracks};
    use crate::db::schema::test_connection;

    fn insert_active_track(conn: &Connection, path: &str, artist_id: i64, album_id: i64) {
        tracks::upsert(conn, &tracks::NewTrack {
            path,
            folder_path: "/music",
            title: "Test",
            track_artist_id: artist_id,
            album_id,
            genre_id: None,
            track_no: Some(1),
            disc_no: Some(1),
            duration_ms: 180_000,
            sample_rate: Some(44100),
            bit_depth: Some(16),
            channels: Some(2),
            codec: "flac",
            bitrate_kbps: None,
            file_size: 1_000,
            file_mtime: 0,
            now: 0,
        }).unwrap();
    }

    #[test]
    fn upsert_is_idempotent_for_the_same_name() {
        let conn = test_connection();
        let first = upsert(&conn, "Thrice").unwrap();
        let second = upsert(&conn, "Thrice").unwrap();
        assert_eq!(first, second);
    }

    #[test]
    fn upsert_gives_different_names_different_ids() {
        let conn = test_connection();
        let a = upsert(&conn, "Thrice").unwrap();
        let b = upsert(&conn, "Norma Jean").unwrap();
        assert_ne!(a, b);
    }

    #[test]
    fn list_album_artists_excludes_artists_with_all_albums_hidden() {
        let conn = test_connection();
        let visible_artist = upsert(&conn, "Visible Artist").unwrap();
        let hidden_artist = upsert(&conn, "Hidden Artist").unwrap();
        let visible_album = albums::upsert(&conn, "Visible Album", visible_artist, Some(2001)).unwrap();
        let hidden_album = albums::upsert(&conn, "Hidden Album", hidden_artist, Some(2002)).unwrap();
        insert_active_track(&conn, "/music/visible.flac", visible_artist, visible_album);
        insert_active_track(&conn, "/music/hidden.flac", hidden_artist, hidden_album);
        super::super::hidden_albums::hide(&conn, hidden_album).unwrap();

        let rows = list_album_artists(&conn).unwrap();
        let names: Vec<&str> = rows.iter().map(|r| r.name.as_str()).collect();
        assert_eq!(names, vec!["Visible Artist"]);
    }

    #[test]
    fn list_album_artists_sorts_case_insensitively_with_symbols_before_letters() {
        let conn = test_connection();
        let names = ["zebra", "Apple", "!!!", "banana", "3OH!3", "Aardvark"];
        for (index, name) in names.iter().enumerate() {
            let artist_id = upsert(&conn, name).unwrap();
            let album_id = albums::upsert(&conn, "Album", artist_id, Some(2000 + index as i64)).unwrap();
            insert_active_track(&conn, &format!("/music/{index}.flac"), artist_id, album_id);
        }

        let rows = list_album_artists(&conn).unwrap();
        let names: Vec<&str> = rows.iter().map(|r| r.name.as_str()).collect();
        assert_eq!(names, vec!["!!!", "3OH!3", "Aardvark", "Apple", "banana", "zebra"]);
    }

    #[test]
    fn list_album_artists_excludes_artists_with_no_active_tracks() {
        let conn = test_connection();
        let has_active = upsert(&conn, "Has Active Track").unwrap();
        let has_archived_only = upsert(&conn, "Has Archived Only").unwrap();
        upsert(&conn, "No Albums").unwrap();

        let active_album = albums::upsert(&conn, "Active Album", has_active, Some(2001)).unwrap();
        let archived_album = albums::upsert(&conn, "Archived Album", has_archived_only, Some(2002)).unwrap();

        insert_active_track(&conn, "/music/active.flac", has_active, active_album);
        insert_active_track(&conn, "/music/archived.flac", has_archived_only, archived_album);
        conn.execute("UPDATE tracks SET is_archived = 1 WHERE path = '/music/archived.flac'", []).unwrap();

        let rows = list_album_artists(&conn).unwrap();
        let names: Vec<&str> = rows.iter().map(|r| r.name.as_str()).collect();
        assert_eq!(names, vec!["Has Active Track"]);
        assert_eq!(rows[0].album_count, 1);
    }
}
