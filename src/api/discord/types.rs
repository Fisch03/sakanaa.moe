use chrono::{serde::ts_milliseconds_option, DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ActivityTimestamps {
    #[serde(with = "ts_milliseconds_option", default)]
    pub start: Option<DateTime<Utc>>,
    #[serde(with = "ts_milliseconds_option", default)]
    pub end: Option<DateTime<Utc>>,
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
    #[serde(default)]
    pub custom_title: Option<String>,

    pub r#type: u64,
    pub state: Option<String>,
    pub timestamps: ActivityTimestamps,
    pub application_id: Option<String>,

    pub name: Option<String>,
    pub details: Option<String>,
    pub assets: Option<DiscordAssets>,
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
