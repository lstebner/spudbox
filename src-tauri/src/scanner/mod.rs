pub mod art;
pub mod tags;
pub mod walk;

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use rayon::prelude::*;
use rusqlite::Connection;
use serde::Serialize;

use crate::db::queries::{albums, artists, genres, tracks};
use crate::error::AppError;
use crate::state::DbPool;

#[derive(Debug, Default, Clone, Serialize)]
pub struct ScanStats {
    pub scanned: usize,
    pub added: usize,
    pub updated: usize,
    pub unchanged: usize,
    pub removed: usize,
    pub errors: usize,
}

fn now_unix() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
}

fn mtime_of(metadata: &std::fs::Metadata) -> i64 {
    metadata
        .modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn resolve_artist(conn: &Connection, cache: &mut HashMap<String, i64>, name: &str) -> Result<i64, AppError> {
    if let Some(id) = cache.get(name) {
        return Ok(*id);
    }
    let id = artists::upsert(conn, name)?;
    cache.insert(name.to_string(), id);
    Ok(id)
}

fn resolve_genre(conn: &Connection, cache: &mut HashMap<String, i64>, name: &str) -> Result<i64, AppError> {
    if let Some(id) = cache.get(name) {
        return Ok(*id);
    }
    let id = genres::upsert(conn, name)?;
    cache.insert(name.to_string(), id);
    Ok(id)
}

fn resolve_album(
    conn: &Connection,
    cache: &mut HashMap<(String, i64, Option<i64>), i64>,
    title: &str,
    album_artist_id: i64,
    year: Option<i64>,
) -> Result<i64, AppError> {
    let key = (title.to_lowercase(), album_artist_id, year);
    if let Some(id) = cache.get(&key) {
        return Ok(*id);
    }
    let id = albums::upsert(conn, title, album_artist_id, year)?;
    cache.insert(key, id);
    Ok(id)
}

/// Scans every enabled root: walks the filesystem, skips files whose
/// mtime/size match what's already in the DB (the incremental-rescan
/// fast path), parallel-extracts tags for the rest, and upserts in
/// batched transactions. Files previously known but no longer found on
/// disk are deleted. `on_progress(done, total)` is called periodically
/// (not on every file) so callers can throttle UI/event updates.
pub fn full_scan(
    pool: &DbPool,
    roots: &[PathBuf],
    mut on_progress: impl FnMut(usize, usize),
) -> Result<ScanStats, AppError> {
    let mut conn = pool.get()?;
    let known_fingerprints = tracks::fingerprints(&conn)?;

    let mut found_files: Vec<PathBuf> = Vec::new();
    for root in roots {
        found_files.extend(walk::find_audio_files(root));
    }

    let mut found_paths: HashSet<String> = HashSet::with_capacity(found_files.len());
    let mut to_process: Vec<PathBuf> = Vec::new();
    let mut unchanged = 0usize;

    for path in &found_files {
        let path_str = path.to_string_lossy().to_string();
        found_paths.insert(path_str.clone());

        let metadata = match std::fs::metadata(path) {
            Ok(m) => m,
            Err(_) => continue,
        };
        let mtime = mtime_of(&metadata);
        let size = metadata.len() as i64;

        match known_fingerprints.get(&path_str) {
            Some((known_mtime, known_size)) if *known_mtime == mtime && *known_size == size => {
                unchanged += 1;
            }
            _ => to_process.push(path.clone()),
        }
    }

    let extracted: Vec<(PathBuf, std::fs::Metadata, Option<tags::TrackMeta>)> = to_process
        .par_iter()
        .filter_map(|path| {
            let metadata = std::fs::metadata(path).ok()?;
            let meta = tags::extract(path);
            Some((path.clone(), metadata, meta))
        })
        .collect();

    let mut stats = ScanStats::default();
    let mut artist_cache: HashMap<String, i64> = HashMap::new();
    let mut genre_cache: HashMap<String, i64> = HashMap::new();
    let mut album_cache: HashMap<(String, i64, Option<i64>), i64> = HashMap::new();

    const BATCH_SIZE: usize = 500;
    let mut processed_in_batch = 0usize;
    let mut tx = conn.transaction()?;
    let total = extracted.len();

    for (i, (path, metadata, meta_opt)) in extracted.iter().enumerate() {
        let meta = match meta_opt {
            Some(m) => m,
            None => {
                stats.errors += 1;
                continue;
            }
        };

        let artist_id = resolve_artist(&tx, &mut artist_cache, &meta.artist)?;
        let album_artist_id = resolve_artist(&tx, &mut artist_cache, &meta.album_artist)?;
        let genre_id = match &meta.genre {
            Some(g) => Some(resolve_genre(&tx, &mut genre_cache, g)?),
            None => None,
        };
        let album_id = resolve_album(
            &tx,
            &mut album_cache,
            &meta.album,
            album_artist_id,
            meta.year.map(|y| y as i64),
        )?;

        let path_str = path.to_string_lossy().to_string();
        let folder_str = path
            .parent()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();
        let was_known = known_fingerprints.contains_key(&path_str);

        tracks::upsert(
            &tx,
            &tracks::NewTrack {
                path: &path_str,
                folder_path: &folder_str,
                title: &meta.title,
                track_artist_id: artist_id,
                album_id,
                genre_id,
                track_no: meta.track_no,
                disc_no: meta.disc_no,
                duration_ms: meta.duration_ms,
                sample_rate: meta.sample_rate,
                bit_depth: meta.bit_depth,
                channels: meta.channels,
                codec: &meta.codec,
                bitrate_kbps: meta.bitrate_kbps,
                file_size: metadata.len() as i64,
                file_mtime: mtime_of(metadata),
                now: now_unix(),
            },
        )?;

        if was_known {
            stats.updated += 1;
        } else {
            stats.added += 1;
        }

        processed_in_batch += 1;
        if processed_in_batch >= BATCH_SIZE {
            tx.commit()?;
            tx = conn.transaction()?;
            processed_in_batch = 0;
        }

        if (i + 1) % 50 == 0 || i + 1 == total {
            on_progress(i + 1, total);
        }
    }
    tx.commit()?;

    stats.unchanged = unchanged;
    stats.scanned = found_files.len();

    let missing: Vec<String> = known_fingerprints
        .keys()
        .filter(|p| !found_paths.contains(*p))
        .cloned()
        .collect();
    if !missing.is_empty() {
        stats.removed = tracks::delete_by_paths(&conn, &missing)?;
    }

    Ok(stats)
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::path::PathBuf;
    use std::time::Instant;

    use crate::db::{pool, schema};
    use crate::db::queries::artists;

    use super::{full_scan, resolve_album};

    #[test]
    fn same_named_albums_with_different_years_get_distinct_ids() {
        let conn = schema::test_connection();
        let artist_id = artists::upsert(&conn, "Cursive").unwrap();
        let mut cache = HashMap::new();

        let original_id = resolve_album(&conn, &mut cache, "Domestica", artist_id, Some(2000)).unwrap();
        let remaster_id = resolve_album(&conn, &mut cache, "Domestica", artist_id, Some(2022)).unwrap();

        assert_ne!(original_id, remaster_id, "same-named albums with different years must be separate entries");
    }

    #[test]
    fn same_named_album_with_same_year_reuses_the_same_id() {
        let conn = schema::test_connection();
        let artist_id = artists::upsert(&conn, "Cursive").unwrap();
        let mut cache = HashMap::new();

        let first_id = resolve_album(&conn, &mut cache, "Domestica", artist_id, Some(2000)).unwrap();
        let second_id = resolve_album(&conn, &mut cache, "Domestica", artist_id, Some(2000)).unwrap();

        assert_eq!(first_id, second_id, "the same album encountered twice in one scan should reuse the same id");
    }

    /// Exercises the scanner against the user's real library rather than a
    /// synthetic fixture, per the plan's verification step. Ignored by
    /// default since it's machine-specific and takes real time; run with
    /// `cargo test real_library_scan -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn real_library_scan_is_correct_and_incremental_rescan_is_fast() {
        let lib_path = PathBuf::from("/home/luke/Dropbox/Music/CD Rips");
        assert!(lib_path.exists(), "test library path not found");

        let tmp_dir = std::env::temp_dir().join(format!("spudbox_test_{}", std::process::id()));
        std::fs::create_dir_all(&tmp_dir).unwrap();
        let db_path = tmp_dir.join("test_library.sqlite3");

        let pool = pool::create_pool(&db_path).unwrap();
        {
            let mut conn = pool.get().unwrap();
            schema::run_migrations(&mut conn).unwrap();
        }

        let roots = vec![lib_path];

        let start = Instant::now();
        let stats1 = full_scan(&pool, &roots, |_, _| {}).unwrap();
        let first_elapsed = start.elapsed();
        println!("First scan:  {stats1:?} in {first_elapsed:?}");

        assert!(
            stats1.scanned > 6000,
            "expected 6000+ audio files, found {}",
            stats1.scanned
        );
        assert_eq!(stats1.unchanged, 0, "first scan should have no unchanged files");
        assert!(
            stats1.errors < stats1.scanned / 100,
            "more than 1% of files failed to parse: {} of {}",
            stats1.errors,
            stats1.scanned
        );

        let start2 = Instant::now();
        let stats2 = full_scan(&pool, &roots, |_, _| {}).unwrap();
        let second_elapsed = start2.elapsed();
        println!("Second scan: {stats2:?} in {second_elapsed:?}");

        assert_eq!(stats2.added, 0, "rescan with no changes should add nothing");
        assert_eq!(stats2.updated, 0, "rescan with no changes should update nothing");
        assert_eq!(
            stats2.unchanged,
            stats1.scanned,
            "every file should be unchanged on immediate rescan"
        );
        assert!(
            second_elapsed < first_elapsed / 2,
            "incremental rescan should be substantially faster: first={first_elapsed:?} second={second_elapsed:?}"
        );

        let _ = std::fs::remove_dir_all(&tmp_dir);
    }
}
