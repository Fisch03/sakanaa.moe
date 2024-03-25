use std::path::PathBuf;

pub use super::audio_processing::bpm::BeatEvent;
use super::audio_processing::metadata::CoverArt;

#[derive(Debug, Clone)]
pub struct Artist {
    pub id: i64,
    pub mbid: Option<String>,
    pub name: String,
}
impl Artist {
    pub fn new(unprocessed: UnprocessedArtist, id: i64) -> Self {
        Self {
            id,
            mbid: unprocessed.mbid,
            name: unprocessed.name,
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct UnprocessedArtist {
    pub mbid: Option<String>,
    pub name: String,
}
impl UnprocessedArtist {
    pub fn merge(self, other: UnprocessedArtist) -> Self {
        Self {
            mbid: self.mbid.or(other.mbid),
            name: self.name,
        }
    }

    pub fn is_ready(&self) -> bool {
        self.mbid.is_some()
    }
}

#[derive(Debug, Clone)]
pub struct Album {
    pub id: i64,
    pub mbid: Option<String>,
    pub name: String,

    pub artist: Artist,
}
impl Album {
    pub fn new(unprocessed: UnprocessedAlbum, id: i64, artist: Artist) -> Self {
        Self {
            id,
            mbid: unprocessed.mbid,
            name: unprocessed.name,
            artist,
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct UnprocessedAlbum {
    pub mbid: Option<String>,
    pub name: String,
    pub artist: Option<UnprocessedArtist>,
}
impl UnprocessedAlbum {
    pub fn merge(self, other: UnprocessedAlbum) -> Self {
        Self {
            mbid: self.mbid.or(other.mbid),
            name: self.name,
            artist: self.artist.or(other.artist),
        }
    }

    pub fn is_ready(&self) -> bool {
        self.mbid.is_some() && self.artist.is_some()
    }
}

#[derive(Debug, Clone)]
pub struct Track {
    pub id: i64,
    pub mbid: Option<String>,
    pub name: String,
    pub beatevents: Vec<BeatEvent>,
    pub artist: Artist,
    pub album: Option<Album>,
    pub file: Option<PathBuf>,
    pub cover: Option<CoverArt>,
}
impl Track {
    pub fn new(
        unprocessed: UnprocessedTrack,
        id: i64,
        artist: Artist,
        album: Option<Album>,
    ) -> Self {
        let cover = if let Some(file) = &unprocessed.file {
            CoverArt::from_file(file).ok()
        } else {
            None
        };

        Self {
            id,
            mbid: unprocessed.mbid,
            name: unprocessed.name,
            beatevents: unprocessed.beatevents.unwrap_or_default(),
            cover,
            artist,
            album,
            file: unprocessed.file,
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct UnprocessedTrack {
    pub mbid: Option<String>,
    pub name: String,
    pub artist: Option<UnprocessedArtist>,
    pub album: Option<UnprocessedAlbum>,
    pub file: Option<PathBuf>,
    pub beatevents: Option<Vec<BeatEvent>>,
}
impl UnprocessedTrack {
    pub fn merge(self, other: UnprocessedTrack) -> Self {
        let artist = if self.artist.is_some() {
            if other.artist.is_some() {
                Some(self.artist.unwrap().merge(other.artist.unwrap()))
            } else {
                self.artist
            }
        } else {
            other.artist
        };

        let album = if self.album.is_some() {
            if other.album.is_some() {
                Some(self.album.unwrap().merge(other.album.unwrap()))
            } else {
                self.album
            }
        } else {
            other.album
        };

        Self {
            mbid: self.mbid.or(other.mbid),
            name: self.name,
            artist,
            album,
            file: self.file.or(other.file),
            beatevents: self.beatevents.or(other.beatevents),
        }
    }

    pub fn is_ready(&self) -> bool {
        self.mbid.is_some()
            && self.artist.as_ref().map_or(false, |a| a.is_ready())
            && self.album.as_ref().map_or(false, |a| a.is_ready())
            && self.file.is_some()
    }
}
