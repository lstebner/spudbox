use rusqlite::Connection;
use rusqlite_migration::{Migrations, M};

use crate::error::AppError;

pub fn run_migrations(conn: &mut Connection) -> Result<(), AppError> {
    let migrations = Migrations::new(vec![M::up(include_str!("../../migrations/0001_init.sql"))]);
    migrations.to_latest(conn)?;
    Ok(())
}
