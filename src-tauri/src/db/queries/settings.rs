use rusqlite::{OptionalExtension, Connection};

use crate::error::AppError;

pub fn get(conn: &Connection, key: &str) -> Result<Option<String>, AppError> {
    conn.query_row("SELECT value FROM app_settings WHERE key = ?1", [key], |row| row.get(0))
        .optional()
        .map_err(AppError::from)
}

pub fn set(conn: &Connection, key: &str, value: &str) -> Result<(), AppError> {
    conn.execute(
        "INSERT INTO app_settings (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        [key, value],
    )?;
    Ok(())
}

/// Returns the stable per-machine identifier, creating one on first call.
/// Reads `/etc/machine-id` (present on all systemd Linux installs); falls
/// back to a pid+nanos string if the file is absent.
pub fn ensure_machine_id(conn: &Connection) -> Result<String, AppError> {
    if let Some(id) = get(conn, "machine_id")? {
        if !id.is_empty() {
            return Ok(id);
        }
    }
    let id = std::fs::read_to_string("/etc/machine-id")
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|_| {
            use std::time::{SystemTime, UNIX_EPOCH};
            let nanos = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .subsec_nanos();
            format!("{:08x}{:08x}", std::process::id(), nanos)
        });
    set(conn, "machine_id", &id)?;
    Ok(id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::schema::test_connection;

    #[test]
    fn get_returns_none_for_an_unset_key() {
        let conn = test_connection();
        assert_eq!(get(&conn, "volume").unwrap(), None);
    }

    #[test]
    fn set_then_get_round_trips() {
        let conn = test_connection();
        set(&conn, "volume", "0.75").unwrap();
        assert_eq!(get(&conn, "volume").unwrap(), Some("0.75".to_string()));
    }

    #[test]
    fn set_overwrites_the_previous_value() {
        let conn = test_connection();
        set(&conn, "volume", "0.75").unwrap();
        set(&conn, "volume", "0.5").unwrap();
        assert_eq!(get(&conn, "volume").unwrap(), Some("0.5".to_string()));
    }
}
