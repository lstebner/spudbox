use rusqlite::{params, Connection};

use crate::error::AppError;

pub fn hide(conn: &Connection, album_id: i64) -> Result<(), AppError> {
    conn.execute(
        "INSERT OR IGNORE INTO hidden_albums (album_id) VALUES (?1)",
        params![album_id],
    )?;
    Ok(())
}

pub fn unhide(conn: &Connection, album_id: i64) -> Result<(), AppError> {
    conn.execute(
        "DELETE FROM hidden_albums WHERE album_id = ?1",
        params![album_id],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::queries::{albums, artists};
    use crate::db::schema::test_connection;

    #[test]
    fn hide_inserts_album_id() {
        let conn = test_connection();
        let artist_id = artists::upsert(&conn, "Thrice").unwrap();
        let album_id = albums::upsert(&conn, "Vheissu", artist_id, Some(2005)).unwrap();
        hide(&conn, album_id).unwrap();
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM hidden_albums WHERE album_id = ?1",
                [album_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn hide_is_idempotent_for_already_hidden_album() {
        let conn = test_connection();
        let artist_id = artists::upsert(&conn, "Thrice").unwrap();
        let album_id = albums::upsert(&conn, "Vheissu", artist_id, Some(2005)).unwrap();
        hide(&conn, album_id).unwrap();
        hide(&conn, album_id).unwrap();
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM hidden_albums WHERE album_id = ?1",
                [album_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn unhide_removes_album_id() {
        let conn = test_connection();
        let artist_id = artists::upsert(&conn, "Thrice").unwrap();
        let album_id = albums::upsert(&conn, "Vheissu", artist_id, Some(2005)).unwrap();
        hide(&conn, album_id).unwrap();
        unhide(&conn, album_id).unwrap();
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM hidden_albums WHERE album_id = ?1",
                [album_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn unhide_is_idempotent_for_nonexistent_album() {
        let conn = test_connection();
        unhide(&conn, 999).unwrap();
    }
}
