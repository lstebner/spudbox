-- Allow NULL rating in album_ratings to support deletion tombstones for cloud sync.
-- A row with NULL rating means "was explicitly unrated on this machine and that
-- deletion needs to propagate to the cloud"; absence of a row still means
-- "never rated here" and is equivalent for display purposes.
-- SQLite doesn't support ALTER COLUMN, so recreate the table.
CREATE TABLE album_ratings_new (
    album_id   INTEGER PRIMARY KEY REFERENCES albums(id) ON DELETE CASCADE,
    rating     REAL,
    updated_at INTEGER NOT NULL DEFAULT 0
);
INSERT INTO album_ratings_new SELECT album_id, rating, updated_at FROM album_ratings;
DROP TABLE album_ratings;
ALTER TABLE album_ratings_new RENAME TO album_ratings;
