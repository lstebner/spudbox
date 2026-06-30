use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager,
};

use crate::audio::{PlaybackSnapshot, PlaybackState, PlayerCommand, PlayerHandle};
use crate::state::AppState;

const POLL_INTERVAL: Duration = Duration::from_secs(1);

const MENU_ITEM_NOW_PLAYING_ID: &str = "now-playing";
const MENU_ITEM_PLAY_PAUSE_ID: &str = "play-pause";
const MENU_ITEM_PREVIOUS_ID: &str = "previous";
const MENU_ITEM_NEXT_ID: &str = "next";
const MENU_ITEM_MUTE_ID: &str = "mute";
const MENU_ITEM_SHOW_WINDOW_ID: &str = "show-window";
const MENU_ITEM_QUIT_ID: &str = "quit";

struct DynamicMenuItems {
    now_playing: MenuItem<tauri::Wry>,
    play_pause: MenuItem<tauri::Wry>,
    mute: MenuItem<tauri::Wry>,
}

// MenuItem<Wry> dispatches set_text to the main thread internally; sending the
// handle across threads is safe.
unsafe impl Send for DynamicMenuItems {}

pub fn setup_tray(app: &tauri::App) -> tauri::Result<()> {
    let now_playing = MenuItem::with_id(
        app,
        MENU_ITEM_NOW_PLAYING_ID,
        "Spudbox",
        false,
        None::<&str>,
    )?;
    let play_pause =
        MenuItem::with_id(app, MENU_ITEM_PLAY_PAUSE_ID, "Play", true, None::<&str>)?;
    let previous = MenuItem::with_id(app, MENU_ITEM_PREVIOUS_ID, "Previous", true, None::<&str>)?;
    let next = MenuItem::with_id(app, MENU_ITEM_NEXT_ID, "Next", true, None::<&str>)?;
    let mute = MenuItem::with_id(app, MENU_ITEM_MUTE_ID, "Mute", true, None::<&str>)?;
    let show_window =
        MenuItem::with_id(app, MENU_ITEM_SHOW_WINDOW_ID, "Show Window", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, MENU_ITEM_QUIT_ID, "Quit", true, None::<&str>)?;

    let menu = Menu::with_items(
        app,
        &[
            &now_playing,
            &PredefinedMenuItem::separator(app)?,
            &play_pause,
            &previous,
            &next,
            &mute,
            &PredefinedMenuItem::separator(app)?,
            &show_window,
            &PredefinedMenuItem::separator(app)?,
            &quit,
        ],
    )?;

    let mute_state: Arc<Mutex<Option<f32>>> = Arc::new(Mutex::new(None));
    let mute_state_for_handler = mute_state.clone();

    let tray = TrayIconBuilder::with_id("spudbox-tray")
        .icon(tauri::include_image!("icons/32x32.png"))
        .tooltip("Spudbox")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(move |app, event| {
            handle_menu_event(app, event.id.as_ref(), &mute_state_for_handler)
        })
        .on_tray_icon_event(|tray, event| handle_tray_event(tray, event))
        .build(app)?;

    // TrayIcon's Drop impl removes it from the OS tray; forget it to keep the tray alive.
    std::mem::forget(tray);

    let player = app.state::<AppState>().player.clone();
    spawn_update_thread(
        DynamicMenuItems {
            now_playing: now_playing.clone(),
            play_pause: play_pause.clone(),
            mute: mute.clone(),
        },
        player,
        mute_state,
    );

    Ok(())
}

fn handle_menu_event(
    app: &AppHandle<tauri::Wry>,
    id: &str,
    mute_state: &Arc<Mutex<Option<f32>>>,
) {
    let player = &app.state::<AppState>().player;
    match id {
        MENU_ITEM_PLAY_PAUSE_ID => {
            let command = if player.snapshot().state == PlaybackState::Playing {
                PlayerCommand::Pause
            } else {
                PlayerCommand::Play
            };
            player.send(command);
        }
        MENU_ITEM_PREVIOUS_ID => player.send(PlayerCommand::Previous),
        MENU_ITEM_NEXT_ID => player.send(PlayerCommand::Next),
        MENU_ITEM_MUTE_ID => {
            let mut locked = mute_state
                .lock()
                .expect("mute state mutex is not poisoned");
            match *locked {
                None => {
                    let current_volume = player.snapshot().volume;
                    *locked = Some(current_volume);
                    player.send(PlayerCommand::SetVolume(0.0));
                }
                Some(saved_volume) => {
                    player.send(PlayerCommand::SetVolume(saved_volume));
                    *locked = None;
                }
            }
        }
        MENU_ITEM_SHOW_WINDOW_ID => {
            let _ = show_and_focus_window(app);
        }
        MENU_ITEM_QUIT_ID => app.exit(0),
        _ => {}
    }
}

fn handle_tray_event(tray: &tauri::tray::TrayIcon<tauri::Wry>, event: TrayIconEvent) {
    if let TrayIconEvent::Click {
        button: MouseButton::Left,
        button_state: MouseButtonState::Up,
        ..
    } = event
    {
        let _ = show_and_focus_window(tray.app_handle());
    }
}

fn show_and_focus_window(app: &AppHandle<tauri::Wry>) -> tauri::Result<()> {
    if let Some(window) = app.get_webview_window("main") {
        window.show()?;
        window.unminimize()?;
    }
    Ok(())
}

fn spawn_update_thread(
    items: DynamicMenuItems,
    player: PlayerHandle,
    mute_state: Arc<Mutex<Option<f32>>>,
) {
    thread::spawn(move || {
        let mut last_now_playing = String::new();
        let mut last_play_pause = "";
        let mut last_mute = "";

        loop {
            thread::sleep(POLL_INTERVAL);

            let snapshot = player.snapshot();
            let is_muted = mute_state
                .lock()
                .expect("mute state mutex is not poisoned")
                .is_some();

            let now_playing = build_now_playing_text(&snapshot);
            if now_playing != last_now_playing {
                let _ = items.now_playing.set_text(&now_playing);
                last_now_playing = now_playing;
            }

            let play_pause = build_play_pause_text(snapshot.state);
            if play_pause != last_play_pause {
                let _ = items.play_pause.set_text(play_pause);
                last_play_pause = play_pause;
            }

            let mute_text = build_mute_text(is_muted);
            if mute_text != last_mute {
                let _ = items.mute.set_text(mute_text);
                last_mute = mute_text;
            }
        }
    });
}

fn build_now_playing_text(snapshot: &PlaybackSnapshot) -> String {
    match (&snapshot.title, &snapshot.artist) {
        (Some(title), Some(artist)) => format!("{title} \u{2014} {artist}"),
        (Some(title), None) => title.clone(),
        _ => "Spudbox".to_owned(),
    }
}

fn build_play_pause_text(state: PlaybackState) -> &'static str {
    match state {
        PlaybackState::Playing => "Pause",
        PlaybackState::Stopped | PlaybackState::Paused => "Play",
    }
}

fn build_mute_text(is_muted: bool) -> &'static str {
    if is_muted {
        "Unmute"
    } else {
        "Mute"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_snapshot(
        title: Option<&str>,
        artist: Option<&str>,
        state: PlaybackState,
    ) -> PlaybackSnapshot {
        PlaybackSnapshot {
            title: title.map(String::from),
            artist: artist.map(String::from),
            state,
            ..Default::default()
        }
    }

    #[test]
    fn now_playing_text_with_title_and_artist() {
        let snapshot = make_snapshot(
            Some("Comfortably Numb"),
            Some("Pink Floyd"),
            PlaybackState::Playing,
        );
        assert_eq!(
            build_now_playing_text(&snapshot),
            "Comfortably Numb \u{2014} Pink Floyd"
        );
    }

    #[test]
    fn now_playing_text_with_title_only() {
        let snapshot = make_snapshot(Some("Unknown Track"), None, PlaybackState::Playing);
        assert_eq!(build_now_playing_text(&snapshot), "Unknown Track");
    }

    #[test]
    fn now_playing_text_when_stopped() {
        let snapshot = make_snapshot(None, None, PlaybackState::Stopped);
        assert_eq!(build_now_playing_text(&snapshot), "Spudbox");
    }

    #[test]
    fn play_pause_text_when_playing() {
        assert_eq!(build_play_pause_text(PlaybackState::Playing), "Pause");
    }

    #[test]
    fn play_pause_text_when_paused() {
        assert_eq!(build_play_pause_text(PlaybackState::Paused), "Play");
    }

    #[test]
    fn play_pause_text_when_stopped() {
        assert_eq!(build_play_pause_text(PlaybackState::Stopped), "Play");
    }

    #[test]
    fn mute_text_when_muted() {
        assert_eq!(build_mute_text(true), "Unmute");
    }

    #[test]
    fn mute_text_when_not_muted() {
        assert_eq!(build_mute_text(false), "Mute");
    }
}
