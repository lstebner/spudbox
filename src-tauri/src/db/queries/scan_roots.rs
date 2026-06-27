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

pub fn remove(conn: &Connection, path: &str) -> Result<(), AppError> {
    let tx = conn.unchecked_transaction()?;
    // substr comparison avoids LIKE's _ and % wildcard interpretation, which
    // would silently match unintended paths if the folder name contained those chars.
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
    tx.execute("DELETE FROM scan_roots WHERE path = ?1", [path])?;
    tx.commit()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::schema::test_connection;

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
}
