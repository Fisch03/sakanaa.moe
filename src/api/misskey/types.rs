use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AvatarDecoration {
    id: String,
    angle: f64,
    flip_h: bool,
    url: String,
    offset_x: f64,
    offset_y: f64,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Instance {
    name: Option<String>,
    software_name: Option<String>,
    software_version: Option<String>,
    icon_url: Option<String>,
    favicon_url: Option<String>,
    theme_color: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct BadgeRole {
    name: String,
    icon_url: Option<String>,
    display_order: u64,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserLite {
    id: String,
    name: Option<String>,
    username: String,
    host: Option<String>,
    avatar_url: Option<String>,
    avatar_blurhash: Option<String>,
    avatar_decorations: Vec<AvatarDecoration>,
    is_bot: bool,
    is_cat: bool,
    instance: Option<Instance>,
    //emojis: Vec<Emoji>,
    online_status: String,
    #[serde(default)]
    badge_roles: Vec<BadgeRole>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DriveFileProperties {
    width: u64,
    height: u64,
    orientation: Option<f64>,
    avg_color: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DriveFolder {
    id: String,
    created_at: DateTime<Utc>,
    name: String,
    parent_id: Option<String>,
    folders_count: u64,
    files_count: u64,
    parent: Option<Box<DriveFolder>>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DriveFile {
    id: String,
    created_at: DateTime<Utc>,
    name: String,
    r#type: String,
    md5: String,
    size: u64,
    is_sensitive: bool,
    blurhash: Option<String>,
    properties: DriveFileProperties,
    url: String,
    thumbnail_url: Option<String>,
    comment: Option<String>,
    folder_id: Option<String>,
    folder: Option<DriveFolder>,
    user_id: Option<String>,
    user: Option<UserLite>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Poll {} //?

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Channel {
    id: String,
    name: String,
    color: String,
    is_sensitive: bool,
    allow_renote_to_external: bool,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Note {
    id: String,
    created_at: DateTime<Utc>,
    deleted_at: Option<DateTime<Utc>>,
    text: Option<String>,
    cw: Option<String>,
    user_id: String,
    user: UserLite,
    reply_id: Option<String>,
    renote_id: Option<String>,
    reply: Option<Box<Note>>,
    renote: Option<Box<Note>>,
    is_hidden: Option<bool>,
    visibility: String,
    #[serde(default)]
    mentions: Vec<String>,
    #[serde(default)]
    visible_user_ids: Vec<String>,
    #[serde(default)]
    file_ids: Vec<String>,
    #[serde(default)]
    files: Vec<DriveFile>,
    #[serde(default)]
    tags: Vec<String>,
    poll: Option<Poll>,
    channel_id: Option<String>,
    channel: Option<Channel>,
    local_only: bool,
    reaction_acceptance: Option<String>,
    //reactions: HashMap<Reaction, u64>,
    renote_count: u64,
    replies_count: u64,
    uri: Option<String>,
    url: Option<String>,
    #[serde(default)]
    reaction_and_user_pair_cache: Vec<String>,
    #[serde(default)]
    clipped_count: u64,
    //myReaction: Option<Reaction>,
}
