use std::path::PathBuf;
use std::time::Duration;

use tauri::State;

use crate::audio::{EQ_BAND_COUNT, PlaybackSnapshot, PlayerCommand, TrackInfo};
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

#[derive(serde::Serialize)]
pub struct EqSnapshot {
    gains_db: Vec<f32>,
    enabled: bool,
}

fn parse_eq_gains(gains_db: Vec<f32>) -> Result<[f32; EQ_BAND_COUNT], AppError> {
    gains_db
        .try_into()
        .map_err(|_| AppError::InvalidArgument(format!("expected {EQ_BAND_COUNT} EQ band gains")))
}

#[tauri::command]
pub fn playback_set_eq(
    state: State<AppState>,
    gains_db: Vec<f32>,
    enabled: bool,
) -> Result<(), AppError> {
    let gains = parse_eq_gains(gains_db)?;
    state.player.send(PlayerCommand::SetEq(gains, enabled));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_eq_gains_accepts_exactly_eight_values() {
        let gains = parse_eq_gains(vec![1.0, -2.0, 3.0, 0.0, 0.0, -1.0, 2.0, 6.0]);
        assert!(gains.is_ok());
        assert_eq!(gains.unwrap()[2], 3.0);
    }

    #[test]
    fn parse_eq_gains_rejects_wrong_count() {
        assert!(parse_eq_gains(vec![]).is_err());
        assert!(parse_eq_gains(vec![0.0; 7]).is_err());
        assert!(parse_eq_gains(vec![0.0; 9]).is_err());
    }
}

#[tauri::command]
pub fn playback_get_eq(state: State<AppState>) -> EqSnapshot {
    let eq = state.player.eq_state();
    EqSnapshot { gains_db: eq.gains_db.to_vec(), enabled: eq.enabled }
}

#[tauri::command]
pub fn playback_enable_visualizer(state: State<AppState>) {
    state.player.enable_visualizer();
}

#[tauri::command]
pub fn playback_disable_visualizer(state: State<AppState>) {
    state.player.disable_visualizer();
}
