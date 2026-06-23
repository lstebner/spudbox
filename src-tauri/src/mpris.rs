use std::sync::Mutex;
use std::time::Duration;

use souvlaki::{
    Error as SouvlakiError, MediaControlEvent, MediaControls, MediaMetadata, MediaPlayback,
    MediaPosition, PlatformConfig, SeekDirection,
};

use crate::audio::{PlaybackState, PlayerCommand, PlayerHandle};

pub struct Mpris {
    controls: Mutex<MediaControls>,
}

impl Mpris {
    pub fn init(player: PlayerHandle) -> Result<Self, SouvlakiError> {
        let config = PlatformConfig {
            dbus_name: "com.lukestebner.musicplayer",
            display_name: "Music Player",
            hwnd: None,
        };

        let mut controls = MediaControls::new(config)?;

        controls.attach(move |event: MediaControlEvent| {
            handle_event(&player, event);
        })?;

        Ok(Self {
            controls: Mutex::new(controls),
        })
    }

    pub fn set_playback(&self, playback: MediaPlayback) {
        if let Ok(mut controls) = self.controls.lock() {
            let _ = controls.set_playback(playback);
        }
    }

    pub fn set_metadata(&self, metadata: MediaMetadata) {
        if let Ok(mut controls) = self.controls.lock() {
            let _ = controls.set_metadata(metadata);
        }
    }
}

fn handle_event(player: &PlayerHandle, event: MediaControlEvent) {
    match event {
        MediaControlEvent::Play => player.send(PlayerCommand::Play),
        MediaControlEvent::Pause => player.send(PlayerCommand::Pause),
        MediaControlEvent::Toggle => {
            let playing = player.snapshot().state == PlaybackState::Playing;
            player.send(if playing {
                PlayerCommand::Pause
            } else {
                PlayerCommand::Play
            });
        }
        MediaControlEvent::SetPosition(MediaPosition(pos)) => {
            player.send(PlayerCommand::Seek(pos));
        }
        MediaControlEvent::Seek(direction) => {
            let pos = Duration::from_millis(player.snapshot().position_ms);
            player.send(PlayerCommand::Seek(seek_by(pos, direction, Duration::from_secs(10))));
        }
        MediaControlEvent::SeekBy(direction, amount) => {
            let pos = Duration::from_millis(player.snapshot().position_ms);
            player.send(PlayerCommand::Seek(seek_by(pos, direction, amount)));
        }
        MediaControlEvent::SetVolume(volume) => {
            player.send(PlayerCommand::SetVolume(volume as f32));
        }
        MediaControlEvent::Next => player.send(PlayerCommand::Next),
        MediaControlEvent::Previous => player.send(PlayerCommand::Previous),
        MediaControlEvent::Stop
        | MediaControlEvent::OpenUri(_)
        | MediaControlEvent::Raise
        | MediaControlEvent::Quit => {}
    }
}

fn seek_by(pos: Duration, direction: SeekDirection, amount: Duration) -> Duration {
    match direction {
        SeekDirection::Forward => pos + amount,
        SeekDirection::Backward => pos.saturating_sub(amount),
    }
}
