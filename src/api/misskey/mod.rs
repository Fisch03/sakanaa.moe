pub mod types;
pub use types::*;

use crate::config::CLIENT;

pub async fn fetch_notes(base_url: &str, user_id: &str) -> Result<Vec<Note>, reqwest::Error> {
    let res = CLIENT
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
