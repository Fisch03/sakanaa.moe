use chrono::{serde::ts_milliseconds_option, DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ActivityTimestamps {
    #[serde(with = "ts_milliseconds_option", default)]
    start: Option<DateTime<Utc>>,
    #[serde(with = "ts_milliseconds_option", default)]
    end: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct DiscordUser {
    pub username: String,
    pub public_flags: u64,
    pub id: String,
    pub avatar: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct DiscordAssets {
    pub large_image: Option<String>,
    pub large_text: Option<String>,
    pub small_image: Option<String>,
    pub small_text: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct DiscordActivity {
    pub r#type: u64,
    pub state: Option<String>,
    pub timestamps: ActivityTimestamps,
    pub application_id: Option<String>,

    pub name: Option<String>,
    pub details: Option<String>,
    pub assets: Option<DiscordAssets>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct MusicActivityFilter {
    #[serde(with = "serde_regex")]
    name_match: regex::Regex,

    title_src: String,
    #[serde(with = "serde_regex", default)]
    title_match: Option<regex::Regex>,

    artist_src: String,
    #[serde(with = "serde_regex", default)]
    artist_match: Option<regex::Regex>,

    album_src: String,
    #[serde(with = "serde_regex", default)]
    album_match: Option<regex::Regex>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct MusicActivity {
    pub timestamps: ActivityTimestamps,
    #[serde(rename = "song")]
    pub song_title: String,
    pub artist: Option<String>,
    pub album_artist: Option<String>,
    pub album_art: Option<String>,
    pub album: Option<String>,
}

pub fn discord_img_url(asset: &str, application_id: &str) -> String {
    if asset.starts_with("mp:external/") {
        format!("https://media.discordapp.net/external/{}", &asset[12..])
    } else {
        format!(
            "https://cdn.discordapp.com/app-assets/{}/{}",
            application_id, asset
        )
    }
}

impl MusicActivityFilter {
    fn get_match_src<'a>(src_name: &str, activity: &'a DiscordActivity) -> Option<&'a String> {
        match src_name {
            "name" => activity.name.as_ref(),
            "details" => activity.details.as_ref(),
            "state" => activity.state.as_ref(),
            "large_text" => activity.assets.as_ref().and_then(|a| a.large_text.as_ref()),
            "small_text" => activity.assets.as_ref().and_then(|a| a.small_text.as_ref()),
            _ => None,
        }
    }

    pub fn apply(&self, activity: &DiscordActivity) -> Option<MusicActivity> {
        let name = activity.name.as_ref()?;

        if !self.name_match.is_match(name) {
            return None;
        }

        let song_container = Self::get_match_src(&self.title_src, activity)?;
        let artist_container = Self::get_match_src(&self.artist_src, activity)?;
        let album_container = Self::get_match_src(&self.album_src, activity)?;

        let song_title: String = match &self.title_match {
            Some(title_match) => {
                let title = song_container.as_str();
                title_match.captures(title)?.get(1)?.as_str().to_string()
            }
            None => song_container.to_string(),
        };

        let artist: Option<String> = match &self.artist_match {
            Some(artist_match) => {
                let artist = artist_container.as_str();
                Some(artist_match.captures(artist)?.get(1)?.as_str().to_string())
            }
            None => Some(artist_container.to_string()),
        };

        let album: Option<String> = match &self.album_match {
            Some(album_match) => {
                let album = album_container.as_str();
                Some(album_match.captures(album)?.get(1)?.as_str().to_string())
            }
            None => Some(album_container.to_string()),
        };

        Some(MusicActivity {
            timestamps: activity.timestamps.clone(),
            song_title,
            artist,
            album_artist: None,
            album_art: activity.assets.as_ref().and_then(|a| {
                Some(discord_img_url(
                    &a.large_image.as_ref()?,
                    &activity.application_id.as_ref()?,
                ))
            }),
            album,
        })
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum DeviceType {
    Desktop,
    Mobile,
    Unknown,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum OnlineStatus {
    Online(DeviceType),
    Idle(DeviceType),
    DoNotDisturb(DeviceType),
    Invisible,
    Offline,
}

impl OnlineStatus {
    pub fn new(status: &str, is_desktop: bool, is_mobile: bool) -> Self {
        let device = if is_desktop {
            DeviceType::Desktop
        } else if is_mobile {
            DeviceType::Mobile
        } else {
            DeviceType::Unknown
        };

        match status {
            "online" => OnlineStatus::Online(device),
            "idle" => OnlineStatus::Idle(device),
            "dnd" => OnlineStatus::DoNotDisturb(device),
            "invisible" => OnlineStatus::Invisible,
            "offline" => OnlineStatus::Offline,
            _ => OnlineStatus::Offline,
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct LiveActivity {
    pub online_status: OnlineStatus,
    pub discord_user: DiscordUser,
    pub music_activity: Option<MusicActivity>,
    pub discord_activities: Vec<DiscordActivity>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct OptionalLiveActivity(pub Option<LiveActivity>);
