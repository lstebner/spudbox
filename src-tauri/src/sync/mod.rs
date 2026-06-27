pub mod client;
pub mod pull;
pub mod push;
pub mod schema;

use serde::Serialize;

use crate::{db::queries::settings, state::DbPool};
use crate::error::AppError;
use client::TursoClient;

#[derive(Debug, Clone)]
pub struct SyncConfig {
    pub db_url: String,
    pub token: String,
    pub machine_id: String,
}

impl SyncConfig {
    pub fn from_db(conn: &rusqlite::Connection) -> Result<Option<Self>, AppError> {
        let machine_id = settings::ensure_machine_id(conn)?;
        let db_url = settings::get(conn, "sync_db_url")?;
        let token = settings::get(conn, "sync_token")?;
        match (db_url, token) {
            (Some(url), Some(tok)) if !url.is_empty() && !tok.is_empty() => {
                Ok(Some(Self { db_url: url, token: tok, machine_id }))
            }
            _ => Ok(None),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct SyncStats {
    pub ratings_pushed: usize,
    pub ratings_pulled: usize,
    pub plays_pushed: usize,
    pub plays_merged: usize,
}

pub struct SyncManager {
    config: SyncConfig,
    client: TursoClient,
    db: DbPool,
}

impl SyncManager {
    pub fn new(config: SyncConfig, db: DbPool) -> Self {
        let client = TursoClient::new(&config.db_url, &config.token);
        Self { config, client, db }
    }

    pub async fn sync(&self) -> Result<SyncStats, SyncError> {
        schema::ensure_cloud_schema(&self.client).await?;

        // Collect data to push (DB read, no await).
        let pending_ratings;
        let pending_plays;
        {
            let conn = self.db.get().map_err(|e| SyncError::Db(e.to_string()))?;
            pending_ratings = push::collect_pending_ratings(&conn)?;
            pending_plays = push::collect_pending_plays(&conn)?;
        }

        // Push first so our own counts are reflected before we pull
        // other machines' data (avoids double-counting).
        push::upload_ratings(&self.client, &pending_ratings).await?;
        push::upload_plays(&self.client, &pending_plays, &self.config.machine_id).await?;

        // Pull cloud state.
        let cloud_ratings = pull::fetch_cloud_ratings(&self.client).await?;
        let cloud_plays = pull::fetch_cloud_plays(&self.client, &self.config.machine_id).await?;

        // Apply to local DB and mark plays as synced.
        let ratings_pulled;
        let plays_merged;
        {
            let conn = self.db.get().map_err(|e| SyncError::Db(e.to_string()))?;
            ratings_pulled = pull::apply_ratings(&conn, &cloud_ratings)?;
            plays_merged = pull::apply_plays(&conn, &cloud_plays)?;
            push::mark_plays_synced(&conn, &pending_plays)?;
        }

        Ok(SyncStats {
            ratings_pushed: pending_ratings.len(),
            ratings_pulled,
            plays_pushed: pending_plays.len(),
            plays_merged,
        })
    }

    pub async fn push_one_album_rating(
        &self,
        title: &str,
        artist: &str,
        year: Option<i64>,
        rating: Option<f64>,
        updated_at: i64,
    ) -> Result<(), SyncError> {
        push::push_one_album_rating(&self.client, title, artist, year, rating, updated_at).await
    }
}

#[derive(Debug, thiserror::Error, Serialize)]
pub enum SyncError {
    #[error("HTTP error: {0}")]
    Http(String),
    #[error("API error: {0}")]
    Api(String),
    #[error("database error: {0}")]
    Db(String),
    #[error("unexpected response: {0}")]
    UnexpectedResponse(String),
}

impl From<reqwest::Error> for SyncError {
    fn from(e: reqwest::Error) -> Self {
        SyncError::Http(e.to_string())
    }
}
