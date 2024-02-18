mod types;
use types::{DiscordStatus, MusicActivityFilter};

mod lanyard;
use lanyard::LanyardResponse;

use crate::config::CONFIG;

use axum::{async_trait, extract::State, routing::get, Router};

use simple_error::SimpleError;

use super::{ApiEndpoint, EndpointDescriptor};
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Debug)]
pub struct DiscordAPI {
    user_id: String,
    music_activity_filter: Vec<MusicActivityFilter>,
    activity: Option<DiscordStatus>,
    last_request: Option<std::time::Instant>,
    last_update: Option<std::time::Instant>,
}

impl DiscordAPI {
    async fn update(&mut self) {
        println!("Updating Discord status");
        self.last_update = Some(std::time::Instant::now());

        if let Ok(response) = LanyardResponse::fetch(&self.user_id).await {
            self.activity = response.to_status(&self.music_activity_filter).ok();
        } else {
            self.activity = None;
        }
    }

    async fn status_handler(State(api): State<Arc<Mutex<DiscordAPI>>>) -> String {
        let mut api = api.lock().await;

        println!("Handling Discord status");
        api.last_request = Some(std::time::Instant::now());

        match &api.activity {
            Some(activity) => serde_json::to_string(&activity).unwrap(),
            None => serde_json::json!({}).to_string(),
        }
    }
}

#[async_trait]
impl ApiEndpoint for DiscordAPI {
    fn new() -> Result<EndpointDescriptor, SimpleError> {
        let user_id = CONFIG
            .get::<String>("discord.user_id")
            .map_err(|_| SimpleError::new("No user_id found in config"))?;
        let music_activity_filter = CONFIG
            .get_array("discord.music_activity_filters")
            .unwrap_or(Vec::new());

        let music_activity_filter = music_activity_filter
            .into_iter()
            .filter_map(|filter| filter.try_deserialize().ok())
            .collect();

        let api = Arc::new(Mutex::new(Self {
            user_id,
            music_activity_filter,
            activity: None,
            last_request: None,
            last_update: None,
        }));

        let router = Router::new()
            .route("/", get(Self::status_handler))
            .with_state(api.clone());

        Ok((router, api))
    }

    async fn run(&mut self) -> tokio::time::Duration {
        // If some user on the webpage has requested the status in the last 30 seconds, update the status often
        if let Some(last_request) = self.last_request {
            if last_request.elapsed().as_secs() < 30 {
                self.update().await;
            }
        // Otherwise update the status slowly
        } else if let Some(last_update) = self.last_update {
            if last_update.elapsed().as_secs() > 120 {
                self.update().await;
            }
        // If the status has never been updated, update it
        } else {
            self.update().await;
        }

        tokio::time::Duration::from_secs(3)
    }
}
