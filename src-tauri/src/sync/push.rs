use rusqlite::Connection;
use serde_json::Value;

use super::{
    client::{float_arg, int_arg, opt_int_arg, text_arg, TursoClient},
    SyncError,
};

pub struct PendingRating {
    pub title: String,
    pub artist: String,
    pub year_str: String,
    pub rating: f64,
    pub updated_at: i64,
}

pub struct PendingPlay {
    pub path: String,
    pub own_play_count: i64,
    pub last_played: Option<i64>,
}

/// Reads all locally rated albums that have a real updated_at timestamp.
pub fn collect_pending_ratings(conn: &Connection) -> Result<Vec<PendingRating>, SyncError> {
    let mut stmt = conn
        .prepare(
            "SELECT al.title, ar.name, al.year, art.rating, art.updated_at
             FROM album_ratings art
             JOIN albums al ON al.id = art.album_id
             JOIN artists ar ON ar.id = al.album_artist_id",
        )
        .map_err(|e| SyncError::Db(e.to_string()))?;

    let rows: Result<Vec<PendingRating>, rusqlite::Error> = stmt
        .query_map([], |row| {
            let year: Option<i64> = row.get(2)?;
            Ok(PendingRating {
                title: row.get(0)?,
                artist: row.get(1)?,
                year_str: year.map(|y| y.to_string()).unwrap_or_default(),
                rating: row.get(3)?,
                updated_at: row.get(4)?,
            })
        })
        .map_err(|e| SyncError::Db(e.to_string()))?
        .collect();
    rows.map_err(|e| SyncError::Db(e.to_string()))
}

/// Reads tracks where own_play_count has grown beyond the last synced value.
pub fn collect_pending_plays(conn: &Connection) -> Result<Vec<PendingPlay>, SyncError> {
    let mut stmt = conn
        .prepare(
            "SELECT t.path, ts.own_play_count, ts.last_played
             FROM track_stats ts
             JOIN tracks t ON t.id = ts.track_id
             WHERE ts.own_play_count > ts.synced_play_count",
        )
        .map_err(|e| SyncError::Db(e.to_string()))?;

    let rows: Result<Vec<PendingPlay>, rusqlite::Error> = stmt
        .query_map([], |row| {
            Ok(PendingPlay {
                path: row.get(0)?,
                own_play_count: row.get(1)?,
                last_played: row.get(2)?,
            })
        })
        .map_err(|e| SyncError::Db(e.to_string()))?
        .collect();
    rows.map_err(|e| SyncError::Db(e.to_string()))
}

/// Upserts pending ratings to the cloud, respecting LWW via updated_at.
pub async fn upload_ratings(client: &TursoClient, ratings: &[PendingRating]) -> Result<(), SyncError> {
    if ratings.is_empty() {
        return Ok(());
    }
    let stmts: Vec<(String, Vec<Value>)> = ratings
        .iter()
        .map(|r| {
            (
                "INSERT INTO cloud_album_ratings
                     (album_title, album_artist, year_str, rating, updated_at)
                 VALUES (?, ?, ?, ?, ?)
                 ON CONFLICT(album_title, album_artist, year_str) DO UPDATE SET
                     rating     = CASE WHEN excluded.updated_at > cloud_album_ratings.updated_at
                                       THEN excluded.rating
                                       ELSE cloud_album_ratings.rating END,
                     updated_at = MAX(cloud_album_ratings.updated_at, excluded.updated_at)"
                    .to_string(),
                vec![
                    text_arg(&r.title),
                    text_arg(&r.artist),
                    text_arg(&r.year_str),
                    float_arg(r.rating),
                    int_arg(r.updated_at),
                ],
            )
        })
        .collect();
    client.execute_batch(stmts).await
}

/// Upserts this machine's play counts to the cloud.
pub async fn upload_plays(
    client: &TursoClient,
    plays: &[PendingPlay],
    machine_id: &str,
) -> Result<(), SyncError> {
    if plays.is_empty() {
        return Ok(());
    }
    let now = unix_now();
    let stmts: Vec<(String, Vec<Value>)> = plays
        .iter()
        .map(|p| {
            (
                "INSERT INTO cloud_track_plays
                     (track_path, machine_id, own_play_count, last_played, updated_at)
                 VALUES (?, ?, ?, ?, ?)
                 ON CONFLICT(track_path, machine_id) DO UPDATE SET
                     own_play_count = excluded.own_play_count,
                     last_played    = excluded.last_played,
                     updated_at     = excluded.updated_at"
                    .to_string(),
                vec![
                    text_arg(&p.path),
                    text_arg(machine_id),
                    int_arg(p.own_play_count),
                    opt_int_arg(p.last_played),
                    int_arg(now),
                ],
            )
        })
        .collect();
    client.execute_batch(stmts).await
}

/// Marks the given track paths as fully synced (synced_play_count = own_play_count).
pub fn mark_plays_synced(conn: &Connection, plays: &[PendingPlay]) -> Result<(), SyncError> {
    if plays.is_empty() {
        return Ok(());
    }
    let placeholders: Vec<String> = (1..=plays.len()).map(|i| format!("?{i}")).collect();
    let sql = format!(
        "UPDATE track_stats SET synced_play_count = own_play_count
         WHERE track_id IN (SELECT id FROM tracks WHERE path IN ({}))",
        placeholders.join(", ")
    );
    let paths: Vec<&str> = plays.iter().map(|p| p.path.as_str()).collect();
    conn.execute(&sql, rusqlite::params_from_iter(paths.iter()))
        .map_err(|e| SyncError::Db(e.to_string()))?;
    Ok(())
}

/// One-shot push of a single album rating change — called fire-and-forget
/// after library_set_album_rating so the cloud stays in sync immediately.
pub async fn push_one_album_rating(
    client: &TursoClient,
    title: &str,
    artist: &str,
    year: Option<i64>,
    rating: Option<f64>,
    updated_at: i64,
) -> Result<(), SyncError> {
    let year_str = year.map(|y| y.to_string()).unwrap_or_default();
    match rating {
        Some(r) => {
            client
                .execute(
                    "INSERT INTO cloud_album_ratings
                         (album_title, album_artist, year_str, rating, updated_at)
                     VALUES (?, ?, ?, ?, ?)
                     ON CONFLICT(album_title, album_artist, year_str) DO UPDATE SET
                         rating     = CASE WHEN excluded.updated_at > cloud_album_ratings.updated_at
                                           THEN excluded.rating
                                           ELSE cloud_album_ratings.rating END,
                         updated_at = MAX(cloud_album_ratings.updated_at, excluded.updated_at)",
                    vec![
                        text_arg(title),
                        text_arg(artist),
                        text_arg(&year_str),
                        float_arg(r),
                        int_arg(updated_at),
                    ],
                )
                .await
        }
        None => {
            client
                .execute(
                    "DELETE FROM cloud_album_ratings
                     WHERE album_title = ? AND album_artist = ? AND year_str = ?",
                    vec![text_arg(title), text_arg(artist), text_arg(&year_str)],
                )
                .await
        }
    }
}

fn unix_now() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs() as i64
}
