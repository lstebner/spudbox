use super::{client::TursoClient, SyncError};

pub async fn ensure_cloud_schema(client: &TursoClient) -> Result<(), SyncError> {
    client
        .execute(
            "CREATE TABLE IF NOT EXISTS cloud_album_ratings (
                album_title  TEXT    NOT NULL,
                album_artist TEXT    NOT NULL,
                year_str     TEXT    NOT NULL DEFAULT '',
                rating       REAL    NOT NULL,
                updated_at   INTEGER NOT NULL,
                PRIMARY KEY (album_title, album_artist, year_str)
            )",
            vec![],
        )
        .await?;

    // Migrate: if the old path-keyed table exists, drop it so we can
    // recreate it with the natural-key schema below.
    let old_schema = client
        .query_scalar_int(
            "SELECT COUNT(*) FROM pragma_table_info('cloud_track_plays') WHERE name='track_path'",
            vec![],
        )
        .await
        .unwrap_or(0);
    if old_schema > 0 {
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
