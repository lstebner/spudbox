use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use tauri::{AppHandle, Emitter};
use walkdir::WalkDir;

use crate::db::queries::{scan_roots, tracks};
use crate::error::AppError;
use crate::state::DbPool;

use super::{SyncEntry, SyncMode, SyncPreview, SyncPreviewProgress, SyncProgress};

const PREVIEW_PROGRESS_EVENT: &str = "device-preview-progress";
const SYNC_PROGRESS_EVENT: &str = "device-sync-progress";

/// How many audio files to find on the device between progress event emissions.
const PREVIEW_PROGRESS_INTERVAL: usize = 25;

const AUDIO_EXTENSIONS: &[&str] = &["flac", "mp3", "aac", "m4a", "wav", "ogg", "opus", "aiff"];

/// Computes what would change if the library were synced to `device_music_path`:
/// files to copy from the library and files to remove from the device.
/// Emits `device-preview-progress` events while walking the device filesystem.
pub fn preview_sync(
    device_music_path: &Path,
    db: &DbPool,
    app_handle: &AppHandle,
) -> Result<SyncPreview, AppError> {
    let conn = db.get()?;
    let roots = scan_roots::list_enabled(&conn)?;
    let library_tracks = tracks::active_paths_with_sizes(&conn)?;

    let library_map = build_library_map(&library_tracks, &roots);
    let device_map = walk_device(device_music_path, app_handle)?;

    let mut to_add: Vec<SyncEntry> = library_map
        .iter()
        .filter(|(path, _)| !device_map.contains_key(*path))
        .map(|(path, entry)| SyncEntry {
            relative_path: path.clone(),
            size_bytes: entry.size_bytes,
            artist: entry.artist.clone(),
            album: entry.album.clone(),
            title: entry.title.clone(),
        })
        .collect();
    to_add.sort_by(|a, b| {
        a.artist.cmp(&b.artist)
            .then(a.album.cmp(&b.album))
            .then(a.title.cmp(&b.title))
    });

    let mut to_delete: Vec<SyncEntry> = device_map
        .iter()
        .filter(|(path, _)| !library_map.contains_key(*path))
        .map(|(path, &size)| {
            let (artist, album, title) = parse_display_info(path);
            SyncEntry { relative_path: path.clone(), size_bytes: size, artist, album, title }
        })
        .collect();
    to_delete.sort_by(|a, b| {
        a.artist.cmp(&b.artist)
            .then(a.album.cmp(&b.album))
            .then(a.title.cmp(&b.title))
    });

    let required_bytes = to_add.iter().map(|e| e.size_bytes).sum();
    let device_free_bytes = available_bytes(device_music_path)?;

    Ok(SyncPreview { to_add, to_delete, device_free_bytes, required_bytes })
}

/// Performs the sync: copies additions to the device and, when `mode` is
/// `All`, also removes device files that are no longer in the library.
/// Emits `device-sync-progress` events throughout. Checks `cancel` between
/// every file operation; when set the sync stops cleanly without partial writes.
pub fn perform_sync(
    device_music_path: PathBuf,
    mode: SyncMode,
    db: DbPool,
    app_handle: AppHandle,
    cancel: Arc<AtomicBool>,
) -> Result<(), AppError> {
    let preview = preview_sync(&device_music_path, &db, &app_handle)?;

    let conn = db.get()?;
    let roots = scan_roots::list_enabled(&conn)?;
    drop(conn);

    let to_delete = if matches!(mode, SyncMode::All) { preview.to_delete } else { Vec::new() };
    let total = preview.to_add.len() + to_delete.len();
    let mut current = 0;

    for entry in &preview.to_add {
        if cancel.load(Ordering::Relaxed) {
            return Ok(());
        }

        let source = find_in_library_roots(&entry.relative_path, &roots);
        let Some(source) = source else {
            current += 1;
            continue;
        };

        let destination = device_music_path.join(&entry.relative_path);
        if let Some(parent) = destination.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::copy(&source, &destination)?;

        current += 1;
        let _ = app_handle.emit(SYNC_PROGRESS_EVENT, SyncProgress {
            current,
            total,
            current_file: format!("{} — {}", entry.artist, entry.title),
            phase: "copying".to_string(),
        });
    }

    for entry in &to_delete {
        if cancel.load(Ordering::Relaxed) {
            return Ok(());
        }

        let target = device_music_path.join(&entry.relative_path);
        if target.exists() {
            std::fs::remove_file(&target)?;
        }

        current += 1;
        let _ = app_handle.emit(SYNC_PROGRESS_EVENT, SyncProgress {
            current,
            total,
            current_file: format!("{} — {}", entry.artist, entry.title),
            phase: "deleting".to_string(),
        });
    }

    Ok(())
}

struct LibraryEntry {
    size_bytes: u64,
    artist: String,
    album: String,
    title: String,
}

/// Builds a map of `relative_path → LibraryEntry` for all active library
/// tracks, using each track's library root to compute the relative path.
fn build_library_map(
    track_list: &[tracks::TrackPathEntry],
    roots: &[String],
) -> HashMap<String, LibraryEntry> {
    let mut map = HashMap::new();
    for track in track_list {
        let track_path = Path::new(&track.path);
        let best_root = roots
            .iter()
            .filter(|root| track_path.starts_with(root.as_str()))
            .max_by_key(|root| root.len());
        if let Some(root) = best_root {
            if let Ok(relative) = track_path.strip_prefix(root) {
                map.insert(relative.to_string_lossy().into_owned(), LibraryEntry {
                    size_bytes: track.size_bytes,
                    artist: track.artist.clone(),
                    album: track.album.clone(),
                    title: track.title.clone(),
                });
            }
        }
    }
    map
}

/// Walks `device_music_path` and returns `relative_path → size_bytes` for
/// every audio file found. Returns an empty map if the path does not exist yet.
/// Emits `device-preview-progress` every `PREVIEW_PROGRESS_INTERVAL` files so
/// the frontend can show a running count while the MTP traversal is in progress.
fn walk_device(
    device_music_path: &Path,
    app_handle: &AppHandle,
) -> Result<HashMap<String, u64>, AppError> {
    if !device_music_path.exists() {
        return Ok(HashMap::new());
    }
    let mut map = HashMap::new();
    for entry in WalkDir::new(device_music_path).into_iter().flatten() {
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        let extension = path
            .extension()
            .map(|e| e.to_string_lossy().to_lowercase())
            .unwrap_or_default();
        if !AUDIO_EXTENSIONS.contains(&extension.as_str()) {
            continue;
        }
        if let Ok(relative) = path.strip_prefix(device_music_path) {
            let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
            map.insert(relative.to_string_lossy().into_owned(), size);

            if map.len() % PREVIEW_PROGRESS_INTERVAL == 0 {
                let _ = app_handle.emit(
                    PREVIEW_PROGRESS_EVENT,
                    SyncPreviewProgress { device_tracks_found: map.len() },
                );
            }
        }
    }
    Ok(map)
}

/// Derives artist, album, and title from a device-side relative path of the
/// form `Artist/Album/title.ext` or `Album/title.ext` or `title.ext`.
fn parse_display_info(relative_path: &str) -> (String, String, String) {
    let parts: Vec<&str> = relative_path.split('/').collect();
    let n = parts.len();

    let filename = parts.last().copied().unwrap_or(relative_path);
    let title = match filename.rfind('.') {
        Some(index) => &filename[..index],
        None => filename,
    }
    .to_string();

    let album = if n >= 2 { parts[n - 2].to_string() } else { String::new() };
    let artist = if n >= 3 { parts[n - 3].to_string() } else { String::new() };

    (artist, album, title)
}

/// Searches each library root for a file at `relative_path`, returning the
/// first absolute path that exists on disk.
fn find_in_library_roots(relative_path: &str, roots: &[String]) -> Option<PathBuf> {
    roots
        .iter()
        .map(|root| Path::new(root).join(relative_path))
        .find(|candidate| candidate.exists())
}

/// Returns available bytes on the filesystem containing `path`, walking up
/// to the nearest existing ancestor if `path` itself does not yet exist.
fn available_bytes(path: &Path) -> Result<u64, AppError> {
    let mut check_path = path.to_path_buf();
    while !check_path.exists() {
        match check_path.parent() {
            Some(parent) => check_path = parent.to_path_buf(),
            None => break,
        }
    }

    let output = std::process::Command::new("df")
        .args(["-B1", "--output=avail", &check_path.to_string_lossy()])
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let bytes = stdout
        .lines()
        .nth(1)
        .and_then(|line| line.trim().parse::<u64>().ok())
        .unwrap_or(0);

    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_track(path: &str, size: u64, artist: &str, album: &str, title: &str) -> tracks::TrackPathEntry {
        tracks::TrackPathEntry {
            path: path.to_string(),
            size_bytes: size,
            artist: artist.to_string(),
            album: album.to_string(),
            title: title.to_string(),
        }
    }

    #[test]
    fn build_library_map_strips_root_prefix() {
        let track_list = vec![make_track("/music/Artist/Album/01.flac", 10_000, "Artist", "Album", "Track 1")];
        let roots = vec!["/music".to_string()];
        let map = build_library_map(&track_list, &roots);
        assert!(map.contains_key("Artist/Album/01.flac"));
        assert_eq!(map["Artist/Album/01.flac"].size_bytes, 10_000);
    }

    #[test]
    fn build_library_map_uses_longest_matching_root() {
        let track_list = vec![make_track("/music/special/track.flac", 5_000, "A", "B", "C")];
        let roots = vec!["/music".to_string(), "/music/special".to_string()];
        let map = build_library_map(&track_list, &roots);
        assert!(map.contains_key("track.flac"));
        assert!(!map.contains_key("special/track.flac"));
    }

    #[test]
    fn build_library_map_skips_tracks_with_no_matching_root() {
        let track_list = vec![make_track("/other/track.flac", 1_000, "A", "B", "C")];
        let roots = vec!["/music".to_string()];
        let map = build_library_map(&track_list, &roots);
        assert!(map.is_empty());
    }

    #[test]
    fn parse_display_info_extracts_artist_album_title() {
        let (artist, album, title) = parse_display_info("The Beatles/Abbey Road/01 Come Together.flac");
        assert_eq!(artist, "The Beatles");
        assert_eq!(album, "Abbey Road");
        assert_eq!(title, "01 Come Together");
    }

    #[test]
    fn parse_display_info_handles_shallow_path() {
        let (artist, album, title) = parse_display_info("track.mp3");
        assert_eq!(artist, "");
        assert_eq!(album, "");
        assert_eq!(title, "track");
    }
}
