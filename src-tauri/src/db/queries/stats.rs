use rusqlite::{params, Connection};

use crate::error::AppError;

/// Records a play: an append-only history row plus a denormalized rollup
/// (play_count/last_played) for fast "most played"/"recently played"
/// sorting later without aggregating history at query time. Counted when a
/// track starts, not on completion — simpler and avoids needing the engine
/// to reliably distinguish "played to the end" from "skipped," which the
/// data so far doesn't need.
pub fn record_play(conn: &Connection, track_id: i64, played_at: i64) -> Result<(), AppError> {
    conn.execute(
        "INSERT INTO play_history (track_id, played_at, completed) VALUES (?1, ?2, 0)",
        params![track_id, played_at],
    )?;
    conn.execute(
        "INSERT INTO track_stats (track_id, play_count, own_play_count, last_played, rating, is_favorite)
         VALUES (?1, 1, 1, ?2, NULL, 0)
         ON CONFLICT(track_id) DO UPDATE SET
             play_count = play_count + 1,
             own_play_count = own_play_count + 1,
             last_played = excluded.last_played",
        params![track_id, played_at],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::queries::tracks::NewTrack;
    use crate::db::queries::{albums, artists, tracks};
    use crate::db::schema::test_connection;

    /// track_stats/play_history both reference tracks(id); the bundled
    /// SQLite build enforces foreign keys by default, so tests need a real
    /// track row rather than an arbitrary id.
    fn setup_track(conn: &Connection, path: &str) -> i64 {
        let artist_id = artists::upsert(conn, "Thrice").unwrap();
        let album_id = albums::upsert(conn, "Vheissu", artist_id, Some(2005)).unwrap();
        tracks::upsert(
            conn,
            &NewTrack {
                path,
                folder_path: "/music/album",
                title: "Image of the Invisible",
                track_artist_id: artist_id,
                album_id,
                genre_id: None,
                track_no: Some(1),
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
            },
        )
        .unwrap()
    }

    fn play_count(conn: &Connection, track_id: i64) -> i64 {
        conn.query_row("SELECT play_count FROM track_stats WHERE track_id = ?1", [track_id], |row| row.get(0))
            .unwrap()
    }

    fn history_count(conn: &Connection, track_id: i64) -> i64 {
        conn.query_row(
            "SELECT COUNT(*) FROM play_history WHERE track_id = ?1",
            [track_id],
            |row| row.get(0),
        )
        .unwrap()
    }

    #[test]
    fn first_play_creates_a_stats_row_with_count_one() {
        let conn = test_connection();
        let track_id = setup_track(&conn, "/music/a.flac");
        record_play(&conn, track_id, 1000).unwrap();
        assert_eq!(play_count(&conn, track_id), 1);
        assert_eq!(history_count(&conn, track_id), 1);
    }

    #[test]
    fn repeated_plays_increment_count_and_append_history() {
        let conn = test_connection();
        let track_id = setup_track(&conn, "/music/a.flac");
        record_play(&conn, track_id, 1000).unwrap();
        record_play(&conn, track_id, 2000).unwrap();
        record_play(&conn, track_id, 3000).unwrap();

        assert_eq!(play_count(&conn, track_id), 3);
        assert_eq!(history_count(&conn, track_id), 3);

        let last_played: i64 = conn
            .query_row("SELECT last_played FROM track_stats WHERE track_id = ?1", [track_id], |row| row.get(0))
            .unwrap();
        assert_eq!(last_played, 3000, "last_played should reflect the most recent play");
    }

    #[test]
    fn different_tracks_are_tracked_independently() {
        let conn = test_connection();
        let track_a = setup_track(&conn, "/music/a.flac");
        let track_b = setup_track(&conn, "/music/b.flac");
        record_play(&conn, track_a, 1000).unwrap();
        record_play(&conn, track_b, 1000).unwrap();
        record_play(&conn, track_a, 2000).unwrap();

        assert_eq!(play_count(&conn, track_a), 2);
        assert_eq!(play_count(&conn, track_b), 1);
    }
}
