use std::path::PathBuf;
use std::time::Duration;

use tauri::State;

use crate::audio::{PlaybackSnapshot, PlayerCommand, TrackInfo};
use crate::db::queries::tracks;
use crate::error::AppError;
use crate::state::AppState;

#[tauri::command]
pub fn playback_play_queue(
    state: State<AppState>,
    track_ids: Vec<i64>,
    start_index: usize,
) -> Result<(), AppError> {
    let playable = {
        let conn = state.db.get()?;
        tracks::get_playable_batch(&conn, &track_ids)?
    };
    let queue: Vec<TrackInfo> = playable
        .into_iter()
        .map(|t| TrackInfo {
            track_id: t.id,
            path: PathBuf::from(t.path),
            duration_ms: t.duration_ms as u64,
            title: t.title,
            artist: t.artist,
            album: t.album,
            album_id: t.album_id,
            art_path: t.art_path,
        })
        .collect();
    state.player.send(PlayerCommand::SetQueue(queue, start_index));
    Ok(())
}

#[tauri::command]
pub fn playback_next(state: State<AppState>) -> Result<(), AppError> {
    state.player.send(PlayerCommand::Next);
    Ok(())
}

#[tauri::command]
pub fn playback_previous(state: State<AppState>) -> Result<(), AppError> {
    state.player.send(PlayerCommand::Previous);
    Ok(())
}

#[tauri::command]
pub fn playback_play(state: State<AppState>) -> Result<(), AppError> {
    state.player.send(PlayerCommand::Play);
    Ok(())
}

#[tauri::command]
pub fn playback_stop(state: State<AppState>) -> Result<(), AppError> {
    state.player.send(PlayerCommand::Stop);
    Ok(())
}

#[tauri::command]
pub fn playback_pause(state: State<AppState>) -> Result<(), AppError> {
    state.player.send(PlayerCommand::Pause);
    Ok(())
}

#[tauri::command]
pub fn playback_seek(state: State<AppState>, position_ms: u64) -> Result<(), AppError> {
    state
        .player
        .send(PlayerCommand::Seek(Duration::from_millis(position_ms)));
    Ok(())
}

#[tauri::command]
pub fn playback_set_volume(state: State<AppState>, volume: f32) -> Result<(), AppError> {
    state.player.send(PlayerCommand::SetVolume(volume));
    Ok(())
}

#[tauri::command]
pub fn playback_get_snapshot(state: State<AppState>) -> PlaybackSnapshot {
    state.player.snapshot()
}
