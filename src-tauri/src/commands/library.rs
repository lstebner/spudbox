use tauri::State;

use crate::error::AppError;
use crate::state::AppState;

#[tauri::command]
pub fn ping(state: State<AppState>) -> Result<String, AppError> {
    let conn = state.db.get()?;
    let value: i64 = conn.query_row("SELECT 1", [], |row| row.get(0))?;
    Ok(format!("pong ({value})"))
}
