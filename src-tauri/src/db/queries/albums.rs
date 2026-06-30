use rusqlite::{params, OptionalExtension, Connection};
use serde::Serialize;

use crate::error::AppError;

#[derive(Debug, Serialize)]
pub struct AlbumRow {
    pub id: i64,
    pub title: String,
    pub album_artist: String,
    pub album_artist_id: i64,
    pub year: Option<i64>,
    pub art_path: Option<String>,
    pub rating: Option<f64>,
}

pub fn list_all(
    conn: &Connection,
    artist_id: Option<i64>,
    hidden_only: bool,
) -> Result<Vec<AlbumRow>, AppError> {
    let sql = if hidden_only {
        "SELECT al.id, al.title, ar.name, al.album_artist_id, al.year, al.art_path, art.rating
         FROM albums al
         JOIN hidden_albums ha ON ha.album_id = al.id
         LEFT JOIN artists ar ON ar.id = al.album_artist_id
         LEFT JOIN album_ratings art ON art.album_id = al.id
         WHERE ?1 IS NULL OR al.album_artist_id = ?1
         ORDER BY ar.sort_name, ar.name, al.year, al.title"
    } else {
        "SELECT al.id, al.title, ar.name, al.album_artist_id, al.year, al.art_path, art.rating
         FROM albums al
         LEFT JOIN artists ar ON ar.id = al.album_artist_id
         LEFT JOIN album_ratings art ON art.album_id = al.id
         LEFT JOIN hidden_albums ha ON ha.album_id = al.id
         WHERE ha.album_id IS NULL
           AND (?1 IS NULL OR al.album_artist_id = ?1)
         ORDER BY ar.sort_name, ar.name, al.year, al.title"
    };
    let mut stmt = conn.prepare(sql)?;
    let rows = stmt.query_map([artist_id], |row| {
        Ok(AlbumRow {
            id: row.get(0)?,
            title: row.get(1)?,
            album_artist: row.get::<_, Option<String>>(2)?.unwrap_or_default(),
            album_artist_id: row.get(3)?,
            year: row.get(4)?,
            art_path: row.get(5)?,
            rating: row.get(6)?,
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
    let id: i64 = conn.query_row(
        "INSERT INTO albums (title, album_artist_id, year) VALUES (?1, ?2, ?3)
         ON CONFLICT(title, album_artist_id, year) DO UPDATE SET title = excluded.title
         RETURNING id",
        params![title, album_artist_id, year],
        |row| row.get(0),
    )?;
    Ok(id)
}

/// Albums that have never had art extraction attempted (`art_source` is
/// only ever NULL before the first attempt, then 'embedded'/'folder'/'none').
pub fn list_missing_art(conn: &Connection) -> Result<Vec<i64>, AppError> {
    let mut stmt = conn.prepare("SELECT id FROM albums WHERE art_source IS NULL")?;
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
    use crate::db::queries::artists;
    use crate::db::schema::test_connection;

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
        upsert(&conn, "Vheissu", a, Some(2005)).unwrap();
        upsert(&conn, "Redeemer", b, Some(2002)).unwrap();

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
        super::super::hidden_albums::hide(&conn, hidden).unwrap();

        let rows = list_all(&conn, None, false).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].id, visible);
    }

    #[test]
    fn list_all_returns_only_hidden_albums_when_hidden_only_is_true() {
        let conn = test_connection();
        let artist_id = artists::upsert(&conn, "Thrice").unwrap();
        upsert(&conn, "Vheissu", artist_id, Some(2005)).unwrap();
        let hidden = upsert(&conn, "The Artist in the Ambulance", artist_id, Some(2003)).unwrap();
        super::super::hidden_albums::hide(&conn, hidden).unwrap();

        let rows = list_all(&conn, None, true).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].id, hidden);
    }

    #[test]
    fn missing_art_then_set_art_round_trips() {
        let conn = test_connection();
        let artist_id = artists::upsert(&conn, "Thrice").unwrap();
        let album_id = upsert(&conn, "Vheissu", artist_id, Some(2005)).unwrap();

        assert_eq!(list_missing_art(&conn).unwrap(), vec![album_id]);

        set_art(&conn, album_id, Some("/cache/art/1.jpg"), "embedded").unwrap();

        assert_eq!(list_missing_art(&conn).unwrap(), Vec::<i64>::new());
        let rows = list_all(&conn, None, false).unwrap();
        assert_eq!(rows[0].art_path.as_deref(), Some("/cache/art/1.jpg"));
    }
}
