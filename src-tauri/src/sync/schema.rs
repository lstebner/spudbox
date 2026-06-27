use super::{client::TursoClient, SyncError};

pub async fn ensure_cloud_schema(client: &TursoClient) -> Result<(), SyncError> {
    client
        .execute_batch(vec![
            (
                "CREATE TABLE IF NOT EXISTS cloud_album_ratings (
                    album_title  TEXT    NOT NULL,
                    album_artist TEXT    NOT NULL,
                    year_str     TEXT    NOT NULL DEFAULT '',
                    rating       REAL    NOT NULL,
                    updated_at   INTEGER NOT NULL,
                    PRIMARY KEY (album_title, album_artist, year_str)
                )"
                .to_string(),
                vec![],
            ),
            (
                "CREATE TABLE IF NOT EXISTS cloud_track_plays (
                    track_path     TEXT    NOT NULL,
                    machine_id     TEXT    NOT NULL,
                    own_play_count INTEGER NOT NULL DEFAULT 0,
                    last_played    INTEGER,
                    updated_at     INTEGER NOT NULL,
                    PRIMARY KEY (track_path, machine_id)
                )"
                .to_string(),
                vec![],
            ),
        ])
        .await
}
