# Spudbox

A custom music player for Linux, built as a nicer alternative to Clementine: library browsing with album art, hi-res FLAC/MP3/AAC/WAV playback, gapless transitions, and MPRIS integration for system media controls. Built with Tauri v2, Rust, and Svelte.

<p align="center">
  <img src="docs/screenshots/album-grid.png" alt="Album grid view" width="48%">
  <img src="docs/screenshots/track-list.png" alt="Track list view" width="48%">
</p>

## Features

- Library scanning with incremental rescans (skips unchanged files), automatic on launch
- Browse by artist/album with a virtualized, art-forward grid and track list
- Hi-res playback (24-bit/96kHz+ FLAC and beyond) via a pure-Rust audio stack (`symphonia` + `rodio`/`cpal`) — no GStreamer dependency
- Gapless playback between queued tracks
- Album art extraction (embedded or folder cover images), cached as thumbnails
- 0–10 half-star album ratings, displayed in the grid and track list
- MPRIS integration (system media keys, GNOME/KDE media widgets)
- Play history and stats tracked per track
- Remembers volume and resumes the last queue/track (paused) on next launch
- **Cloud sync** — ratings and play counts sync across machines via a [Turso](https://turso.tech) database (free tier); configure once in Settings with a DB URL and auth token
- **8-band graphic equalizer** — launched from the toolbar, with a live SVG frequency-response curve. Outermost bands (63 Hz, 8 kHz) are shelf filters that affect the full low/high extremes; middle six bands are peaking filters. Includes named presets (Flat, Bass Boost, Treble Boost, Rock, Mid Boost, Classical, Vocal), a Custom slot that remembers your curve when switching presets, and a bypass toggle. EQ settings persist across restarts.
- **Device sync** — copy your library to a connected DAP or USB drive via the toolbar icon. Supports any device that mounts as a filesystem (USB mass storage, MTP via gvfs). The sync panel shows a preview of what will change before anything is written; syncing can be cancelled at any point and safely resumed by re-running the preview.

  > **Transfer speed note:** USB mass storage devices transfer at full USB speed and are strongly recommended when your player supports it. MTP (used by most modern DAPs and Android phones) is inherently serial — the protocol requires a device acknowledgement after every file before the next one can begin — so syncing a large library over MTP will be slow regardless of USB generation. If your device offers a choice of connection mode, choose USB storage (or "USB disk") over MTP.

## Prerequisites (Linux)

- Rust (via [rustup](https://rustup.rs))
- Node.js + npm
- System packages (Ubuntu/Debian):
  ```
  sudo apt install libwebkit2gtk-4.1-dev libasound2-dev libdbus-1-dev pkg-config
  ```
  Note: the generic Tauri prereq list includes several packages (`libxdo-dev`, `libayatana-appindicator3-dev`, etc.) that this project does not actually need.

## Development

```
npm install
npm run tauri dev
```

## Building an installable package

```
npm run tauri build -- --bundles deb,appimage
```

Produces a `.deb` (Debian/Ubuntu) and a portable `.AppImage` (any modern Linux distro) under `src-tauri/target/release/bundle/`.

## Testing

```
cd src-tauri && cargo test    # Rust: db queries (in-memory SQLite), the queue model, scanner helpers
npm run test                  # frontend: pure-logic helpers (vitest)
```

## Recommended IDE Setup

[Neovim](https://neovim.io/) with [nvim-lspconfig](https://github.com/neovim/nvim-lspconfig), running `rust_analyzer` for the Tauri backend and `svelte` (svelte-language-server) for the frontend. [mason.nvim](https://github.com/williamboman/mason.nvim) is the easiest way to install both servers.

## License

Copyright (C) 2026 Luke Stebner.

Licensed under the [GNU Affero General Public License v3.0](LICENSE) (AGPL-3.0-or-later). You're free to use, modify, and redistribute Spudbox, including commercially — but any distributed or network-hosted derivative must also be released under AGPLv3, source included.
