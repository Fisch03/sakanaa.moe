use crate::db::music;
use serde::{Deserialize, Deserializer, Serialize};

pub enum LastFMPeriod {
    Overall,
    SevenDay,
    OneMonth,
    ThreeMonth,
    SixMonth,
    OneYear,
}

impl LastFMPeriod {
    pub fn to_string(&self) -> String {
        match self {
            Self::Overall => "overall".to_string(),
            Self::SevenDay => "7day".to_string(),
            Self::OneMonth => "1month".to_string(),
            Self::ThreeMonth => "3month".to_string(),
            Self::SixMonth => "6month".to_string(),
            Self::OneYear => "12month".to_string(),
        }
    }
}

fn non_empty_str<'de, D: Deserializer<'de>>(d: D) -> Result<Option<String>, D::Error> {
    let o: Option<String> = Option::deserialize(d)?;
    Ok(o.filter(|s| !s.is_empty()))
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct LastFMArtist {
    pub name: String,
    #[serde(default)]
    pub playcount: Option<u32>,
    #[serde(deserialize_with = "non_empty_str")]
    pub mbid: Option<String>,
    pub url: String,
}
impl From<LastFMArtist> for music::UnprocessedArtist {
    fn from(artist: LastFMArtist) -> Self {
        Self {
            mbid: artist.mbid,
            name: artist.name,
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct LastFMTrack {
    pub name: String,
    #[serde(default)]
    pub playcount: Option<u32>,
    #[serde(deserialize_with = "non_empty_str")]
    pub mbid: Option<String>,
    pub url: String,
    pub artist: LastFMArtist,
}
impl From<LastFMTrack> for music::UnprocessedTrack {
    fn from(track: LastFMTrack) -> Self {
        Self {
            mbid: track.mbid,
            name: track.name,
            artist: Some(track.artist.into()),
            ..Default::default()
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct LastFMAlbum {
    pub name: String,
    #[serde(default)]
    pub playcount: Option<u32>,
    #[serde(deserialize_with = "non_empty_str")]
    pub mbid: Option<String>,
    pub url: String,
    pub artist: LastFMArtist,
}
impl From<LastFMAlbum> for music::UnprocessedAlbum {
    fn from(album: LastFMAlbum) -> Self {
        Self {
            mbid: album.mbid,
            name: album.name,
            artist: Some(album.artist.into()),
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct LastFMRecentAlbum {
    #[serde(rename = "#text")]
    pub name: String,
    #[serde(deserialize_with = "non_empty_str")]
    pub mbid: Option<String>,
}
