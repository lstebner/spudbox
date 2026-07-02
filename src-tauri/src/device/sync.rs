use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use tauri::{AppHandle, Emitter};
use walkdir::WalkDir;

use crate::db::queries::{scan_roots, tracks};
use crate::error::AppError;
use crate::state::DbPool;

use super::{SyncEntry, SyncMode, SyncPreview, SyncPreviewProgress, SyncProgress, SyncResult};

const PREVIEW_PROGRESS_EVENT: &str = "device-preview-progress";
const SYNC_PROGRESS_EVENT: &str = "device-sync-progress";

/// How many audio files to find on the device between progress event emissions.
const PREVIEW_PROGRESS_INTERVAL: usize = 25;

/// MTP sessions can go stale mid-transfer (device power-saving, USB hiccup).
/// Retrying after a short delay recovers from transient failures transparently.
const MAX_COPY_RETRIES: u32 = 3;
const COPY_RETRY_DELAY: std::time::Duration = std::time::Duration::from_secs(2);

const AUDIO_EXTENSIONS: &[&str] = &["flac", "mp3", "aac", "m4a", "wav", "ogg", "opus", "aiff"];

/// Computes what would change if the library were synced to `device_music_path`:
/// files to copy from the library and files to remove from the device.
/// Emits `device-preview-progress` events while walking the device filesystem.
/// Returns `AppError::Cancelled` immediately if `cancel` is set mid-walk.
pub fn preview_sync(
    device_music_path: &Path,
    db: &DbPool,
    app_handle: &AppHandle,
    cancel: &Arc<AtomicBool>,
) -> Result<SyncPreview, AppError> {
    let conn = db.get()?;
    let roots = scan_roots::list_enabled(&conn)?;
    let library_tracks = tracks::active_paths_with_sizes(&conn)?;

    let library_map = build_library_map(&library_tracks, &roots);
    let device_map = walk_device(device_music_path, app_handle, cancel)?;

    // Use the original source path in SyncEntry.relative_path so that
    // find_in_library_roots can locate the file on disk. The FAT-sanitized
    // key is only used here for the device comparison; perform_sync
    // re-sanitizes the destination when writing.
    let mut to_add: Vec<SyncEntry> = library_map
        .iter()
        .filter(|(fat_key, _)| !device_map.contains_key(*fat_key))
        .map(|(_, entry)| SyncEntry {
            relative_path: entry.source_relative_path.clone(),
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

/// Performs the sync using a preview already computed by the caller.
/// Copies additions to the device and, when `mode` is `All`, also removes
/// device files that are no longer in the library.
/// Emits `device-sync-progress` events throughout. Checks `cancel` between
/// every file operation; when set the sync stops cleanly without partial writes.
/// Returns actual counts of files copied/deleted and whether the sync was cancelled.
pub fn perform_sync(
    device_music_path: PathBuf,
    mode: SyncMode,
    preview: SyncPreview,
    db: DbPool,
    app_handle: AppHandle,
    cancel: Arc<AtomicBool>,
) -> Result<SyncResult, AppError> {
    let conn = db.get()?;
    let roots = scan_roots::list_enabled(&conn)?;
    drop(conn);

    let to_delete = if matches!(mode, SyncMode::All) { preview.to_delete } else { Vec::new() };
    let total = preview.to_add.len() + to_delete.len();
    let mut current = 0;
    let mut copied = 0usize;
    let mut deleted = 0usize;

    for entry in &preview.to_add {
        if cancel.load(Ordering::Relaxed) {
            return Ok(SyncResult { copied, deleted, cancelled: true });
        }

        let source = find_in_library_roots(&entry.relative_path, &roots);
        let Some(source) = source else {
            current += 1;
            continue;
        };

        // Sanitize for FAT32/exFAT: the library path may contain characters
        // (e.g. `?`) that are valid on Linux but forbidden on device filesystems.
        let destination = device_music_path.join(sanitize_path_for_fat(&entry.relative_path));
        copy_to_device(&source, &destination)?;

        copied += 1;
        current += 1;
        let _ = app_handle.emit(SYNC_PROGRESS_EVENT, SyncProgress {
            current,
            total,
            current_file: format!("{} — {}", entry.artist, entry.title),
            phase: "copying".to_string(),
            completed_relative_path: entry.relative_path.clone(),
        });
    }

    for entry in &to_delete {
        if cancel.load(Ordering::Relaxed) {
            return Ok(SyncResult { copied, deleted, cancelled: true });
        }

        let target = device_music_path.join(&entry.relative_path);
        if target.exists() {
            std::fs::remove_file(&target)?;
        }

        deleted += 1;
        current += 1;
        let _ = app_handle.emit(SYNC_PROGRESS_EVENT, SyncProgress {
            current,
            total,
            current_file: format!("{} — {}", entry.artist, entry.title),
            phase: "deleting".to_string(),
            completed_relative_path: entry.relative_path.clone(),
        });
    }

    Ok(SyncResult { copied, deleted, cancelled: false })
}

struct LibraryEntry {
    /// Original path on the Linux filesystem, used to locate the source file.
    source_relative_path: String,
    size_bytes: u64,
    artist: String,
    album: String,
    title: String,
}

/// Builds a map keyed by the FAT-sanitized relative path (for correct comparison
/// against device filenames) with the original Linux path stored in the value
/// (for source file lookup during sync).
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
                let source_relative_path = relative.to_string_lossy().into_owned();
                let fat_key = sanitize_path_for_fat(&source_relative_path);
                map.insert(fat_key, LibraryEntry {
                    source_relative_path,
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

/// Replaces characters forbidden in FAT32/exFAT filenames with `_` and strips
/// trailing dots and spaces from each path component. MTP devices (including
/// DAPs like the FiiO M21) use FAT-based filesystems that reject filenames
/// containing `\ : * ? " < > |` or control characters — gvfs surfaces this
/// rejection as EIO on `File::create`, before any data is transferred.
fn sanitize_path_for_fat(path: &str) -> String {
    path.split('/')
        .map(|component| {
            let sanitized: String = component
                .chars()
                .map(|c| match c {
                    '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
                    c if (c as u32) < 32 => '_',
                    c => c,
                })
                .collect();
            sanitized.trim_end_matches(['.', ' ']).to_string()
        })
        .collect::<Vec<_>>()
        .join("/")
}

/// Walks `device_music_path` and returns `relative_path → size_bytes` for
/// every audio file found. Returns an empty map if the path does not exist yet.
/// Emits `device-preview-progress` every `PREVIEW_PROGRESS_INTERVAL` files so
/// the frontend can show a running count while the MTP traversal is in progress.
/// Returns `AppError::Cancelled` immediately when `cancel` is set.
fn walk_device(
    device_music_path: &Path,
    app_handle: &AppHandle,
    cancel: &Arc<AtomicBool>,
) -> Result<HashMap<String, u64>, AppError> {
    if !device_music_path.exists() {
        return Ok(HashMap::new());
    }
    let mut map = HashMap::new();
    for entry in WalkDir::new(device_music_path).into_iter().flatten() {
        if cancel.load(Ordering::Relaxed) {
            return Err(AppError::Cancelled);
        }
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

/// Copies `source` to `destination`, retrying up to `MAX_COPY_RETRIES` times
/// on failure. Each retry removes any partial file and waits `COPY_RETRY_DELAY`
/// before trying again, giving the MTP session time to recover from USB hiccups
/// that cause transient EIO errors during long transfers.
fn copy_to_device(source: &Path, destination: &Path) -> Result<(), AppError> {
    let mut last_error: Option<std::io::Error> = None;
    for attempt in 0..=MAX_COPY_RETRIES {
        if attempt > 0 {
            let _ = std::fs::remove_file(destination);
            std::thread::sleep(COPY_RETRY_DELAY);
        }
        let result = (|| -> Result<(), std::io::Error> {
            if let Some(parent) = destination.parent() {
                std::fs::create_dir_all(parent)?;
            }
            // std::fs::copy uses copy_file_range on Linux, which gvfs-fuse returns
            // EOPNOTSUPP (os error 95) for. Read/write via io::copy works correctly.
            let mut source_file = std::fs::File::open(source)?;
            let mut destination_file = std::fs::File::create(destination)?;
            std::io::copy(&mut source_file, &mut destination_file)?;
            Ok(())
        })();
        match result {
            Ok(()) => return Ok(()),
            Err(error) => last_error = Some(error),
        }
    }
    Err(AppError::Io(last_error.unwrap()))
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
    fn sanitize_path_for_fat_replaces_forbidden_characters() {
        assert_eq!(
            sanitize_path_for_fat("Artist/Album/01 - What Is This_.flac"),
            "Artist/Album/01 - What Is This_.flac"
        );
        assert_eq!(
            sanitize_path_for_fat("Curl Up and Die/Unfortunately We're Not Robots/Why the Fuck Do You?.flac"),
            "Curl Up and Die/Unfortunately We're Not Robots/Why the Fuck Do You_.flac"
        );
        assert_eq!(
            sanitize_path_for_fat("Artist: Live/Track \"One\" <Two>.flac"),
            "Artist_ Live/Track _One_ _Two_.flac"
        );
        assert_eq!(
            sanitize_path_for_fat("Band/Album.../Track .flac"),
            "Band/Album/Track .flac"
        );
    }

    #[test]
    fn parse_display_info_handles_shallow_path() {
        let (artist, album, title) = parse_display_info("track.mp3");
        assert_eq!(artist, "");
        assert_eq!(album, "");
        assert_eq!(title, "track");
    }
}
