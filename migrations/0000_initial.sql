CREATE TABLE artists(
	artist_id INTEGER NOT NULL PRIMARY KEY,
	mbid BLOB,
	name TEXT NOT NULL
);
CREATE INDEX artists_mbid ON artists(mbid);
CREATE INDEX artists_names ON artists(name);

CREATE TABLE covers(
    cover_id INTEGER NOT NULL PRIMARY KEY,
    source_path TEXT NOT NULL
);

CREATE TABLE albums(
	album_id INTEGER NOT NULL PRIMARY KEY,
    path TEXT NOT NULL,
    cover_id INTEGER,
	mbid BLOB,
	title TEXT NOT NULL,
	artist_id INTEGER,
	FOREIGN KEY (artist_id) REFERENCES artists(artist_id),
    FOREIGN KEY (cover_id) REFERENCES covers(cover_id)
);
CREATE INDEX albums_path ON albums(path);
CREATE INDEX albums_mbid ON albums(mbid);
CREATE INDEX albums_names ON albums(title, artist_id);

CREATE TABLE tracks(
    track_id INTEGER NOT NULL PRIMARY KEY,
    path TEXT NOT NULL,
    cover_id INTEGER,
    mbid BLOB,
    title TEXT NOT NULL,
    album_id INTEGER,
    artist_id INTEGER,
    FOREIGN KEY (album_id) REFERENCES albums(album_id),
    FOREIGN KEY (artist_id) REFERENCES artists(artist_id),
    FOREIGN KEY (cover_id) REFERENCES covers(cover_id)
);
CREATE UNIQUE INDEX tracks_path ON tracks(path);
CREATE INDEX tracks_mbid ON tracks(mbid);
CREATE INDEX tracks_names ON tracks(title, artist_id, album_id);
