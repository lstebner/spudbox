import { invoke } from "@tauri-apps/api/core";
import type { AlbumRow, ArtistRow, PlaybackSnapshot, ScanResult, TrackRow } from "$lib/types";

export const commands = {
  ping: () => invoke<string>("ping"),

  libraryAddRoot: (path: string) => invoke<void>("library_add_root", { path }),
  libraryScan: () => invoke<ScanResult>("library_scan"),
  libraryGetArtists: () => invoke<ArtistRow[]>("library_get_artists"),
  libraryGetAlbums: (artistId: number | null) =>
    invoke<AlbumRow[]>("library_get_albums", { artistId }),
  libraryGetTracks: () => invoke<TrackRow[]>("library_get_tracks"),
  libraryGetTracksByAlbum: (albumId: number) =>
    invoke<TrackRow[]>("library_get_tracks_by_album", { albumId }),

  playbackPlayTrack: (trackId: number) => invoke<void>("playback_play_track", { trackId }),
  playbackPlay: () => invoke<void>("playback_play"),
  playbackPause: () => invoke<void>("playback_pause"),
  playbackSeek: (positionMs: number) => invoke<void>("playback_seek", { positionMs }),
  playbackSetVolume: (volume: number) => invoke<void>("playback_set_volume", { volume }),
  playbackGetSnapshot: () => invoke<PlaybackSnapshot>("playback_get_snapshot"),
};
