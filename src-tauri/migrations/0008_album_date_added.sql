ALTER TABLE albums ADD COLUMN date_added INTEGER;

UPDATE albums SET date_added = (
    SELECT MIN(t.date_added) FROM tracks t WHERE t.album_id = albums.id
);
