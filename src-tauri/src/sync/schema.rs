use super::{client::TursoClient, SyncError};

pub async fn ensure_cloud_schema(client: &TursoClient) -> Result<(), SyncError> {
    // ---- cloud_album_ratings ----
    // If the table already exists with NOT NULL rating (old schema), migrate it
    // in-place to allow NULL (needed for deletion tombstones), preserving all data.
    let ratings_not_null = client
        .query_scalar_int(
            "SELECT COUNT(*) FROM pragma_table_info('cloud_album_ratings') WHERE name='rating' AND notnull=1",
            vec![],
        )
        .await
        .unwrap_or(0);

    if ratings_not_null > 0 {
        client
            .execute_batch(vec![
                ("BEGIN".to_string(), vec![]),
                (
                    "CREATE TABLE cloud_album_ratings_new (
                        album_title  TEXT    NOT NULL,
                        album_artist TEXT    NOT NULL,
                        year_str     TEXT    NOT NULL DEFAULT '',
                        rating       REAL,
                        updated_at   INTEGER NOT NULL,
                        PRIMARY KEY (album_title, album_artist, year_str)
                    )"
                    .to_string(),
                    vec![],
                ),
                (
                    "INSERT INTO cloud_album_ratings_new
                     SELECT album_title, album_artist, year_str, rating, updated_at
                     FROM cloud_album_ratings"
                        .to_string(),
                    vec![],
                ),
                ("DROP TABLE cloud_album_ratings".to_string(), vec![]),
                (
                    "ALTER TABLE cloud_album_ratings_new RENAME TO cloud_album_ratings"
                        .to_string(),
                    vec![],
                ),
                ("COMMIT".to_string(), vec![]),
            ])
            .await?;
    } else {
        client
            .execute(
                "CREATE TABLE IF NOT EXISTS cloud_album_ratings (
                    album_title  TEXT    NOT NULL,
                    album_artist TEXT    NOT NULL,
                    year_str     TEXT    NOT NULL DEFAULT '',
                    rating       REAL,
                    updated_at   INTEGER NOT NULL,
                    PRIMARY KEY (album_title, album_artist, year_str)
                )",
                vec![],
            )
            .await?;
    }

    // ---- cloud_track_plays ----
    // Migrate from old path-keyed schema to natural-key schema if needed.
    let old_path_schema = client
        .query_scalar_int(
            "SELECT COUNT(*) FROM pragma_table_info('cloud_track_plays') WHERE name='track_path'",
            vec![],
        )
        .await
        .unwrap_or(0);
    if old_path_schema > 0 {
        client.execute("DROP TABLE cloud_track_plays", vec![]).await?;
    }

    client
        .execute(
            "CREATE TABLE IF NOT EXISTS cloud_track_plays (
                album_title  TEXT    NOT NULL,
                album_artist TEXT    NOT NULL,
                year_str     TEXT    NOT NULL DEFAULT '',
                track_title  TEXT    NOT NULL,
                disc_no      INTEGER NOT NULL DEFAULT 1,
                track_no     INTEGER NOT NULL DEFAULT 0,
                machine_id   TEXT    NOT NULL,
                own_play_count INTEGER NOT NULL DEFAULT 0,
                last_played    INTEGER,
                updated_at     INTEGER NOT NULL,
                PRIMARY KEY (album_title, album_artist, year_str, track_title, disc_no, track_no, machine_id)
            )",
            vec![],
        )
        .await
}
