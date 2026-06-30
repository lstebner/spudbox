use rusqlite::Connection;
use rusqlite_migration::{Migrations, M};

use crate::error::AppError;

pub fn run_migrations(conn: &mut Connection) -> Result<(), AppError> {
    let migrations = Migrations::new(vec![
        M::up(include_str!("../../migrations/0001_init.sql")),
        M::up(include_str!("../../migrations/0002_app_settings.sql")),
        M::up(include_str!("../../migrations/0003_album_ratings.sql")),
        M::up(include_str!("../../migrations/0004_sync.sql")),
        M::up(include_str!("../../migrations/0005_nullable_rating.sql")),
        M::up(include_str!("../../migrations/0006_hidden_albums.sql")),
    ]);
    migrations.to_latest(conn)?;
    Ok(())
}

/// An in-memory, fully-migrated connection for query-module unit tests, so
/// each test gets a fresh isolated schema without touching a real db file.
#[cfg(test)]
pub fn test_connection() -> Connection {
    let mut conn = Connection::open_in_memory().expect("open in-memory sqlite connection");
    run_migrations(&mut conn).expect("run migrations on test connection");
    conn
}
