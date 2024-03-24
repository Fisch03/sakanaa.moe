use anyhow::Result;
use reqwest::header;
use serde::Deserialize;

use crate::config::config;
use crate::db::{
    db,
    music::{self, MusicDBExt, UnprocessedAlbum, UnprocessedTrack},
};

use super::types::*;

#[derive(Debug, Deserialize)]
pub struct LastFMConfig {
    user: String,
    api_key: String,
}

pub async fn get_current_track() -> Result<Option<music::Track>> {
    // i love the last.fm api /s
    #[derive(Debug, Deserialize)]
    struct Attr {
        #[serde(default)]
        nowplaying: Option<String>,
    }

    #[derive(Debug, Deserialize)]
    struct RecentTrack {
        #[serde(flatten)]
        track: LastFMTrack,
        album: LastFMRecentAlbum,
        #[serde(rename = "@attr", default)]
        attr: Option<Attr>,
    }

    #[derive(Debug, Deserialize)]
    struct RecentTracksResponseInner {
        #[serde(default)]
        track: Vec<RecentTrack>,
    }

    #[derive(Debug, Deserialize)]
    struct RecentTracksResponse {
        recenttracks: RecentTracksResponseInner,
    }

    let user = &config().api.lastfm.user;

    let res: RecentTracksResponse = do_request(
        "user.getRecentTracks",
        &[
            format!("user={}", user),
            "limit=1".to_string(),
            "extended=1".to_string(),
        ],
    )
    .await?;

    let current = res.recenttracks.track.iter().find_map(|t| {
        let nowplaying = if let Some(attr) = &t.attr {
            attr.nowplaying.as_ref()
        } else {
            None
        };

        if nowplaying.is_some() && nowplaying.unwrap() == "true" {
            Some((t.track.clone(), t.album.clone()))
        } else {
            None
        }
    });

    match current {
        Some((track, album)) => {
            let mut track = UnprocessedTrack::from(track);
            track.album = Some(UnprocessedAlbum {
                name: album.name,
                mbid: album.mbid,
                artist: None,
            });
            db().await.upsert_track(track).await.map(|t| Some(t))
        }
        None => return Ok(None),
    }
}

pub async fn top_tracks(user: &str, period: LastFMPeriod, limit: u32) -> Result<Vec<LastFMTrack>> {
    #[derive(Debug, Deserialize)]
    struct TopTracksResponse {
        toptracks: Vec<LastFMTrack>,
    }

    let res: TopTracksResponse = do_request(
        "user.getTopTracks",
        &[
            format!("user={}", user),
            format!("period={}", period.to_string()),
            format!("limit={}", limit),
        ],
    )
    .await?;

    Ok(res.toptracks)
}

pub async fn top_artists(
    user: &str,
    period: LastFMPeriod,
    limit: u32,
) -> Result<Vec<LastFMArtist>> {
    #[derive(Debug, Deserialize)]
    struct TopArtistsResponse {
        topartists: Vec<LastFMArtist>,
    }

    let res: TopArtistsResponse = do_request(
        "user.getTopArtists",
        &[
            format!("user={}", user),
            format!("period={}", period.to_string()),
            format!("limit={}", limit),
        ],
    )
    .await?;

    Ok(res.topartists)
}

pub async fn top_albums(user: &str, period: LastFMPeriod, limit: u32) -> Result<Vec<LastFMAlbum>> {
    #[derive(Debug, Deserialize)]
    struct TopAlbumsResponse {
        topalbums: Vec<LastFMAlbum>,
    }

    let res: TopAlbumsResponse = do_request(
        "user.getTopAlbums",
        &[
            format!("user={}", user),
            format!("period={}", period.to_string()),
            format!("limit={}", limit),
        ],
    )
    .await?;

    Ok(res.topalbums)
}

async fn do_request<T>(method: &str, params: &[String]) -> Result<T>
where
    T: for<'de> Deserialize<'de>,
{
    let client = config().server.client();
    let api_key = &config().api.lastfm.api_key;

    let res = client
        .post("https://ws.audioscrobbler.com/2.0/")
        .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
        .body(format!(
            "api_key={}&method={}&format=json&{}",
            api_key,
            method,
            params.join("&")
        ))
        .send()
        .await?
        .json::<T>()
        .await?;

    Ok(res)
}
