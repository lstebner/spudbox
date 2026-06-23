mod audio;
mod commands;
mod db;
mod error;
mod events;
mod mpris;
mod scanner;
mod state;

use std::sync::Arc;

use tauri::Manager;

use mpris::Mpris;
use state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
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

            let engine_builder = audio::EngineBuilder::new();
            let player = engine_builder.handle();
            let mpris = Arc::new(Mpris::init(player.clone()).expect("failed to init mpris"));
            engine_builder.spawn(app.handle().clone(), mpris);

            app.manage(AppState { db: pool, player });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::library::ping,
            commands::library::library_add_root,
            commands::library::library_has_roots,
            commands::library::library_scan,
            commands::library::library_get_tracks,
            commands::library::library_get_tracks_by_album,
            commands::library::library_get_artists,
            commands::library::library_get_albums,
            commands::playback::playback_play_queue,
            commands::playback::playback_play,
            commands::playback::playback_pause,
            commands::playback::playback_next,
            commands::playback::playback_previous,
            commands::playback::playback_seek,
            commands::playback::playback_set_volume,
            commands::playback::playback_get_snapshot
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
