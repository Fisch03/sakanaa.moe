use crate::config::CLIENT;

use super::types::*;

use reqwest::{header, Error};

use lazy_static::lazy_static;
use serde::Deserialize;

lazy_static! {
    pub static ref LASTFM_API_KEY: String = crate::config::CONFIG
        .get::<String>("lastfm.api_key")
        .expect("Missing lastfm.api_key in config");
}

pub async fn get_current_track(user: &str) -> Result<Option<LastFMTrack>, Error> {
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
            Some(t.track.clone())
        } else {
            None
        }
    });

    Ok(current)
}

pub async fn top_tracks(
    user: &str,
    period: LastFMPeriod,
    limit: u32,
) -> Result<Vec<LastFMTrack>, Error> {
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
) -> Result<Vec<LastFMArtist>, Error> {
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

pub async fn top_albums(
    user: &str,
    period: LastFMPeriod,
    limit: u32,
) -> Result<Vec<LastFMAlbum>, Error> {
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

pub async fn do_request<T>(method: &str, params: &[String]) -> Result<T, Error>
where
    T: for<'de> Deserialize<'de>,
    T: std::fmt::Debug,
{
    /*
    {
        let body = format!(
            "api_key={}&method={}&format=json&{}",
            &*LASTFM_API_KEY,
            method,
            params.join("&")
        );

        dbg!(&body);

        let res = CLIENT
            .post("https://ws.audioscrobbler.com/2.0/")
            .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
            .body(body)
            .send()
            .await?;

        dbg!(&res.json::<T>().await);
    }
    */

    let res = CLIENT
        .post("https://ws.audioscrobbler.com/2.0/")
        .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
        .body(format!(
            "api_key={}&method={}&format=json&{}",
            &*LASTFM_API_KEY,
            method,
            params.join("&")
        ))
        .send()
        .await?
        .json::<T>()
        .await?;

    Ok(res)
}
