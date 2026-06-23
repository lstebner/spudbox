mod commands;
mod db;
mod error;
mod state;

use tauri::Manager;

use state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let app_dir = app
                .path()
                .app_data_dir()
                .expect("failed to resolve app data dir");
            std::fs::create_dir_all(&app_dir).expect("failed to create app data dir");
            let db_path = app_dir.join("library.sqlite3");

            let pool = db::pool::create_pool(&db_path).expect("failed to create db pool");
            {
                let mut conn = pool.get().expect("failed to get db connection");
                db::schema::run_migrations(&mut conn).expect("failed to run migrations");
            }

            app.manage(AppState { db: pool });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![commands::library::ping])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
