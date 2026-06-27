use rusqlite::{params, Connection, OptionalExtension};

use super::{
    client::{row_float, row_int, row_text, text_arg, TursoClient},
    SyncError,
};

pub struct CloudRating {
    pub title: String,
    pub artist: String,
    pub year_str: String,
    pub rating: Option<f64>, // None = tombstone (rating was deleted on another machine)
    pub updated_at: i64,
}

pub struct CloudPlay {
    pub album_title: String,
    pub album_artist: String,
    pub year_str: String,
    pub track_title: String,
    pub disc_no: i64,
    pub track_no: i64,
    /// SUM of all machines' own_play_count except this one.
    pub other_count: i64,
    /// MAX(last_played) across all machines.
    pub latest_played: Option<i64>,
}

pub async fn fetch_cloud_ratings(client: &TursoClient) -> Result<Vec<CloudRating>, SyncError> {
    let result = client
        .query(
            "SELECT album_title, album_artist, year_str, rating, updated_at
             FROM cloud_album_ratings",
            vec![],
        )
        .await?;

    let mut out = Vec::new();
    for row in &result.rows {
        let title = row_text(&row[0]).unwrap_or("").to_string();
        let artist = row_text(&row[1]).unwrap_or("").to_string();
        let year_str = row_text(&row[2]).unwrap_or("").to_string();
        let rating = row_float(&row[3]); // None = tombstone
        let updated_at = row_int(&row[4]).unwrap_or(0);
        out.push(CloudRating { title, artist, year_str, rating, updated_at });
    }
    Ok(out)
}

pub async fn fetch_cloud_plays(
    client: &TursoClient,
    machine_id: &str,
) -> Result<Vec<CloudPlay>, SyncError> {
    let result = client
        .query(
            "SELECT album_title, album_artist, year_str, track_title, disc_no, track_no,
                    SUM(own_play_count) AS total,
                    MAX(last_played)    AS latest,
                    SUM(CASE WHEN machine_id = ? THEN own_play_count ELSE 0 END) AS my_count
             FROM cloud_track_plays
             GROUP BY album_title, album_artist, year_str, track_title, disc_no, track_no",
            vec![text_arg(machine_id)],
        )
        .await?;

    let mut out = Vec::new();
    for row in &result.rows {
        let album_title = row_text(&row[0]).unwrap_or("").to_string();
        let album_artist = row_text(&row[1]).unwrap_or("").to_string();
        let year_str = row_text(&row[2]).unwrap_or("").to_string();
        let track_title = row_text(&row[3]).unwrap_or("").to_string();
        let disc_no = row_int(&row[4]).unwrap_or(1);
        let track_no = row_int(&row[5]).unwrap_or(0);
        let total = row_int(&row[6]).unwrap_or(0);
        let latest_played = row_int(&row[7]);
        let my_count = row_int(&row[8]).unwrap_or(0);
        let other_count = total - my_count;
        if other_count > 0 || latest_played.is_some() {
            out.push(CloudPlay {
                album_title,
                album_artist,
                year_str,
                track_title,
                disc_no,
                track_no,
                other_count,
                latest_played,
            });
        }
    }
    Ok(out)
}

/// Applies cloud ratings to the local DB (LWW: cloud wins when its timestamp is newer).
pub fn apply_ratings(conn: &Connection, ratings: &[CloudRating]) -> Result<usize, SyncError> {
    let mut applied = 0;
    for cr in ratings {
        let year: Option<i64> = if cr.year_str.is_empty() {
            None
        } else {
            cr.year_str.parse().ok()
        };

        let album_id: Option<i64> = conn
            .query_row(
                "SELECT al.id FROM albums al
                 JOIN artists ar ON ar.id = al.album_artist_id
                 WHERE al.title = ?1
                   AND ar.name  = ?2
                   AND ((?3 IS NULL AND al.year IS NULL) OR al.year = ?3)",
                params![cr.title, cr.artist, year],
                |row| row.get(0),
            )
            .optional()
            .map_err(|e| SyncError::Db(e.to_string()))?;

        let Some(album_id) = album_id else { continue };

        let local_ts: i64 = conn
            .query_row(
                "SELECT COALESCE(updated_at, 0) FROM album_ratings WHERE album_id = ?1",
                [album_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(|e| SyncError::Db(e.to_string()))?
            .unwrap_or(0);

        if cr.updated_at >= local_ts {
            // rating is Option<f64>: Some(r) = real rating, None = tombstone.
            // rusqlite binds None as SQL NULL, which is what we want for tombstones.
            conn.execute(
                "INSERT INTO album_ratings (album_id, rating, updated_at) VALUES (?1, ?2, ?3)
                 ON CONFLICT(album_id) DO UPDATE SET
                     rating     = excluded.rating,
                     updated_at = excluded.updated_at",
                params![album_id, cr.rating, cr.updated_at],
            )
            .map_err(|e| SyncError::Db(e.to_string()))?;
            applied += 1;
        }
    }
    Ok(applied)
}

/// Merges other machines' play counts into the local track_stats.
pub fn apply_plays(conn: &Connection, plays: &[CloudPlay]) -> Result<usize, SyncError> {
    let mut merged = 0;
    for cp in plays {
        if cp.other_count <= 0 {
            continue;
        }

        let year: Option<i64> = if cp.year_str.is_empty() {
            None
        } else {
            cp.year_str.parse().ok()
        };

        let track_id: Option<i64> = conn
            .query_row(
                "SELECT t.id FROM tracks t
                 JOIN albums al  ON al.id  = t.album_id
                 JOIN artists ar ON ar.id  = al.album_artist_id
                 WHERE al.title  = ?1
                   AND ar.name   = ?2
                   AND ((?3 IS NULL AND al.year IS NULL) OR al.year = ?3)
                   AND t.title   = ?4
                   AND COALESCE(t.disc_no,  1) = ?5
                   AND COALESCE(t.track_no, 0) = ?6",
                params![cp.album_title, cp.album_artist, year, cp.track_title, cp.disc_no, cp.track_no],
                |row| row.get(0),
            )
            .optional()
            .map_err(|e| SyncError::Db(e.to_string()))?;

        let Some(track_id) = track_id else { continue };

        conn.execute(
            "INSERT INTO track_stats
                 (track_id, play_count, own_play_count, last_played, rating, is_favorite)
             VALUES (?1, ?2, 0, ?3, NULL, 0)
             ON CONFLICT(track_id) DO UPDATE SET
                 play_count  = own_play_count + ?2,
                 last_played = CASE WHEN ?3 IS NOT NULL
                                     AND ?3 > COALESCE(last_played, 0)
                                    THEN ?3
                                    ELSE last_played END",
            params![track_id, cp.other_count, cp.latest_played],
        )
        .map_err(|e| SyncError::Db(e.to_string()))?;

        merged += 1;
    }
    Ok(merged)
}
