-- Track this machine's own plays separately from the merged total.
-- own_play_count: plays originated on THIS machine only.
-- synced_play_count: own_play_count as of the last successful cloud push.
-- play_count continues to hold the merged total (own + remote).
ALTER TABLE track_stats ADD COLUMN own_play_count   INTEGER NOT NULL DEFAULT 0;
ALTER TABLE track_stats ADD COLUMN synced_play_count INTEGER NOT NULL DEFAULT 0;
UPDATE track_stats SET own_play_count = play_count;

-- Timestamp for last-write-wins conflict resolution on album ratings.
-- DEFAULT 0: existing ratings have no timestamp and will be overwritten by
-- any cloud row that has a real timestamp.
ALTER TABLE album_ratings ADD COLUMN updated_at INTEGER NOT NULL DEFAULT 0;
