use std::path::Path;

use lofty::file::FileType;
use lofty::prelude::*;
use lofty::tag::ItemKey;

pub struct TrackMeta {
    pub title: String,
    pub artist: String,
    pub album: String,
    pub album_artist: String,
    pub genre: Option<String>,
    pub track_no: Option<u32>,
    pub disc_no: Option<u32>,
    pub year: Option<u32>,
    pub duration_ms: i64,
    pub sample_rate: Option<u32>,
    pub bit_depth: Option<u8>,
    pub channels: Option<u8>,
    pub codec: String,
    pub bitrate_kbps: Option<u32>,
}

fn codec_label(file_type: FileType) -> String {
    match file_type {
        FileType::Mpeg => "mp3".to_string(),
        FileType::Aac => "aac".to_string(),
        FileType::Aiff => "aiff".to_string(),
        FileType::Ape => "ape".to_string(),
        FileType::Flac => "flac".to_string(),
        FileType::Mp4 => "mp4".to_string(),
        FileType::Mpc => "mpc".to_string(),
        FileType::Opus => "opus".to_string(),
        FileType::Vorbis => "vorbis".to_string(),
        FileType::Speex => "speex".to_string(),
        FileType::Wav => "wav".to_string(),
        FileType::WavPack => "wavpack".to_string(),
        FileType::Custom(s) => s.to_lowercase(),
        _ => "unknown".to_string(),
    }
}

/// Extracts tag + audio-property metadata for a single file. Falls back to
/// filesystem-derived values (filename, parent folder name) when tags are
/// missing, and defaults artist/album_artist to a non-null placeholder so
/// album identity never relies on SQL NULL equality semantics.
pub fn extract(path: &Path) -> Option<TrackMeta> {
    let tagged_file = lofty::read_from_path(path).ok()?;
    let properties = tagged_file.properties();
    let tag = tagged_file.primary_tag().or_else(|| tagged_file.first_tag());

    let fallback_title = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Unknown Title")
        .to_string();
    let fallback_album = path
        .parent()
        .and_then(|p| p.file_name())
        .and_then(|s| s.to_str())
        .unwrap_or("Unknown Album")
        .to_string();

    let artist = tag
        .and_then(|t| t.artist())
        .map(|c| c.into_owned())
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| "Unknown Artist".to_string());

    let album_artist = tag
        .and_then(|t| t.get_string(&ItemKey::AlbumArtist))
        .map(|s| s.to_string())
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| artist.clone());

    Some(TrackMeta {
        title: tag
            .and_then(|t| t.title())
            .map(|c| c.into_owned())
            .filter(|s| !s.trim().is_empty())
            .unwrap_or(fallback_title),
        artist,
        album: tag
            .and_then(|t| t.album())
            .map(|c| c.into_owned())
            .filter(|s| !s.trim().is_empty())
            .unwrap_or(fallback_album),
        album_artist,
        genre: tag
            .and_then(|t| t.genre())
            .map(|c| c.into_owned())
            .filter(|s| !s.trim().is_empty()),
        track_no: tag.and_then(|t| t.track()),
        disc_no: tag.and_then(|t| t.disk()),
        year: tag.and_then(|t| t.year()),
        duration_ms: properties.duration().as_millis() as i64,
        sample_rate: properties.sample_rate(),
        bit_depth: properties.bit_depth(),
        channels: properties.channels(),
        codec: codec_label(tagged_file.file_type()),
        bitrate_kbps: properties.audio_bitrate(),
    })
}
