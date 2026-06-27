use rusqlite::{params, Connection};

use crate::error::AppError;

fn unix_now() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs() as i64
}

/// `None` deletes the row (unrated); `Some(r)` upserts it. Absence of a row
/// is the unrated sentinel — see migration 0003 for why this isn't a
/// nullable column instead. `updated_at` is stamped automatically for
/// cloud LWW conflict resolution.
pub fn set_rating(conn: &Connection, album_id: i64, rating: Option<f64>) -> Result<i64, AppError> {
    let now = unix_now();
    match rating {
        Some(r) => conn.execute(
            "INSERT INTO album_ratings (album_id, rating, updated_at) VALUES (?1, ?2, ?3)
             ON CONFLICT(album_id) DO UPDATE SET
                 rating = excluded.rating,
                 updated_at = excluded.updated_at",
            params![album_id, r, now],
        )?,
        // Write a tombstone row (rating=NULL, updated_at=now) instead of deleting the row,
        // so cloud sync can propagate the unrating to other machines via the updated_at LWW.
        None => conn.execute(
            "INSERT INTO album_ratings (album_id, rating, updated_at) VALUES (?1, NULL, ?2)
             ON CONFLICT(album_id) DO UPDATE SET rating = NULL, updated_at = excluded.updated_at",
            params![album_id, now],
        )?,
    };
    Ok(now)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::queries::{albums, artists};
    use crate::db::schema::test_connection;

    fn setup_album(conn: &Connection) -> i64 {
        let artist_id = artists::upsert(conn, "Thrice").unwrap();
        albums::upsert(conn, "Vheissu", artist_id, Some(2005)).unwrap()
    }

    #[test]
    fn set_rating_then_list_all_reflects_it() {
        let conn = test_connection();
        let album_id = setup_album(&conn);
        set_rating(&conn, album_id, Some(8.5)).unwrap();
        let rows = albums::list_all(&conn, None).unwrap();
        assert_eq!(rows[0].rating, Some(8.5));
    }

    #[test]
    fn set_rating_returns_a_nonzero_timestamp() {
        let conn = test_connection();
        let album_id = setup_album(&conn);
        let ts = set_rating(&conn, album_id, Some(5.0)).unwrap();
        assert!(ts > 0, "timestamp should be a real unix timestamp");
    }

    #[test]
    fn set_rating_overwrites_previous_value() {
        let conn = test_connection();
        let album_id = setup_album(&conn);
        set_rating(&conn, album_id, Some(3.0)).unwrap();
        set_rating(&conn, album_id, Some(9.5)).unwrap();
        let rows = albums::list_all(&conn, None).unwrap();
        assert_eq!(rows[0].rating, Some(9.5));
    }

    #[test]
    fn set_rating_none_clears_to_unrated() {
        let conn = test_connection();
        let album_id = setup_album(&conn);
        set_rating(&conn, album_id, Some(5.0)).unwrap();
        set_rating(&conn, album_id, None).unwrap();
        let rows = albums::list_all(&conn, None).unwrap();
        assert_eq!(rows[0].rating, None);
    }

    #[test]
    fn unrated_album_has_no_row_and_list_all_returns_none() {
        let conn = test_connection();
        setup_album(&conn);
        let rows = albums::list_all(&conn, None).unwrap();
        assert_eq!(rows[0].rating, None);
    }
}
