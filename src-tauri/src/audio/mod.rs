mod decode;
mod engine;
mod queue;

pub use queue::Queue;

use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use arc_swap::ArcSwap;
use serde::Serialize;
use tauri::AppHandle;

use crate::mpris::Mpris;
use crate::state::DbPool;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PlaybackState {
    Stopped,
    Playing,
    Paused,
}

#[derive(Debug, Clone, Serialize)]
pub struct PlaybackSnapshot {
    pub state: PlaybackState,
    pub track_id: Option<i64>,
    pub position_ms: u64,
    pub duration_ms: u64,
    pub volume: f32,
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub album_id: Option<i64>,
    pub art_path: Option<String>,
}

impl Default for PlaybackSnapshot {
    fn default() -> Self {
        Self {
            state: PlaybackState::Stopped,
            track_id: None,
            position_ms: 0,
            duration_ms: 0,
            volume: 1.0,
            title: None,
            artist: None,
            album: None,
            album_id: None,
            art_path: None,
        }
    }
}

pub struct TrackInfo {
    pub track_id: i64,
    pub path: PathBuf,
    pub duration_ms: u64,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub album_id: Option<i64>,
    pub art_path: Option<String>,
}

pub enum PlayerCommand {
    SetQueue(Vec<TrackInfo>, usize),
    Play,
    Pause,
    Stop,
    Next,
    Previous,
    Seek(Duration),
    SetVolume(f32),
}

#[derive(Clone)]
pub struct PlayerHandle {
    tx: Sender<PlayerCommand>,
    snapshot: Arc<ArcSwap<PlaybackSnapshot>>,
}

impl PlayerHandle {
    pub fn send(&self, cmd: PlayerCommand) {
        let _ = self.tx.send(cmd);
    }

    pub fn snapshot(&self) -> PlaybackSnapshot {
        (**self.snapshot.load()).clone()
    }
}

/// Two-step construction resolves the circular dependency between the
/// engine (needs an `Mpris` to push state to) and `Mpris` (needs a
/// `PlayerHandle` to forward incoming OS media-key events to): build the
/// command channel and a `PlayerHandle` first, hand that to `Mpris::init`,
/// then spawn the engine thread with the receiving end.
pub struct EngineBuilder {
    tx: Sender<PlayerCommand>,
    rx: Receiver<PlayerCommand>,
    snapshot: Arc<ArcSwap<PlaybackSnapshot>>,
}

impl EngineBuilder {
    pub fn new() -> Self {
        let (tx, rx) = channel();
        let snapshot = Arc::new(ArcSwap::from_pointee(PlaybackSnapshot::default()));
        Self { tx, rx, snapshot }
    }

    pub fn handle(&self) -> PlayerHandle {
        PlayerHandle {
            tx: self.tx.clone(),
            snapshot: self.snapshot.clone(),
        }
    }

    pub fn spawn(self, app: AppHandle, mpris: Arc<Mpris>, db: DbPool) {
        thread::spawn(move || engine::run_engine(self.rx, self.snapshot, app, mpris, db));
    }
}
