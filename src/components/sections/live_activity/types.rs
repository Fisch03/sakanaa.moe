use serde::{Deserialize, Serialize};

use crate::api::discord::*;
use crate::api::lastfm::*;

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct LiveActivity {
    pub online_status: Option<OnlineStatus>,
    pub discord_user: Option<DiscordUser>,
    pub music_activity: Option<MusicActivity>,
    pub discord_activities: Vec<DiscordActivity>,
}

impl LiveActivity {
    pub fn from_lanyard_response(
        response: LanyardResponse,
        music_activity_filters: &Vec<MusicActivityFilter>,
    ) -> Result<Self, MissingDataError> {
        let data = response.data.ok_or(MissingDataError)?;

        let online_status = OnlineStatus::new(
            &data.discord_status,
            data.active_on_discord_desktop || data.active_on_discord_web,
            data.active_on_discord_mobile,
        );

        let discord_user = data.discord_user;

        let mut music_activities = Vec::new();
        let discord_activities = data
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

        Ok(LiveActivity {
            online_status: Some(online_status),
            discord_user: Some(discord_user),
            music_activity: music_activities.first().cloned(),
            discord_activities,
        })
    }
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

impl MusicActivity {
    pub fn from_lastfm_track(track: LastFMTrack) -> Self {
        MusicActivity {
            timestamps: ActivityTimestamps {
                start: None,
                end: None,
            },
            song_title: track.name,
            artist: Some(track.artist.name),
            album_artist: None,
            album_art: None, //TODO: fetch this from musicbrainz or library (same as music section)
            album: None,
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct CustomActivityFilter {
    #[serde(with = "serde_regex")]
    name_match: regex::Regex,
    new_title: String,
    hide_name: bool,
}

impl CustomActivityFilter {
    pub fn apply(&self, activity: &DiscordActivity) -> Option<DiscordActivity> {
        if !self.name_match.is_match(activity.name.as_ref()?) {
            return None;
        }

        let mut new_activity = activity.clone();
        new_activity.name = if self.hide_name {
            None
        } else {
            Some(self.new_title.clone())
        };

        new_activity.custom_title = Some(self.new_title.clone());

        Some(new_activity)
    }
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
