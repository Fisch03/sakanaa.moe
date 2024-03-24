use anyhow::{bail, Context, Result};
use axum::async_trait;

use super::super::types::{UnprocessedAlbum, UnprocessedArtist, UnprocessedTrack};
use super::AudioDataSource;
use crate::api::musicbrainz::{self, MBError};

pub struct MusicBrainzLookupSource {}
impl MusicBrainzLookupSource {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl AudioDataSource for MusicBrainzLookupSource {
    async fn lookup_track(
        &self,
        mut track: UnprocessedTrack,
        replace: bool,
    ) -> Result<UnprocessedTrack> {
        let mbid = track.mbid.as_ref().context("Track has no MBID")?;

        let mb_track = musicbrainz::track_by_id(mbid).await;
        match mb_track {
            Ok(mb_track) => {
                let artist;
                if let Some(existing_artist) = track.artist.as_ref() {
                    let name_lower = existing_artist.name.to_lowercase();
                    let mut fitting_artist = mb_track
                        .artist_credit
                        .iter()
                        .find(|ac| name_lower.starts_with(&ac.artist.name.to_lowercase()));
                    if fitting_artist.is_none() {
                        fitting_artist = mb_track.artist_credit.get(0);
                    }
                    artist = fitting_artist.map(|fitting_artist| UnprocessedArtist {
                        name: fitting_artist.artist.name.clone(),
                        mbid: Some(fitting_artist.artist.id.clone()),
                    })
                } else {
                    artist = None;
                }

                let album;
                if let Some(existing_album) = track.album.as_ref() {
                    let name_lower = existing_album.name.to_lowercase();
                    let mut fitting_release = mb_track
                        .releases
                        .iter()
                        .find(|r| name_lower.starts_with(&r.title.to_lowercase()));
                    if fitting_release.is_none() {
                        fitting_release = mb_track.releases.get(0);
                    }
                    album = fitting_release.map(|fitting_release| UnprocessedAlbum {
                        name: fitting_release.title.clone(),
                        mbid: Some(fitting_release.id.clone()),

                        // unfortunately, musicbrainz doesn't return the album artist
                        // in the track lookup. i might add a separate lookup for this
                        // if it really becomes an issue, but i'd rather keep api
                        // requests to a minimum for now due to the 1s ratelimit
                        artist: None,
                    })
                } else {
                    album = None;
                }

                let mb_track = UnprocessedTrack {
                    mbid: Some(mb_track.id.clone()),
                    name: mb_track.title,
                    artist,
                    album,
                    beatevents: None,
                    file: None,
                };

                if replace {
                    track = mb_track.merge(track);
                } else {
                    track = track.merge(mb_track);
                }
            }
            Err(MBError::NotFound) => {
                // the mbid seems to be invalid, remove it
                // (this happens a lot with tracks from lastfm...)
                track.mbid = None;
            }
            Err(e) => {
                bail!("Failed to lookup track: {}", e);
            }
        }

        Ok(track)
    }
}

pub struct MusicBrainzSearchSource {}
impl MusicBrainzSearchSource {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl AudioDataSource for MusicBrainzSearchSource {
    async fn lookup_track(
        &self,
        track: UnprocessedTrack,
        replace: bool,
    ) -> Result<UnprocessedTrack> {
        Err(anyhow::anyhow!("Not implemented"))
    }
}
