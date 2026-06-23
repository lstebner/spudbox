use std::path::PathBuf;
use std::time::Duration;

use tauri::State;

use crate::audio::{PlaybackSnapshot, PlayerCommand, TrackInfo};
use crate::db::queries::tracks;
use crate::error::AppError;
use crate::state::AppState;

#[tauri::command]
pub fn playback_play_track(state: State<AppState>, track_id: i64) -> Result<(), AppError> {
    let track = {
        let conn = state.db.get()?;
        tracks::get_playable(&conn, track_id)?
    };
    state.player.send(PlayerCommand::PlayPath(TrackInfo {
        track_id,
        path: PathBuf::from(track.path),
        duration_ms: track.duration_ms as u64,
        title: track.title,
        artist: track.artist,
        album: track.album,
        art_path: track.art_path,
    }));
    Ok(())
}

#[tauri::command]
pub fn playback_play(state: State<AppState>) -> Result<(), AppError> {
    state.player.send(PlayerCommand::Play);
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
