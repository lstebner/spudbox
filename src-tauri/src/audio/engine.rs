use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::sync::Arc;
use std::time::Duration;

use arc_swap::ArcSwap;
use rodio::{OutputStream, OutputStreamHandle, Sink};
use souvlaki::{MediaMetadata, MediaPlayback, MediaPosition};
use tauri::{AppHandle, Emitter};

use super::decode::FileSource;
use super::{PlaybackSnapshot, PlaybackState, PlayerCommand, Queue, TrackInfo};
use crate::mpris::Mpris;

const PROGRESS_EVENT: &str = "playback-progress";
const TICK: Duration = Duration::from_millis(250);

struct EngineState {
    handle: OutputStreamHandle,
    sink: Option<Sink>,
    queue: Option<Queue>,
    last_sink_len: usize,
    volume: f32,
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
        queue: None,
        last_sink_len: 0,
        volume: 1.0,
    };

    loop {
        match rx.recv_timeout(TICK) {
            Ok(cmd) => handle_command(&mut state, cmd, &mpris),
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => break,
        }

        poll_queue_advance(&mut state, &mpris);

        let snap = build_snapshot(&state);
        snapshot.store(Arc::new(snap.clone()));
        let _ = app.emit(PROGRESS_EVENT, snap);
    }
}

/// Detects track transitions (natural end-of-track, or a `skip_one()` from
/// `Next`) by watching the sink's internal queue length drop, and reacts by
/// advancing our own `Queue` to match and keeping the sink one track
/// pre-appended ahead. rodio's `Sink`/`SourcesQueueOutput` already hands off
/// between sequentially-appended sources at the sample level with no
/// inserted gap or silence (verified by reading its source), and the device
/// mixer wraps every appended source in a `UniformSourceIterator` that
/// re-resamples/re-channel-converts at each source boundary automatically —
/// so a queue of mixed sample rates/bit depths needs no manual handling
/// here, just keeping the next source appended in time.
fn poll_queue_advance(state: &mut EngineState, mpris: &Mpris) {
    let Some(sink) = &state.sink else { return };
    let current_len = sink.len();

    if current_len >= state.last_sink_len {
        state.last_sink_len = current_len;
        return;
    }

    let advanced = match &mut state.queue {
        Some(queue) => queue.advance().is_some(),
        None => false,
    };

    if advanced {
        if let Some(queue) = &state.queue {
            announce_current(queue, mpris);
            if let Some(next) = queue.peek_next() {
                append_track(sink, next);
            }
        }
    } else {
        state.queue = None;
        mpris.set_playback(MediaPlayback::Stopped);
    }
    state.last_sink_len = sink.len();
}

fn handle_command(state: &mut EngineState, cmd: PlayerCommand, mpris: &Mpris) {
    match cmd {
        PlayerCommand::SetQueue(tracks, start_index) => {
            if tracks.is_empty() {
                return;
            }
            state.queue = Some(Queue::new(tracks, start_index));
            start_playback_from_queue(state, mpris);
        }
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
        PlayerCommand::Next => {
            if let Some(sink) = &state.sink {
                sink.skip_one();
            }
        }
        PlayerCommand::Previous => {
            let moved = state
                .queue
                .as_mut()
                .map(|q| q.move_to_previous().is_some())
                .unwrap_or(false);
            if moved {
                start_playback_from_queue(state, mpris);
            } else if let Some(sink) = &state.sink {
                let _ = sink.try_seek(Duration::ZERO);
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

/// Creates a fresh sink for `state.queue`'s current position, appends the
/// current track plus (if any) the next one so the sink is always one
/// track ahead, and starts playback. Used both for a brand-new queue and
/// for restarting at a different position (`Previous`).
fn start_playback_from_queue(state: &mut EngineState, mpris: &Mpris) {
    let sink = match Sink::try_new(&state.handle) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("failed to create audio sink: {e}");
            return;
        }
    };
    sink.set_volume(state.volume);

    {
        let Some(queue) = &state.queue else { return };
        let Some(current) = queue.current() else { return };
        if !append_track(&sink, current) {
            return;
        }
        if let Some(next) = queue.peek_next() {
            append_track(&sink, next);
        }
        sink.play();
        announce_current(queue, mpris);
    }

    state.last_sink_len = sink.len();
    state.sink = Some(sink);
}

fn append_track(sink: &Sink, track: &TrackInfo) -> bool {
    match FileSource::open(&track.path) {
        Ok(source) => {
            sink.append(source);
            true
        }
        Err(e) => {
            eprintln!("failed to decode {:?}: {e}", track.path);
            false
        }
    }
}

fn announce_current(queue: &Queue, mpris: &Mpris) {
    let Some(track) = queue.current() else { return };
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
    let current = state.queue.as_ref().and_then(|q| q.current());
    PlaybackSnapshot {
        state: playback_state,
        track_id: current.map(|t| t.track_id),
        position_ms,
        duration_ms: current.map(|t| t.duration_ms).unwrap_or(0),
        volume: state.volume,
        title: current.map(|t| t.title.clone()),
        artist: current.map(|t| t.artist.clone()),
        album: current.map(|t| t.album.clone()),
        art_path: current.and_then(|t| t.art_path.clone()),
    }
}
