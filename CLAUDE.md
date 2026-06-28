# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project

Spudbox: a custom Linux music player (Tauri v2 + Rust backend + Svelte 5 frontend). Library browsing with album art, hi-res FLAC/MP3/AAC/WAV playback, gapless transitions, MPRIS integration. See `README.md` for the feature list and `docs/PROJECT_PLAN.md` for the full architecture rationale, phase history, and what's still outstanding.

## Commands

```bash
# Frontend dev server + Tauri window, with hot reload for both Rust and Svelte
npm run tauri dev

# Type-check the frontend (svelte-check)
npm run check

# Frontend unit tests (vitest)
npm run test

# Rust unit tests (run from src-tauri/)
cd src-tauri && cargo test --lib

# Run a single Rust test
cd src-tauri && cargo test --lib db::queries::tracks::tests::upsert_inserts_a_new_track

# Run the ignored real-library integration test (hardcoded to a real path on
# the original dev machine — /home/luke/Dropbox/Music/CD Rips — so this will
# skip/fail on any other machine; treat as a manual regression check, not CI)
cd src-tauri && cargo test --lib real_library_scan -- --ignored --nocapture

# Build installable packages (.deb + portable AppImage)
npm run tauri build -- --bundles deb,appimage
```

Linux build prerequisites: `libwebkit2gtk-4.1-dev`, `libasound2-dev`, `libdbus-1-dev`, `pkg-config` (confirmed minimal set; other generically-recommended Tauri prereqs like `libxdo-dev` are not actually needed by this dependency tree).

CI (`.github/workflows/ci.yml`) runs both test suites on `pull_request` to `main` only — it does not run on direct pushes.

## Coding and workflow guidelines

- **Conventional commits**: all commit messages must follow the [Conventional Commits](https://www.conventionalcommits.org/) format (`feat:`, `fix:`, `refactor:`, `chore:`, etc.).
- **Never work on main**: always create a branch for changes; never commit directly to `main`.
- **Spell words out**: no abbreviations or shorthand in variable names, function names, or identifiers — write `index` not `idx`, `error` not `err`, `configuration` not `cfg`, etc.
- **Self-documenting names**: choose names that make the code's intent clear without needing a comment to explain it.
- **Comments only when necessary**: add a comment only when the code does something non-obvious that a future reader would need help staying oriented — not to describe what the code does (the code already does that).
- **No `.unwrap()` in non-test code**: use `?` for propagation or handle the error explicitly. A panic in the audio engine thread kills playback silently; `.expect("reason")` is acceptable only when the invariant is truly guaranteed and worth documenting.
- **Test names describe behavior**: name tests after what they assert, not what they call — `upsert_inserts_a_new_track`, not `test_upsert`. This is already the established pattern; keep it.
- **Always write tests**: every new feature and every bug fix should include tests — a test that reproduces the bug before the fix, then passes after.
- **No debug logging in commits**: `println!`, `dbg!`, `console.log`, and similar should not appear in committed code.
- **Keep PRs focused**: one concern per PR — a bug fix should not also refactor unrelated code; split them into separate PRs.
- **No magic numbers or strings**: use named constants for anything non-obvious (timeouts, buffer sizes, thresholds, limits).
- **Accessibility**: follow accessibility best practices throughout the UI — semantic HTML elements, ARIA attributes where needed, keyboard navigability, sufficient color contrast. The goal is for the entire app to be accessibility friendly.

## Architecture

### Process model

Three independent threads, communicating only through `AppState` (db pool handle) and channels — no shared mutable state between them:

- **Main/GTK thread**: the webview + Tauri's command dispatch. Tauri's default (non-`async`) `#[tauri::command]` functions run *inline* on whatever thread receives the IPC call, which on Linux is this thread. Any slow command freezes the whole window. `library_scan` (the one genuinely slow command — a fresh scan or first-run art backfill can take seconds) is `async` and wraps its body in `tauri::async_runtime::spawn_blocking` for exactly this reason; new slow commands must follow the same pattern.
- **Audio engine thread** (`audio/engine.rs`): owns the `rodio::OutputStream`/`Sink` and a SQLite connection (for play-stats and session-persistence writes — see below). Driven by a `PlayerCommand` channel (`audio::PlayerHandle`); publishes a `PlaybackSnapshot` via `arc_swap::ArcSwap` (lock-free reads) and a `playback-progress` Tauri event ~4Hz. MPRIS callbacks and Tauri commands both just send `PlayerCommand`s through the same handle — single source of truth, no duplicated state machines. Constructed via `audio::EngineBuilder`'s two-step pattern (`new()` → `handle()` → spawn `Mpris` with that handle → `spawn()` the engine thread) specifically to break the circular dependency between the engine (needs an `Mpris` to push state to) and `Mpris` (needs a `PlayerHandle` to forward incoming OS media-key events to).
- **Scanner work**: runs on Tauri's blocking-task pool (via the `spawn_blocking` above), not a dedicated long-lived thread — it's a one-shot job per scan, not continuous.

### Gapless playback (`audio/queue.rs` + `audio/engine.rs`)

Needs no custom sample-stitching source. rodio's own `Sink`/`SourcesQueueOutput` already hands off between sequentially-`append()`ed sources at the sample level with no inserted gap, and `OutputStreamHandle::play_raw` wraps everything in a `UniformSourceIterator` that auto-resamples/channel-converts at frame boundaries — so a queue mixing sample rates/bit depths needs no manual resampling code. The engine just keeps the sink *one track pre-appended ahead* of whatever's currently playing at all times: `audio::queue::Queue` tracks position in a `Vec<TrackInfo>`, and `engine.rs`'s tick loop detects track transitions by polling `Sink::len()` for a decrease (covers both natural end-of-track and an explicit `Next`/`skip_one()`), then advances the queue and appends the next track to restore the one-ahead invariant.

### Decoding (`audio/decode.rs`)

Bypasses rodio's `Decoder` convenience type entirely. rodio 0.20's internal `ReadSeekSource` hardcodes `byte_len() -> None`, which makes symphonia's format readers unable to compute seek targets (`SeekError::Unseekable`) — playback worked but scrubbing silently did nothing. `decode::FileSource` drives `symphonia`'s `FormatReader`/`Decoder` directly with a `MediaSource` impl that reports the real file length, fixing seeking. If touching playback/seeking, the fix lives here, not in how `Sink`/`OutputStream` are used.

### Library scanning (`scanner/`)

`full_scan` (`scanner/mod.rs`): `walkdir` traversal → parallel (`rayon`) `lofty` tag extraction → batched transactional upserts, keyed on file path. Skips re-parsing any file whose mtime+size match what's already in the DB — this is the only "incremental" mechanism; there is no filesystem watcher (deliberately — see Project status below). Album art (`scanner/art.rs`) is extracted as a *separate pass* after the main scan, not inline per-track, to avoid holding many full-size embedded images in memory at once for art that's identical across an album.

Tag extraction (`scanner/tags.rs`) always resolves artist/album/title to a non-null value (falling back to filename/parent-folder-name/"Unknown Artist") rather than leaving them NULL. This matters because the `albums` table's `UNIQUE(title, album_artist_id, year)` constraint is useless for dedup if `album_artist_id` is NULL — SQLite treats every NULL as distinct from every other NULL under a UNIQUE constraint. The scanner also keeps an in-process `HashMap` cache of resolved artist/album ids for the duration of one scan as the *real* dedup mechanism; the DB constraint is a secondary safety net, not the primary one.

### Database

SQLite via `rusqlite` with the `bundled` feature. That bundled build compiles with `SQLITE_DEFAULT_FOREIGN_KEYS=1` and FTS5 enabled — foreign keys are enforced even though nothing in this codebase explicitly runs `PRAGMA foreign_keys = ON` for migrations/tests (production connections set in via `db/pool.rs`'s `with_init`). Query modules live under `db/queries/`, one file per table-ish concern; each has its own `#[cfg(test)] mod tests` using `db::schema::test_connection()` (an in-memory, fully-migrated connection) — tests that insert `tracks`/`albums`/`track_stats` rows must create real referenced rows first or they'll hit a foreign-key constraint failure.

Migrations are plain `.sql` files in `src-tauri/migrations/`, embedded via `include_str!` and run through `rusqlite_migration` (`db/schema.rs`). Add new migrations as new numbered files; never edit a committed one.

### Frontend state (`src/lib/stores/`)

Two Svelte 5 runes-based stores (`.svelte.ts` files, not plain `.ts` — required for `$state`/`$derived` outside a component), each a singleton created once at module load:

- `player.svelte.ts`: mirrors the backend's `PlaybackSnapshot`, updated by listening to the `playback-progress` event. Commands are fire-and-forget wrappers around `invoke`.
- `library.svelte.ts`: holds both `albums` (scoped to whatever artist is currently selected, for the main grid) and `allAlbums` (always the complete unfiltered list, for the sidebar's search/grouping) as separate state — they're often the same data but diverge as soon as an artist filter is applied, and the sidebar needs the complete set regardless of what the main view is showing.

Tauri IPC argument names: Rust command parameters should stay `snake_case`; Tauri's generated bindings auto-convert to/from `camelCase` on the JS side. Don't manually camelCase Rust parameter names.

### Frontend layout (`src/routes/+page.svelte`)

The album grid and track list are *both* always mounted (toggled via a `display: none` wrapper, not `{#if}/{:else}`) so the album grid's scroll position survives navigating into a track list and back. The track list view *is* re-keyed on `library.selectedAlbumId` (via `{#key}`) so switching directly between two albums' track lists fully resets its virtualizer rather than reusing one sized for the previous album — this only works correctly because `library.svelte.ts`'s `selectAlbum` fetches the new tracks *before* assigning `selectedAlbumId`; assigning the id first would let the keyed remount read the still-stale `tracks` array.

Any "fill the parent" layout relationship in this codebase uses `position: absolute; inset: 0;`, not `height: 100%`, after a real bug where WebKitGTK silently failed to resolve `height: 100%` through a particular nested-div chain (Chromium-based browsers handled it fine — this is a Linux-webview-specific gotcha, not a general CSS one).

A Svelte 5 `$effect` that both reactively reads a store (via `$store` auto-subscription) *and* calls a method on that same store that causes it to re-emit will infinite-loop. Where this is needed (e.g. `@tanstack/svelte-virtual`'s `setOptions`), read the store value with `get(store)` from `svelte/store` (a one-time, non-reactive read) instead of `$store`.

### Icons

`@lucide/svelte` (the actively maintained package — not the deprecated `lucide-svelte`).

## Project status

Built in phases (0–5 of the original plan complete; Phase 6, device sync, not started — see `docs/PROJECT_PLAN.md` for the full breakdown and deferred-items list). One deliberate deviation from the original plan: Phase 4's filesystem watcher (`notify`-based live change detection) was dropped in favor of a plain scan on every launch — the existing incremental scan is already fast enough when nothing's changed that a live watcher wasn't worth the added complexity for this use case.

The Tauri app `identifier` is `com.lukestebner.musicplayer` even though the product is branded "Spudbox" — this is deliberate, not legacy cruft. Changing it would change the resolved app-data directory and orphan the already-scanned library/art cache on every machine it's installed on. The MPRIS `dbus_name` (`com.lukestebner.spudbox`) was changed during the rename since that has no data-directory implications.
