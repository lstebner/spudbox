use std::path::Path;
use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::sync::Arc;
use std::time::Duration;

use arc_swap::ArcSwap;
use rodio::{OutputStream, OutputStreamHandle, Sink};
use souvlaki::{MediaMetadata, MediaPlayback, MediaPosition};
use tauri::{AppHandle, Emitter};

use super::decode::{DecodeError, FileSource};
use super::{PlaybackSnapshot, PlaybackState, PlayerCommand};
use crate::mpris::Mpris;

const PROGRESS_EVENT: &str = "playback-progress";
const TICK: Duration = Duration::from_millis(250);

struct EngineState {
    handle: OutputStreamHandle,
    sink: Option<Sink>,
    track_id: Option<i64>,
    duration_ms: u64,
    volume: f32,
    title: Option<String>,
    artist: Option<String>,
    album: Option<String>,
    art_path: Option<String>,
}

pub(super) fn run_engine(
    rx: Receiver<PlayerCommand>,
    snapshot: Arc<ArcSwap<PlaybackSnapshot>>,
    app: AppHandle,
    mpris: Arc<Mpris>,
) {
    let (_stream, handle) = match OutputStream::try_default() {
        Ok(v) => v,
        Err(e) => {
            eprintln!("failed to open default audio output: {e}");
            return;
        }
    };

    let mut state = EngineState {
        handle,
        sink: None,
        track_id: None,
        duration_ms: 0,
        volume: 1.0,
        title: None,
        artist: None,
        album: None,
        art_path: None,
    };

    loop {
        match rx.recv_timeout(TICK) {
            Ok(cmd) => handle_command(&mut state, cmd, &mpris),
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => break,
        }

        if let Some(sink) = &state.sink {
            if sink.empty() && state.track_id.is_some() {
                state.track_id = None;
                state.duration_ms = 0;
                state.title = None;
                state.artist = None;
                state.album = None;
                state.art_path = None;
                mpris.set_playback(MediaPlayback::Stopped);
            }
        }

        let snap = build_snapshot(&state);
        snapshot.store(Arc::new(snap.clone()));
        let _ = app.emit(PROGRESS_EVENT, snap);
    }
}

fn handle_command(state: &mut EngineState, cmd: PlayerCommand, mpris: &Mpris) {
    match cmd {
        PlayerCommand::PlayPath(track) => match load_source(&track.path) {
            Ok(source) => match Sink::try_new(&state.handle) {
                Ok(sink) => {
                    sink.set_volume(state.volume);
                    sink.append(source);
                    sink.play();
                    state.sink = Some(sink);
                    state.track_id = Some(track.track_id);
                    state.duration_ms = track.duration_ms;

                    mpris.set_metadata(MediaMetadata {
                        title: Some(&track.title),
                        artist: Some(&track.artist),
                        album: Some(&track.album),
                        duration: Some(Duration::from_millis(track.duration_ms)),
                        cover_url: None,
                    });
                    mpris.set_playback(MediaPlayback::Playing {
                        progress: Some(MediaPosition(Duration::ZERO)),
                    });

                    state.title = Some(track.title);
                    state.artist = Some(track.artist);
                    state.album = Some(track.album);
                    state.art_path = track.art_path;
                }
                Err(e) => eprintln!("failed to create audio sink: {e}"),
            },
            Err(e) => eprintln!("failed to decode {:?}: {e}", track.path),
        },
        PlayerCommand::Play => {
            if let Some(sink) = &state.sink {
                sink.play();
                mpris.set_playback(MediaPlayback::Playing {
                    progress: Some(MediaPosition(sink.get_pos())),
                });
            }
        }
        PlayerCommand::Pause => {
            if let Some(sink) = &state.sink {
                sink.pause();
                mpris.set_playback(MediaPlayback::Paused {
                    progress: Some(MediaPosition(sink.get_pos())),
                });
            }
        }
        PlayerCommand::Seek(pos) => {
            if let Some(sink) = &state.sink {
                if let Err(e) = sink.try_seek(pos) {
                    eprintln!("seek failed: {e}");
                }
            }
        }
        PlayerCommand::SetVolume(volume) => {
            state.volume = volume;
            if let Some(sink) = &state.sink {
                sink.set_volume(volume);
            }
        }
    }
}

fn load_source(path: &Path) -> Result<FileSource, DecodeError> {
    FileSource::open(path)
}

fn build_snapshot(state: &EngineState) -> PlaybackSnapshot {
    let position_ms = state
        .sink
        .as_ref()
        .map(|s| s.get_pos().as_millis() as u64)
        .unwrap_or(0);
    let playback_state = match &state.sink {
        None => PlaybackState::Stopped,
        Some(sink) if sink.is_paused() => PlaybackState::Paused,
        Some(_) => PlaybackState::Playing,
    };
    PlaybackSnapshot {
        state: playback_state,
        track_id: state.track_id,
        position_ms,
        duration_ms: state.duration_ms,
        volume: state.volume,
        title: state.title.clone(),
        artist: state.artist.clone(),
        album: state.album.clone(),
        art_path: state.art_path.clone(),
    }
}
