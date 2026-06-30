use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use arc_swap::ArcSwap;
use rodio::{OutputStream, OutputStreamHandle, Sink};
use souvlaki::{MediaMetadata, MediaPlayback, MediaPosition};
use tauri::{AppHandle, Emitter};

use super::decode::FileSource;
use super::{PlaybackSnapshot, PlaybackState, PlayerCommand, Queue, TrackInfo};
use crate::db::queries::{settings, stats, tracks};
use crate::mpris::Mpris;
use crate::state::DbPool;

const PROGRESS_EVENT: &str = "playback-progress";
const TICK: Duration = Duration::from_millis(250);
/// How often to checkpoint the session (queue/position) to disk during
/// otherwise-uneventful playback, so an ungraceful exit only loses a few
/// seconds of resume accuracy rather than falling back to wherever the
/// current track last started.
const SESSION_CHECKPOINT_TICKS: u64 = 40; // ~10s at the 250ms tick above

const SETTING_VOLUME: &str = "volume";
const SETTING_LAST_QUEUE: &str = "last_queue";
const SETTING_LAST_QUEUE_INDEX: &str = "last_queue_index";
const SETTING_LAST_POSITION_MS: &str = "last_position_ms";

struct EngineState {
    handle: OutputStreamHandle,
    sink: Option<Sink>,
    queue: Option<Queue>,
    last_sink_len: usize,
    volume: f32,
    db: DbPool,
    tick_count: u64,
}

pub(super) fn run_engine(
    rx: Receiver<PlayerCommand>,
    snapshot: Arc<ArcSwap<PlaybackSnapshot>>,
    app: AppHandle,
    mpris: Arc<Mpris>,
    db: DbPool,
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
        db,
        tick_count: 0,
    };

    restore_session(&mut state, &mpris);

    loop {
        match rx.recv_timeout(TICK) {
            Ok(cmd) => handle_command(&mut state, cmd, &mpris),
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => break,
        }

        poll_queue_advance(&mut state, &mpris);

        state.tick_count += 1;
        let is_playing = state.sink.as_ref().is_some_and(|s| !s.is_paused());
        if is_playing && state.tick_count % SESSION_CHECKPOINT_TICKS == 0 {
            persist_session(&state);
        }

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
            announce_current(queue, mpris, true);
            if let Some(next) = queue.peek_next() {
                append_track(sink, next);
            }
        }
        record_current_play(state);
        persist_session(state);
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
            start_playback_from_queue(state, mpris, true, None);
            record_current_play(state);
            persist_session(state);
        }
        PlayerCommand::Play => {
            if let Some(sink) = &state.sink {
                sink.play();
                mpris.set_playback(MediaPlayback::Playing {
                    progress: Some(MediaPosition(sink.get_pos())),
                });
            }
            persist_session(state);
        }
        PlayerCommand::Pause => {
            if let Some(sink) = &state.sink {
                sink.pause();
                mpris.set_playback(MediaPlayback::Paused {
                    progress: Some(MediaPosition(sink.get_pos())),
                });
            }
            persist_session(state);
        }
        PlayerCommand::Stop => {
            state.sink = None;
            state.queue = None;
            state.last_sink_len = 0;
            mpris.set_playback(MediaPlayback::Stopped);
            clear_session(state);
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
                start_playback_from_queue(state, mpris, true, None);
                record_current_play(state);
                persist_session(state);
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
            persist_session(state);
        }
    }
}

/// Creates a fresh sink for `state.queue`'s current position, appends the
/// current track plus (if any) the next one so the sink is always one
/// track ahead, and starts (or, for session restore, leaves paused at a
/// given position) playback.
fn start_playback_from_queue(state: &mut EngineState, mpris: &Mpris, autoplay: bool, seek_to: Option<Duration>) {
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
        if autoplay {
            sink.play();
        } else {
            sink.pause();
        }
        if let Some(pos) = seek_to {
            let _ = sink.try_seek(pos);
        }
        announce_current(queue, mpris, autoplay);
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

fn announce_current(queue: &Queue, mpris: &Mpris, playing: bool) {
    let Some(track) = queue.current() else { return };
    mpris.set_metadata(MediaMetadata {
        title: Some(&track.title),
        artist: Some(&track.artist),
        album: Some(&track.album),
        duration: Some(Duration::from_millis(track.duration_ms)),
        cover_url: None,
    });
    let progress = Some(MediaPosition(Duration::ZERO));
    mpris.set_playback(if playing {
        MediaPlayback::Playing { progress }
    } else {
        MediaPlayback::Paused { progress }
    });
}

fn record_current_play(state: &EngineState) {
    let Some(track_id) = state.queue.as_ref().and_then(|q| q.current()).map(|t| t.track_id) else {
        return;
    };
    let Ok(conn) = state.db.get() else { return };
    let played_at = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64;
    if let Err(e) = stats::record_play(&conn, track_id, played_at) {
        eprintln!("failed to record play for track {track_id}: {e}");
    }
}

fn persist_session(state: &EngineState) {
    let Ok(conn) = state.db.get() else { return };
    let _ = settings::set(&conn, SETTING_VOLUME, &state.volume.to_string());

    let Some(queue) = &state.queue else { return };
    let ids = queue.track_ids();
    let Ok(ids_json) = serde_json::to_string(&ids) else { return };
    let position_ms = state.sink.as_ref().map(|s| s.get_pos().as_millis() as u64).unwrap_or(0);

    let _ = settings::set(&conn, SETTING_LAST_QUEUE, &ids_json);
    let _ = settings::set(&conn, SETTING_LAST_QUEUE_INDEX, &queue.index().to_string());
    let _ = settings::set(&conn, SETTING_LAST_POSITION_MS, &position_ms.to_string());
}

fn clear_session(state: &EngineState) {
    let Ok(conn) = state.db.get() else { return };
    let _ = settings::set(&conn, SETTING_LAST_QUEUE, "");
    let _ = settings::set(&conn, SETTING_LAST_QUEUE_INDEX, "0");
    let _ = settings::set(&conn, SETTING_LAST_POSITION_MS, "0");
}

/// Restores volume and, if a previous queue was saved, reconstructs it and
/// leaves it paused at the last known position — never autoplays on
/// launch. Deliberately does not call `record_current_play`: restoring a
/// session isn't a new play of that track.
fn restore_session(state: &mut EngineState, mpris: &Mpris) {
    let Ok(conn) = state.db.get() else { return };

    if let Ok(Some(vol)) = settings::get(&conn, SETTING_VOLUME) {
        if let Ok(vol) = vol.parse::<f32>() {
            state.volume = vol;
        }
    }

    let Ok(Some(ids_json)) = settings::get(&conn, SETTING_LAST_QUEUE) else { return };
    let Ok(track_ids) = serde_json::from_str::<Vec<i64>>(&ids_json) else { return };
    if track_ids.is_empty() {
        return;
    }
    let index: usize = settings::get(&conn, SETTING_LAST_QUEUE_INDEX)
        .ok()
        .flatten()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    let position_ms: u64 = settings::get(&conn, SETTING_LAST_POSITION_MS)
        .ok()
        .flatten()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);

    let Ok(playable) = tracks::get_playable_batch(&conn, &track_ids) else { return };
    if playable.is_empty() {
        return;
    }
    drop(conn);

    let queue_tracks: Vec<TrackInfo> = playable
        .into_iter()
        .map(|t| TrackInfo {
            track_id: t.id,
            path: t.path.into(),
            duration_ms: t.duration_ms as u64,
            title: t.title,
            artist: t.artist,
            album: t.album,
            album_id: t.album_id,
            art_path: t.art_path,
        })
        .collect();

    state.queue = Some(Queue::new(queue_tracks, index));
    start_playback_from_queue(state, mpris, false, Some(Duration::from_millis(position_ms)));
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
        album_id: current.and_then(|t| t.album_id),
        art_path: current.and_then(|t| t.art_path.clone()),
    }
}
