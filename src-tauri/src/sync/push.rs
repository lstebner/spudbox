use rusqlite::Connection;
use serde_json::Value;

use super::{
    client::{float_arg, int_arg, null_arg, opt_int_arg, text_arg, TursoClient},
    SyncError,
};

pub struct PendingRating {
    pub title: String,
    pub artist: String,
    pub year_str: String,
    pub rating: Option<f64>, // None = tombstone (was explicitly unrated)
    pub updated_at: i64,
}

pub struct PendingPlay {
    pub track_id: i64,
    pub album_title: String,
    pub album_artist: String,
    pub year_str: String,
    pub track_title: String,
    pub disc_no: i64,
    pub track_no: i64,
    pub own_play_count: i64,
    pub last_played: Option<i64>,
}

/// Reads all locally rated albums.
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
                rating: row.get(3)?,  // Option<f64>: None for tombstone rows
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
            "SELECT ts.track_id,
                    al.title,
                    ar.name,
                    COALESCE(CAST(al.year AS TEXT), ''),
                    t.title,
                    COALESCE(t.disc_no, 1),
                    COALESCE(t.track_no, 0),
                    ts.own_play_count,
                    ts.last_played
             FROM track_stats ts
             JOIN tracks t  ON t.id  = ts.track_id
             JOIN albums al ON al.id = t.album_id
             JOIN artists ar ON ar.id = al.album_artist_id
             WHERE ts.own_play_count > ts.synced_play_count",
        )
        .map_err(|e| SyncError::Db(e.to_string()))?;

    let rows: Result<Vec<PendingPlay>, rusqlite::Error> = stmt
        .query_map([], |row| {
            Ok(PendingPlay {
                track_id: row.get(0)?,
                album_title: row.get(1)?,
                album_artist: row.get(2)?,
                year_str: row.get(3)?,
                track_title: row.get(4)?,
                disc_no: row.get(5)?,
                track_no: row.get(6)?,
                own_play_count: row.get(7)?,
                last_played: row.get(8)?,
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
                    match r.rating { Some(v) => float_arg(v), None => null_arg() },
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
                     (album_title, album_artist, year_str, track_title, disc_no, track_no,
                      machine_id, own_play_count, last_played, updated_at)
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                 ON CONFLICT(album_title, album_artist, year_str, track_title, disc_no, track_no, machine_id)
                 DO UPDATE SET
                     own_play_count = excluded.own_play_count,
                     last_played    = excluded.last_played,
                     updated_at     = excluded.updated_at"
                    .to_string(),
                vec![
                    text_arg(&p.album_title),
                    text_arg(&p.album_artist),
                    text_arg(&p.year_str),
                    text_arg(&p.track_title),
                    int_arg(p.disc_no),
                    int_arg(p.track_no),
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

/// Marks the given tracks as synced up to the count that was actually uploaded.
/// Uses the captured `own_play_count` from collection time, NOT the current
/// `own_play_count` — plays that arrive during the HTTP round-trip increment
/// `own_play_count` further, and those new plays must not be silently consumed.
pub fn mark_plays_synced(conn: &Connection, plays: &[PendingPlay]) -> Result<(), SyncError> {
    for p in plays {
        conn.execute(
            "UPDATE track_stats SET synced_play_count = ?2
             WHERE track_id = ?1 AND synced_play_count < ?2",
            rusqlite::params![p.track_id, p.own_play_count],
        )
        .map_err(|e| SyncError::Db(e.to_string()))?;
    }
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
            // Tombstone: write NULL rating with current timestamp so other machines
            // can learn about this deletion on their next pull.
            client
                .execute(
                    "INSERT INTO cloud_album_ratings
                         (album_title, album_artist, year_str, rating, updated_at)
                     VALUES (?, ?, ?, NULL, ?)
                     ON CONFLICT(album_title, album_artist, year_str) DO UPDATE SET
                         rating     = NULL,
                         updated_at = MAX(cloud_album_ratings.updated_at, excluded.updated_at)",
                    vec![
                        text_arg(title),
                        text_arg(artist),
                        text_arg(&year_str),
                        int_arg(updated_at),
                    ],
                )
                .await
        }
    }
}

fn unix_now() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs() as i64
}
