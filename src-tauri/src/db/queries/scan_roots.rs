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

pub fn remove(conn: &Connection, path: &str, keep_stats: bool) -> Result<(), AppError> {
    let tx = conn.unchecked_transaction()?;
    // substr comparison avoids LIKE's _ and % wildcard interpretation, which
    // would silently match unintended paths if the folder name contained those chars.
    if keep_stats {
        tx.execute(
            "UPDATE tracks SET is_archived = 1 WHERE substr(path, 1, length(?1) + 1) = ?1 || '/'",
            [path],
        )?;
        // Albums and artists are left in place; the library queries filter them
        // out by checking for at least one non-archived track.
    } else {
        tx.execute(
            "DELETE FROM tracks WHERE substr(path, 1, length(?1) + 1) = ?1 || '/'",
            [path],
        )?;
        tx.execute(
            "DELETE FROM albums WHERE id NOT IN (SELECT DISTINCT album_id FROM tracks WHERE album_id IS NOT NULL)",
            [],
        )?;
        tx.execute(
            "DELETE FROM artists WHERE id NOT IN (
                SELECT DISTINCT album_artist_id FROM albums WHERE album_artist_id IS NOT NULL
                UNION
                SELECT DISTINCT track_artist_id FROM tracks WHERE track_artist_id IS NOT NULL
            )",
            [],
        )?;
    }
    tx.execute("DELETE FROM scan_roots WHERE path = ?1", [path])?;
    tx.commit()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{
        queries::{albums, artists, tracks},
        schema::test_connection,
    };

    fn insert_track(conn: &Connection, path: &str, folder: &str, album_id: i64, artist_id: i64) {
        tracks::upsert(conn, &tracks::NewTrack {
            path,
            folder_path: folder,
            title: "Test Track",
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
        })
        .unwrap();
    }

    fn track_count(conn: &Connection) -> i64 {
        conn.query_row("SELECT COUNT(*) FROM tracks", [], |r| r.get(0)).unwrap()
    }

    fn album_count(conn: &Connection) -> i64 {
        conn.query_row("SELECT COUNT(*) FROM albums", [], |r| r.get(0)).unwrap()
    }

    fn artist_count(conn: &Connection) -> i64 {
        conn.query_row("SELECT COUNT(*) FROM artists", [], |r| r.get(0)).unwrap()
    }

    #[test]
    fn has_enabled_is_false_until_a_root_is_added() {
        let conn = test_connection();
        assert!(!has_enabled(&conn).unwrap());
        add(&conn, "/home/luke/Music").unwrap();
        assert!(has_enabled(&conn).unwrap());
    }

    #[test]
    fn add_is_idempotent_for_the_same_path() {
        let conn = test_connection();
        add(&conn, "/home/luke/Music").unwrap();
        add(&conn, "/home/luke/Music").unwrap();
        assert_eq!(list_enabled(&conn).unwrap(), vec!["/home/luke/Music".to_string()]);
    }

    #[test]
    fn list_enabled_returns_every_added_root() {
        let conn = test_connection();
        add(&conn, "/home/luke/Music").unwrap();
        add(&conn, "/mnt/nas/Music").unwrap();
        let mut roots = list_enabled(&conn).unwrap();
        roots.sort();
        assert_eq!(roots, vec!["/home/luke/Music".to_string(), "/mnt/nas/Music".to_string()]);
    }

    #[test]
    fn remove_deletes_tracks_and_cleans_up_album_and_artist() {
        let conn = test_connection();
        let artist_id = artists::upsert(&conn, "Radiohead").unwrap();
        let album_id = albums::upsert(&conn, "OK Computer", artist_id, Some(1997)).unwrap();
        add(&conn, "/music").unwrap();
        insert_track(&conn, "/music/ok_computer/track1.flac", "/music/ok_computer", album_id, artist_id);

        remove(&conn, "/music", false).unwrap();

        assert_eq!(track_count(&conn), 0);
        assert_eq!(album_count(&conn), 0);
        assert_eq!(artist_count(&conn), 0);
        assert!(!has_enabled(&conn).unwrap());
    }

    #[test]
    fn remove_does_not_affect_tracks_from_other_roots() {
        let conn = test_connection();
        let artist_id = artists::upsert(&conn, "Artist").unwrap();
        let album_a = albums::upsert(&conn, "Album A", artist_id, Some(2000)).unwrap();
        let album_b = albums::upsert(&conn, "Album B", artist_id, Some(2001)).unwrap();
        add(&conn, "/music").unwrap();
        add(&conn, "/nas/music").unwrap();
        insert_track(&conn, "/music/a/track.flac", "/music/a", album_a, artist_id);
        insert_track(&conn, "/nas/music/b/track.flac", "/nas/music/b", album_b, artist_id);

        remove(&conn, "/music", false).unwrap();

        assert_eq!(track_count(&conn), 1);
        assert_eq!(album_count(&conn), 1);
    }

    #[test]
    fn remove_does_not_match_sibling_dirs_with_underscore_in_name() {
        // Regression: the old LIKE '/%' pattern treated '_' as a wildcard,
        // so removing "/music" would also delete tracks from "/music_extra".
        let conn = test_connection();
        let artist_id = artists::upsert(&conn, "Artist").unwrap();
        let album_a = albums::upsert(&conn, "Album A", artist_id, Some(2000)).unwrap();
        let album_b = albums::upsert(&conn, "Album B", artist_id, Some(2001)).unwrap();
        add(&conn, "/music").unwrap();
        add(&conn, "/music_extra").unwrap();
        insert_track(&conn, "/music/track.flac", "/music", album_a, artist_id);
        insert_track(&conn, "/music_extra/track.flac", "/music_extra", album_b, artist_id);

        remove(&conn, "/music", false).unwrap();

        assert_eq!(track_count(&conn), 1, "track in /music_extra must survive");
        let remaining: String = conn
            .query_row("SELECT path FROM tracks", [], |r| r.get(0))
            .unwrap();
        assert_eq!(remaining, "/music_extra/track.flac");
    }

    #[test]
    fn remove_with_keep_stats_archives_tracks_instead_of_deleting() {
        let conn = test_connection();
        let artist_id = artists::upsert(&conn, "Radiohead").unwrap();
        let album_id = albums::upsert(&conn, "OK Computer", artist_id, Some(1997)).unwrap();
        add(&conn, "/music").unwrap();
        insert_track(&conn, "/music/ok_computer/track1.flac", "/music/ok_computer", album_id, artist_id);

        remove(&conn, "/music", true).unwrap();

        assert_eq!(track_count(&conn), 1, "track must remain in DB");
        let archived: i64 = conn
            .query_row("SELECT is_archived FROM tracks", [], |r| r.get(0))
            .unwrap();
        assert_eq!(archived, 1, "track must be marked archived");
        // Album and artist survive because their tracks are still referenced
        assert_eq!(album_count(&conn), 1);
        assert_eq!(artist_count(&conn), 1);
        assert!(!has_enabled(&conn).unwrap());
    }

    #[test]
    fn remove_with_keep_stats_only_archives_tracks_from_the_given_root() {
        let conn = test_connection();
        let artist_id = artists::upsert(&conn, "Artist").unwrap();
        let album_a = albums::upsert(&conn, "Album A", artist_id, Some(2000)).unwrap();
        let album_b = albums::upsert(&conn, "Album B", artist_id, Some(2001)).unwrap();
        add(&conn, "/music").unwrap();
        add(&conn, "/nas/music").unwrap();
        insert_track(&conn, "/music/a/track.flac", "/music/a", album_a, artist_id);
        insert_track(&conn, "/nas/music/b/track.flac", "/nas/music/b", album_b, artist_id);

        remove(&conn, "/music", true).unwrap();

        let archived: i64 = conn
            .query_row("SELECT is_archived FROM tracks WHERE path = '/music/a/track.flac'", [], |r| r.get(0))
            .unwrap();
        assert_eq!(archived, 1);
        let active: i64 = conn
            .query_row("SELECT is_archived FROM tracks WHERE path = '/nas/music/b/track.flac'", [], |r| r.get(0))
            .unwrap();
        assert_eq!(active, 0, "track from other root must remain active");
    }

    #[test]
    fn remove_preserves_albums_that_still_have_tracks_in_another_root() {
        let conn = test_connection();
        let artist_id = artists::upsert(&conn, "Artist").unwrap();
        let album_id = albums::upsert(&conn, "Split Album", artist_id, Some(2000)).unwrap();
        add(&conn, "/root_a").unwrap();
        add(&conn, "/root_b").unwrap();
        insert_track(&conn, "/root_a/track1.flac", "/root_a", album_id, artist_id);
        insert_track(&conn, "/root_b/track2.flac", "/root_b", album_id, artist_id);

        remove(&conn, "/root_a", false).unwrap();

        assert_eq!(track_count(&conn), 1);
        assert_eq!(album_count(&conn), 1, "album still has a track in /root_b");
        assert_eq!(artist_count(&conn), 1);
    }
}
