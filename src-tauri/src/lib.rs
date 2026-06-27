mod audio;
mod commands;
mod db;
mod error;
mod events;
mod mpris;
mod scanner;
mod state;
mod sync;

use std::sync::Arc;

use tauri::Manager;

use mpris::Mpris;
use state::AppState;
use sync::{SyncConfig, SyncManager};

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

            // Ensure this machine has a stable identifier, then kick off a
            // background sync if cloud credentials are configured.
            {
                let conn = pool.get().expect("failed to get db connection for sync setup");
                let config = SyncConfig::from_db(&conn).expect("failed to read sync config");
                if let Some(config) = config {
                    let sync_db = pool.clone();
                    tauri::async_runtime::spawn(async move {
                        match SyncManager::new(config, sync_db).sync().await {
                            Ok(stats) => eprintln!("[sync] startup sync done: {stats:?}"),
                            Err(e) => eprintln!("[sync] startup sync failed: {e}"),
                        }
                    });
                }
            }

            let engine_builder = audio::EngineBuilder::new();
            let player = engine_builder.handle();
            let mpris = Arc::new(Mpris::init(player.clone()).expect("failed to init mpris"));
            engine_builder.spawn(app.handle().clone(), mpris, pool.clone());

            app.manage(AppState { db: pool, player });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::library::ping,
            commands::library::library_add_root,
            commands::library::library_has_roots,
            commands::library::library_list_roots,
            commands::library::library_remove_root,
            commands::library::library_scan,
            commands::library::library_get_tracks,
            commands::library::library_get_tracks_by_album,
            commands::library::library_get_artists,
            commands::library::library_get_albums,
            commands::library::library_set_album_rating,
            commands::sync::sync_configure,
            commands::sync::sync_status,
            commands::sync::sync_now,
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
