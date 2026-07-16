mod analysis;
mod decode;
mod engine;
pub mod eq;
mod queue;

pub use analysis::{AnalysisSource, NUM_BANDS, WAVEFORM_SAMPLES};
pub use eq::{EqGains, EqualizerSource, EQ_BAND_COUNT};
pub use queue::Queue;

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU32};
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
    pub rms_amplitude: f32,
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
            rms_amplitude: 0.0,
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
    SetEq([f32; EQ_BAND_COUNT], bool),
}

#[derive(Clone)]
pub struct PlayerHandle {
    tx: Sender<PlayerCommand>,
    snapshot: Arc<ArcSwap<PlaybackSnapshot>>,
    eq: Arc<ArcSwap<EqGains>>,
    pub visualizer_enabled: Arc<AtomicBool>,
}

impl PlayerHandle {
    pub fn send(&self, cmd: PlayerCommand) {
        let _ = self.tx.send(cmd);
    }

    pub fn snapshot(&self) -> PlaybackSnapshot {
        (**self.snapshot.load()).clone()
    }

    pub fn eq_state(&self) -> Arc<EqGains> {
        self.eq.load_full()
    }

    pub fn enable_visualizer(&self) {
        self.visualizer_enabled.store(true, std::sync::atomic::Ordering::Relaxed);
    }

    pub fn disable_visualizer(&self) {
        self.visualizer_enabled.store(false, std::sync::atomic::Ordering::Relaxed);
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
    eq: Arc<ArcSwap<EqGains>>,
    rms: Arc<AtomicU32>,
    fft_bands: Arc<ArcSwap<Vec<f32>>>,
    waveform_samples: Arc<ArcSwap<Vec<f32>>>,
    visualizer_enabled: Arc<AtomicBool>,
}

impl EngineBuilder {
    pub fn new() -> Self {
        let (tx, rx) = channel();
        let snapshot = Arc::new(ArcSwap::from_pointee(PlaybackSnapshot::default()));
        let eq = Arc::new(ArcSwap::from_pointee(EqGains::default()));
        let rms = Arc::new(AtomicU32::new(0.0f32.to_bits()));
        let fft_bands = Arc::new(ArcSwap::from_pointee(vec![0.0f32; NUM_BANDS]));
        let waveform_samples = Arc::new(ArcSwap::from_pointee(vec![0.0f32; WAVEFORM_SAMPLES]));
        let visualizer_enabled = Arc::new(AtomicBool::new(false));
        Self { tx, rx, snapshot, eq, rms, fft_bands, waveform_samples, visualizer_enabled }
    }

    pub fn handle(&self) -> PlayerHandle {
        PlayerHandle {
            tx: self.tx.clone(),
            snapshot: self.snapshot.clone(),
            eq: self.eq.clone(),
            visualizer_enabled: self.visualizer_enabled.clone(),
        }
    }

    pub fn spawn(self, app: AppHandle, mpris: Arc<Mpris>, db: DbPool) {
        thread::spawn(move || {
            engine::run_engine(
                self.rx,
                self.snapshot,
                app,
                mpris,
                db,
                self.eq,
                self.rms,
                self.fft_bands,
                self.waveform_samples,
                self.visualizer_enabled,
            )
        });
    }
}
