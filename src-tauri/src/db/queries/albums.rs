use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::{params, OptionalExtension, Connection};
use serde::Serialize;

use crate::error::AppError;

const NEW_ALBUM_THRESHOLD_SECS: i64 = 5 * 86_400;

fn now_unix() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

#[derive(Debug, Serialize)]
pub struct AlbumRow {
    pub id: i64,
    pub title: String,
    pub album_artist: String,
    pub album_artist_id: i64,
    pub year: Option<i64>,
    pub art_path: Option<String>,
    pub rating: Option<f64>,
    pub date_added: Option<i64>,
    pub is_new: bool,
}

pub fn list_all(
    conn: &Connection,
    artist_id: Option<i64>,
    hidden_only: bool,
) -> Result<Vec<AlbumRow>, AppError> {
    let new_since = now_unix() - NEW_ALBUM_THRESHOLD_SECS;
    let is_new_expr =
        "CASE WHEN (al.date_added IS NOT NULL AND al.date_added > ?2
              AND NOT EXISTS (
                  SELECT 1 FROM track_stats ts
                  JOIN tracks t ON t.id = ts.track_id
                  WHERE t.album_id = al.id AND ts.play_count > 0
              )) THEN 1 ELSE 0 END";

    let sql = if hidden_only {
        format!(
            "SELECT al.id, al.title, ar.name, al.album_artist_id, al.year, al.art_path,
                    art.rating, al.date_added, {is_new_expr} AS is_new
             FROM albums al
             JOIN hidden_albums ha ON ha.album_id = al.id
             LEFT JOIN artists ar ON ar.id = al.album_artist_id
             LEFT JOIN album_ratings art ON art.album_id = al.id
             WHERE (?1 IS NULL OR al.album_artist_id = ?1)
               AND EXISTS (SELECT 1 FROM tracks WHERE album_id = al.id AND is_archived = 0)
             ORDER BY ar.sort_name, ar.name, al.year, al.title"
        )
    } else {
        format!(
            "SELECT al.id, al.title, ar.name, al.album_artist_id, al.year, al.art_path,
                    art.rating, al.date_added, {is_new_expr} AS is_new
             FROM albums al
             LEFT JOIN artists ar ON ar.id = al.album_artist_id
             LEFT JOIN album_ratings art ON art.album_id = al.id
             LEFT JOIN hidden_albums ha ON ha.album_id = al.id
             WHERE ha.album_id IS NULL
               AND (?1 IS NULL OR al.album_artist_id = ?1)
               AND EXISTS (SELECT 1 FROM tracks WHERE album_id = al.id AND is_archived = 0)
             ORDER BY ar.sort_name, ar.name, al.year, al.title"
        )
    };
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params![artist_id, new_since], |row| {
        Ok(AlbumRow {
            id: row.get(0)?,
            title: row.get(1)?,
            album_artist: row.get::<_, Option<String>>(2)?.unwrap_or_default(),
            album_artist_id: row.get(3)?,
            year: row.get(4)?,
            art_path: row.get(5)?,
            rating: row.get(6)?,
            date_added: row.get(7)?,
            is_new: row.get(8)?,
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
    let now = now_unix();
    let id: i64 = conn.query_row(
        "INSERT INTO albums (title, album_artist_id, year, date_added) VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT(title, album_artist_id, year) DO UPDATE SET title = excluded.title
         RETURNING id",
        params![title, album_artist_id, year, now],
        |row| row.get(0),
    )?;
    Ok(id)
}

/// Albums lacking cached art: never attempted (`art_source IS NULL`), or
/// previously attempted and found nothing (`art_source = 'none'`). The
/// latter must be retried on every scan rather than cached permanently,
/// since a folder cover image can be added to an album's directory after
/// it was first scanned.
pub fn list_missing_art(conn: &Connection) -> Result<Vec<i64>, AppError> {
    let mut stmt = conn.prepare("SELECT id FROM albums WHERE art_source IS NULL OR art_source = 'none'")?;
    let rows = stmt.query_map([], |row| row.get::<_, i64>(0))?;
    rows.collect::<Result<Vec<_>, _>>().map_err(AppError::from)
}

pub struct AlbumNaturalKey {
    pub title: String,
    pub artist: String,
    pub year: Option<i64>,
}

pub fn get_natural_key(conn: &Connection, album_id: i64) -> Result<Option<AlbumNaturalKey>, AppError> {
    conn.query_row(
        "SELECT al.title, ar.name, al.year
         FROM albums al
         JOIN artists ar ON ar.id = al.album_artist_id
         WHERE al.id = ?1",
        [album_id],
        |row| {
            Ok(AlbumNaturalKey {
                title: row.get(0)?,
                artist: row.get(1)?,
                year: row.get(2)?,
            })
        },
    )
    .optional()
    .map_err(AppError::from)
}

pub fn set_art(conn: &Connection, album_id: i64, art_path: Option<&str>, art_source: &str) -> Result<(), AppError> {
    conn.execute(
        "UPDATE albums SET art_path = ?1, art_source = ?2 WHERE id = ?3",
        params![art_path, art_source, album_id],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::queries::{artists, stats, tracks};
    use crate::db::schema::test_connection;

    fn insert_active_track(conn: &Connection, path: &str, artist_id: i64, album_id: i64) -> i64 {
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
        }).unwrap()
    }

    #[test]
    fn upsert_is_idempotent_for_the_same_title_artist_year() {
        let conn = test_connection();
        let artist_id = artists::upsert(&conn, "Thrice").unwrap();
        let first = upsert(&conn, "Vheissu", artist_id, Some(2005)).unwrap();
        let second = upsert(&conn, "Vheissu", artist_id, Some(2005)).unwrap();
        assert_eq!(first, second);
    }

    #[test]
    fn upsert_with_different_year_creates_a_different_album() {
        let conn = test_connection();
        let artist_id = artists::upsert(&conn, "Thrice").unwrap();
        let a = upsert(&conn, "Vheissu", artist_id, Some(2005)).unwrap();
        let b = upsert(&conn, "Vheissu", artist_id, Some(1999)).unwrap();
        assert_ne!(a, b);
    }

    /// Documents a real SQLite gotcha: a UNIQUE constraint treats every
    /// NULL as distinct from every other NULL, so calling upsert twice with
    /// no year does NOT dedupe the way it does with a real year value.
    /// This is why the scanner resolves album identity through an
    /// in-process cache during a scan rather than relying on this
    /// constraint alone for albums with no YEAR tag.
    #[test]
    fn upsert_with_null_year_does_not_dedupe_across_separate_calls() {
        let conn = test_connection();
        let artist_id = artists::upsert(&conn, "Thrice").unwrap();
        let a = upsert(&conn, "Untitled Demo", artist_id, None).unwrap();
        let b = upsert(&conn, "Untitled Demo", artist_id, None).unwrap();
        assert_ne!(a, b, "NULL year values are never equal to each other under the UNIQUE constraint");
    }

    #[test]
    fn list_all_filters_by_artist_when_given() {
        let conn = test_connection();
        let a = artists::upsert(&conn, "Thrice").unwrap();
        let b = artists::upsert(&conn, "Norma Jean").unwrap();
        let album_a = upsert(&conn, "Vheissu", a, Some(2005)).unwrap();
        let album_b = upsert(&conn, "Redeemer", b, Some(2002)).unwrap();
        insert_active_track(&conn, "/music/a.flac", a, album_a);
        insert_active_track(&conn, "/music/b.flac", b, album_b);

        let all = list_all(&conn, None, false).unwrap();
        assert_eq!(all.len(), 2);

        let just_a = list_all(&conn, Some(a), false).unwrap();
        assert_eq!(just_a.len(), 1);
        assert_eq!(just_a[0].title, "Vheissu");
    }

    #[test]
    fn list_all_excludes_hidden_albums() {
        let conn = test_connection();
        let artist_id = artists::upsert(&conn, "Thrice").unwrap();
        let visible = upsert(&conn, "Vheissu", artist_id, Some(2005)).unwrap();
        let hidden = upsert(&conn, "The Artist in the Ambulance", artist_id, Some(2003)).unwrap();
        insert_active_track(&conn, "/music/vheissu.flac", artist_id, visible);
        insert_active_track(&conn, "/music/taia.flac", artist_id, hidden);
        super::super::hidden_albums::hide(&conn, hidden).unwrap();

        let rows = list_all(&conn, None, false).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].id, visible);
    }

    #[test]
    fn list_all_returns_only_hidden_albums_when_hidden_only_is_true() {
        let conn = test_connection();
        let artist_id = artists::upsert(&conn, "Thrice").unwrap();
        let visible = upsert(&conn, "Vheissu", artist_id, Some(2005)).unwrap();
        let hidden = upsert(&conn, "The Artist in the Ambulance", artist_id, Some(2003)).unwrap();
        insert_active_track(&conn, "/music/vheissu.flac", artist_id, visible);
        insert_active_track(&conn, "/music/taia.flac", artist_id, hidden);
        super::super::hidden_albums::hide(&conn, hidden).unwrap();

        let rows = list_all(&conn, None, true).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].id, hidden);
    }

    #[test]
    fn list_all_hidden_only_excludes_albums_with_only_archived_tracks() {
        let conn = test_connection();
        let artist_id = artists::upsert(&conn, "Artist").unwrap();
        let hidden_active = upsert(&conn, "Hidden Active", artist_id, Some(2020)).unwrap();
        let hidden_archived = upsert(&conn, "Hidden Archived", artist_id, Some(2019)).unwrap();
        insert_active_track(&conn, "/music/active.flac", artist_id, hidden_active);
        insert_active_track(&conn, "/music/archived.flac", artist_id, hidden_archived);
        conn.execute("UPDATE tracks SET is_archived = 1 WHERE path = '/music/archived.flac'", []).unwrap();
        super::super::hidden_albums::hide(&conn, hidden_active).unwrap();
        super::super::hidden_albums::hide(&conn, hidden_archived).unwrap();

        let rows = list_all(&conn, None, true).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].id, hidden_active);
    }

    #[test]
    fn list_all_excludes_albums_with_only_archived_tracks() {
        let conn = test_connection();
        let artist_id = artists::upsert(&conn, "Artist").unwrap();
        let active_album = upsert(&conn, "Active Album", artist_id, Some(2020)).unwrap();
        let archived_album = upsert(&conn, "Archived Album", artist_id, Some(2019)).unwrap();
        insert_active_track(&conn, "/music/active.flac", artist_id, active_album);
        insert_active_track(&conn, "/music/archived.flac", artist_id, archived_album);
        conn.execute("UPDATE tracks SET is_archived = 1 WHERE path = '/music/archived.flac'", []).unwrap();

        let rows = list_all(&conn, None, false).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].title, "Active Album");
    }

    #[test]
    fn missing_art_then_set_art_round_trips() {
        let conn = test_connection();
        let artist_id = artists::upsert(&conn, "Thrice").unwrap();
        let album_id = upsert(&conn, "Vheissu", artist_id, Some(2005)).unwrap();
        insert_active_track(&conn, "/music/vheissu.flac", artist_id, album_id);

        assert_eq!(list_missing_art(&conn).unwrap(), vec![album_id]);

        set_art(&conn, album_id, Some("/cache/art/1.jpg"), "embedded").unwrap();

        assert_eq!(list_missing_art(&conn).unwrap(), Vec::<i64>::new());
        let rows = list_all(&conn, None, false).unwrap();
        assert_eq!(rows[0].art_path.as_deref(), Some("/cache/art/1.jpg"));
    }

    #[test]
    fn albums_previously_found_to_have_no_art_are_retried_on_the_next_scan() {
        let conn = test_connection();
        let artist_id = artists::upsert(&conn, "Thrice").unwrap();
        let album_id = upsert(&conn, "Vheissu", artist_id, Some(2005)).unwrap();
        insert_active_track(&conn, "/music/vheissu.flac", artist_id, album_id);

        set_art(&conn, album_id, None, "none").unwrap();

        assert_eq!(
            list_missing_art(&conn).unwrap(),
            vec![album_id],
            "a cover image may have been added to the album folder since the last scan, so \
             albums with no art found previously must be re-checked rather than skipped forever"
        );
    }

    #[test]
    fn is_new_is_true_for_recently_added_unplayed_album() {
        let conn = test_connection();
        let artist_id = artists::upsert(&conn, "Thrice").unwrap();
        let album_id = upsert(&conn, "Vheissu", artist_id, Some(2005)).unwrap();
        insert_active_track(&conn, "/music/a.flac", artist_id, album_id);

        let rows = list_all(&conn, None, false).unwrap();
        assert_eq!(rows.len(), 1);
        assert!(rows[0].is_new, "a freshly added unplayed album should be new");
    }

    #[test]
    fn is_new_is_false_after_any_track_is_played() {
        let conn = test_connection();
        let artist_id = artists::upsert(&conn, "Thrice").unwrap();
        let album_id = upsert(&conn, "Vheissu", artist_id, Some(2005)).unwrap();
        let track_id = insert_active_track(&conn, "/music/a.flac", artist_id, album_id);
        stats::record_play(&conn, track_id, 1_000).unwrap();

        let rows = list_all(&conn, None, false).unwrap();
        assert!(!rows[0].is_new, "an album with at least one play should not be new");
    }

    #[test]
    fn is_new_is_false_when_date_added_is_older_than_threshold() {
        let conn = test_connection();
        let artist_id = artists::upsert(&conn, "Thrice").unwrap();
        let album_id = upsert(&conn, "Vheissu", artist_id, Some(2005)).unwrap();
        insert_active_track(&conn, "/music/a.flac", artist_id, album_id);

        let six_days_ago = now_unix() - 6 * 86_400;
        conn.execute(
            "UPDATE albums SET date_added = ?1 WHERE id = ?2",
            params![six_days_ago, album_id],
        ).unwrap();

        let rows = list_all(&conn, None, false).unwrap();
        assert!(!rows[0].is_new, "an album added more than 5 days ago should not be new");
    }
}
