use super::types::*;
use crate::config::config;

use serde::Deserialize;
#[derive(Debug, Deserialize)]
pub struct MisskeyConfig {
    base_url: String,
    user_id: String,
}

pub async fn fetch_notes() -> Result<Vec<Note>, reqwest::Error> {
    let client = config().server.client();
    let base_url = &config().api.misskey.base_url;
    let user_id = &config().api.misskey.user_id;

    let res = client
        .post(format!("{}/api/users/notes", base_url))
        .json(&serde_json::json!({
            "userId": user_id,
            "includeMyRenotes": false,
            "includeReplies": false,
            "excludeNsfw": true
        }))
        .send()
        .await?
        .json::<Vec<Note>>()
        .await?;

    Ok(res)
}
