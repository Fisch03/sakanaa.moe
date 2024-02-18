mod types;
use types::Note;

use crate::config::CONFIG;

use axum::{async_trait, extract::State, routing::get, Router};

use simple_error::SimpleError;

use super::{ApiEndpoint, EndpointDescriptor};
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Debug)]
pub struct MisskeyAPI {
    base_url: String,
    user_id: String,

    notes: Vec<Note>,
}

/*
#[derive(Debug, serde::Serialize)]
struct NotesResponse {
    notes: Vec<Note>,
    //emojis: Vec<Emoji>,
    //instance: String,
}
*/

impl MisskeyAPI {
    async fn update(&mut self) {
        self.notes = self.fetch_notes().await.unwrap_or(Vec::new());
    }

    async fn fetch_notes(&self) -> Result<Vec<Note>, reqwest::Error> {
        let client = reqwest::Client::new();
        let res = client
            .post(format!("{}/api/users/notes", self.base_url))
            .json(&serde_json::json!({
                "userId": self.user_id,
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

    //#[debug_handler]
    async fn notes_handler(State(api): State<Arc<Mutex<MisskeyAPI>>>) -> String {
        let api = api.lock().await;
        serde_json::to_string(&api.notes).unwrap()
    }
}

#[async_trait]
impl ApiEndpoint for MisskeyAPI {
    fn new() -> Result<EndpointDescriptor, SimpleError> {
        let base_url = CONFIG
            .get::<String>("misskey.base_url")
            .map_err(|_| SimpleError::new("Failed to read misskey.base_url from config"))?;
        let user_id = CONFIG
            .get::<String>("misskey.user_id")
            .map_err(|_| SimpleError::new("Failed to read misskey.user_id from config"))?;

        let api = Arc::new(Mutex::new(Self {
            base_url,
            user_id,
            notes: Vec::new(),
        }));

        let router = Router::new()
            .route("/notes", get(Self::notes_handler))
            .with_state(api.clone());

        Ok((router, api))
    }

    async fn run(&mut self) -> tokio::time::Duration {
        println!("Fetching notes from Misskey API");
        self.update().await;

        tokio::time::Duration::from_secs(120)
    }
}
