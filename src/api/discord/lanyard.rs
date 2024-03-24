use serde::{Deserialize, Serialize};

use crate::config::client;

use super::types::*;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct SpotifyData {
    pub track_id: String,
    pub timestamps: ActivityTimestamps,
    pub song: String,
    pub artist: String,
    pub album_art_url: String,
    pub album: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct LanyardData {
    pub active_on_discord_mobile: bool,
    pub active_on_discord_desktop: bool,
    pub active_on_discord_web: bool,
    //#[serde(default)]
    //kv: Vec<LanyardKV>,
    pub listening_to_spotify: bool,
    pub spotify: Option<SpotifyData>,
    pub discord_user: DiscordUser,
    pub discord_status: String,
    #[serde(default)]
    pub activities: Vec<DiscordActivity>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct LanyardError {
    code: String,
    message: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct LanyardResponse {
    pub success: bool,
    #[serde(default)]
    pub data: Option<LanyardData>,
    #[serde(default)]
    pub error: Option<LanyardError>,
}

pub struct MissingDataError;

impl LanyardResponse {
    pub async fn fetch(user_id: &String) -> Result<Self, reqwest::Error> {
        let res = client()
            .get(format!("https://api.lanyard.rest/v1/users/{}", user_id))
            .send()
            .await?
            .json::<LanyardResponse>()
            .await?;

        Ok(res)
    }
}
