use tauri::State;

use crate::db::queries::settings;
use crate::error::AppError;
use crate::state::AppState;

const SETTING_THEME: &str = "theme";
const DEFAULT_THEME: &str = "dark";

fn resolve_theme(stored: Option<String>) -> String {
    stored.unwrap_or_else(|| DEFAULT_THEME.to_string())
}

/// Returns the saved theme name, or the default if none has been set yet.
#[tauri::command]
pub fn appearance_get_theme(state: State<AppState>) -> Result<String, AppError> {
    let conn = state.db.get()?;
    Ok(resolve_theme(settings::get(&conn, SETTING_THEME)?))
}

#[tauri::command]
pub fn appearance_set_theme(state: State<AppState>, theme: String) -> Result<(), AppError> {
    let conn = state.db.get()?;
    settings::set(&conn, SETTING_THEME, &theme)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_theme_falls_back_to_dark_when_unset() {
        assert_eq!(resolve_theme(None), "dark");
    }

    #[test]
    fn resolve_theme_returns_the_stored_value_when_present() {
        assert_eq!(resolve_theme(Some("green".to_string())), "green");
    }
}
