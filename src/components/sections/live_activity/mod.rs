mod types;
use maud::{html, Markup, Render};
use types::{MusicActivityFilter, OptionalLiveActivity};

mod lanyard;
use lanyard::LanyardResponse;

mod render;

use crate::{components::*, config::CONFIG};

use axum::{async_trait, extract::State, routing::get, Router};

use simple_error::SimpleError;

use crate::dyn_component::{ComponentDescriptor, DynamicComponent};
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Debug)]
pub struct LiveActivityComponent {
    discord_user_id: String,
    discord_music_activity_filter: Vec<MusicActivityFilter>,

    activity: OptionalLiveActivity,
    render_endpoint: String,

    last_request: Option<std::time::Instant>,
    last_update: Option<std::time::Instant>,
}

impl LiveActivityComponent {
    async fn update(&mut self) {
        self.last_update = Some(std::time::Instant::now());

        if let Ok(response) = LanyardResponse::fetch(&self.discord_user_id).await {
            self.activity =
                OptionalLiveActivity(response.to_status(&self.discord_music_activity_filter).ok());
        } else {
            self.activity = OptionalLiveActivity(None);
        }
    }

    async fn status_handler(State(api): State<Arc<Mutex<LiveActivityComponent>>>) -> Markup {
        let mut api = api.lock().await;

        api.last_request = Some(std::time::Instant::now());

        html!((api.activity))
    }
}

impl Render for LiveActivityComponent {
    fn render(&self) -> Markup {
        section_raw(
            self.activity.render(),
            &SectionConfig {
                id: Some("Discord"),
                htmx: Some(HTMXConfig {
                    get: &self.render_endpoint,
                    trigger: "load, every 5s",
                }),
                ..Default::default()
            },
        )
    }
}

#[async_trait]
impl DynamicComponent for LiveActivityComponent {
    fn new(full_path: &str) -> Result<ComponentDescriptor, SimpleError> {
        let discord_user_id = CONFIG
            .get::<String>("discord.user_id")
            .map_err(|_| SimpleError::new("No user_id found in config"))?;
        let discord_music_activity_filter = CONFIG
            .get_array("discord.music_activity_filters")
            .unwrap_or(Vec::new());

        let discord_music_activity_filter = discord_music_activity_filter
            .into_iter()
            .filter_map(|filter| filter.try_deserialize().ok())
            .collect();

        let component = Arc::new(Mutex::new(Self {
            render_endpoint: full_path.to_string(),

            discord_user_id,
            discord_music_activity_filter,

            activity: OptionalLiveActivity(None),

            last_request: None,
            last_update: None,
        }));

        let router = Router::new()
            .route("/", get(Self::status_handler))
            .with_state(component.clone());

        Ok(ComponentDescriptor {
            component: component,
            router: Some(router),
        })
    }

    async fn run(&mut self) -> tokio::time::Duration {
        // If some user on the webpage has requested the status in the last 15 seconds, update the status often
        if let Some(last_request) = self.last_request {
            if last_request.elapsed().as_secs() < 15 {
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

        tokio::time::Duration::from_secs(5)
    }
}
