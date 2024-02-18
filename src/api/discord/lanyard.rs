use serde::{Deserialize, Serialize};

use super::types::*;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct SpotifyData {
    track_id: String,
    timestamps: ActivityTimestamps,
    song: String,
    artist: String,
    album_art_url: String,
    album: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct LanyardData {
    active_on_discord_mobile: bool,
    active_on_discord_desktop: bool,
    active_on_discord_web: bool,
    //#[serde(default)]
    //kv: Vec<LanyardKV>,
    listening_to_spotify: bool,
    spotify: Option<SpotifyData>,
    discord_user: DiscordUser,
    discord_status: String,
    #[serde(default)]
    activities: Vec<DiscordActivity>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct LanyardError {
    code: String,
    message: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct LanyardResponse {
    success: bool,
    #[serde(default)]
    data: Option<LanyardData>,
    #[serde(default)]
    error: Option<LanyardError>,
}

pub struct MissingDataError;

impl LanyardResponse {
    pub async fn fetch(user_id: &String) -> Result<Self, reqwest::Error> {
        let client = reqwest::Client::new();

        let res = client
            .get(format!("https://api.lanyard.rest/v1/users/{}", user_id))
            .send()
            .await?
            .json::<LanyardResponse>()
            .await?;

        Ok(res)
    }

    pub fn to_status(
        self,
        music_activity_filters: &Vec<MusicActivityFilter>,
    ) -> Result<DiscordStatus, MissingDataError> {
        let data = self.data.ok_or(MissingDataError)?;

        let online_status = OnlineStatus::new(
            &data.discord_status,
            data.active_on_discord_desktop || data.active_on_discord_web,
            data.active_on_discord_mobile,
        );

        let user = data.discord_user;

        let mut music_activities = Vec::new();
        let activities = data
            .activities
            .into_iter()
            .filter(|activity| {
                if let Some(music_activity) = music_activity_filters
                    .iter()
                    .find_map(|filter| filter.apply(&activity))
                {
                    music_activities.push(music_activity);
                    return false;
                }
                true
            })
            .collect();

        Ok(DiscordStatus {
            online_status,
            user,
            music_activity: music_activities.first().cloned(),
            activities,
        })
    }
}
