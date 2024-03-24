pub mod discord;
pub mod lastfm;
pub mod misskey;
pub mod musicbrainz;

use discord::DiscordConfig;
use lastfm::LastFMConfig;
use misskey::MisskeyConfig;

use serde::Deserialize;
#[derive(Debug, Deserialize)]
pub struct ApiConfig {
    pub discord: DiscordConfig,
    pub lastfm: LastFMConfig,
    pub misskey: MisskeyConfig,
}
