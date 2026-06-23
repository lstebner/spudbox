CREATE TABLE artists (
    id          INTEGER PRIMARY KEY,
    name        TEXT NOT NULL,
    sort_name   TEXT,
    UNIQUE(name)
);

CREATE TABLE albums (
    id              INTEGER PRIMARY KEY,
    title           TEXT NOT NULL,
    album_artist_id INTEGER REFERENCES artists(id),
    year            INTEGER,
    musicbrainz_id  TEXT,
    art_path        TEXT,
    art_source      TEXT,
    UNIQUE(title, album_artist_id, year)
);

CREATE TABLE genres (
    id   INTEGER PRIMARY KEY,
    name TEXT NOT NULL UNIQUE
);

CREATE TABLE tracks (
    id              INTEGER PRIMARY KEY,
    path            TEXT NOT NULL UNIQUE,
    folder_path     TEXT NOT NULL,
    title           TEXT NOT NULL,
    track_artist_id INTEGER REFERENCES artists(id),
    album_id        INTEGER REFERENCES albums(id),
    genre_id        INTEGER REFERENCES genres(id),
    track_no        INTEGER,
    disc_no         INTEGER,
    duration_ms     INTEGER NOT NULL,
    sample_rate     INTEGER,
    bit_depth       INTEGER,
    channels        INTEGER,
    codec           TEXT,
    bitrate_kbps    INTEGER,
    file_size       INTEGER,
    file_mtime      INTEGER NOT NULL,
    file_hash       TEXT,
    date_added       INTEGER NOT NULL,
    date_modified_db INTEGER NOT NULL
);

CREATE INDEX idx_tracks_album ON tracks(album_id);
CREATE INDEX idx_tracks_artist ON tracks(track_artist_id);
CREATE INDEX idx_tracks_genre ON tracks(genre_id);
CREATE INDEX idx_tracks_folder ON tracks(folder_path);

CREATE VIRTUAL TABLE tracks_fts USING fts5(
    title, artist_name, album_title,
    content='',
    tokenize='unicode61'
);

CREATE TABLE playlists (
    id          INTEGER PRIMARY KEY,
    name        TEXT NOT NULL,
    is_smart    INTEGER NOT NULL DEFAULT 0,
    created_at  INTEGER NOT NULL
);

CREATE TABLE playlist_tracks (
    playlist_id INTEGER NOT NULL REFERENCES playlists(id) ON DELETE CASCADE,
    track_id    INTEGER NOT NULL REFERENCES tracks(id) ON DELETE CASCADE,
    position    INTEGER NOT NULL,
    PRIMARY KEY (playlist_id, position)
);

CREATE TABLE play_history (
    id        INTEGER PRIMARY KEY,
    track_id  INTEGER NOT NULL REFERENCES tracks(id) ON DELETE CASCADE,
    played_at INTEGER NOT NULL,
    completed INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE track_stats (
    track_id    INTEGER PRIMARY KEY REFERENCES tracks(id) ON DELETE CASCADE,
    play_count  INTEGER NOT NULL DEFAULT 0,
    last_played INTEGER,
    rating      INTEGER,
    is_favorite INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE scan_roots (
    id         INTEGER PRIMARY KEY,
    path       TEXT NOT NULL UNIQUE,
    enabled    INTEGER NOT NULL DEFAULT 1,
    last_scan  INTEGER
);
