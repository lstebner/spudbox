export type PlaybackState = "stopped" | "playing" | "paused";

export type PlaybackSnapshot = {
  state: PlaybackState;
  track_id: number | null;
  position_ms: number;
  duration_ms: number;
  volume: number;
  title: string | null;
  artist: string | null;
  album: string | null;
  album_id: number | null;
  art_path: string | null;
};

export type ArtistRow = {
  id: number;
  name: string;
  album_count: number;
};

export type AlbumRow = {
  id: number;
  title: string;
  album_artist: string;
  album_artist_id: number;
  year: number | null;
  art_path: string | null;
  rating: number | null;
};

export type TrackRow = {
  id: number;
  title: string;
  artist: string;
  album: string;
  album_id: number | null;
  duration_ms: number;
  sample_rate: number | null;
  bit_depth: number | null;
  channels: number | null;
  codec: string | null;
  disc_no: number | null;
  track_no: number | null;
};

export type ScanStats = {
  scanned: number;
  added: number;
  updated: number;
  unchanged: number;
  removed: number;
  errors: number;
};

export type ArtStats = {
  embedded: number;
  folder: number;
  none: number;
  errors: number;
};

export type ScanResult = {
  library: ScanStats;
  art: ArtStats;
};

export type SyncStats = {
  ratings_pushed: number;
  ratings_pulled: number;
  plays_pushed: number;
  plays_merged: number;
};

export type SyncStatus = {
  configured: boolean;
  machine_id: string;
};
