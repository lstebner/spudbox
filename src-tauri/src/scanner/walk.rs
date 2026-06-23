use std::path::{Path, PathBuf};

use walkdir::WalkDir;

const AUDIO_EXTENSIONS: &[&str] = &["flac", "mp3", "m4a", "aac", "ogg", "wav", "aiff", "aif"];

pub fn find_audio_files(root: &Path) -> Vec<PathBuf> {
    WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().is_file())
        .filter(|entry| {
            entry
                .path()
                .extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| AUDIO_EXTENSIONS.contains(&ext.to_lowercase().as_str()))
                .unwrap_or(false)
        })
        .map(|entry| entry.path().to_path_buf())
        .collect()
}
