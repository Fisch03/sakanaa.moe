mod render;
mod types;
use types::*;

use crate::api::{discord::*, lastfm::*};
use crate::components::*;
use crate::config::CONFIG;
use crate::dyn_component::*;

use axum::{extract::State, routing::get, Router};

#[derive(Debug)]
pub struct LiveActivityComponent {
    discord_user_id: String,
    discord_music_activity_filter: Vec<MusicActivityFilter>,
    discord_custom_activity_filter: Vec<CustomActivityFilter>,

    activity: LiveActivity,
    render_endpoint: String,

    last_request: Option<std::time::Instant>,
    last_update: Option<std::time::Instant>,
}

impl LiveActivityComponent {
    async fn fetch_lanyard_live_activity(&self) -> Option<LiveActivity> {
        let response = LanyardResponse::fetch(&self.discord_user_id).await.ok()?;

        LiveActivity::from_lanyard_response(response, &self.discord_music_activity_filter).ok()
    }

    async fn update(&mut self) {
        self.last_update = Some(std::time::Instant::now());

        let mut new_activity: LiveActivity;

        if let Some(activity) = self.fetch_lanyard_live_activity().await {
            new_activity = activity;
        } else {
            new_activity = LiveActivity {
                online_status: None,
                discord_user: None,
                music_activity: None,
                discord_activities: Vec::new(),
            };
        }

        if new_activity.music_activity.is_none() {
            new_activity.music_activity =
                get_current_track(&CONFIG.get::<String>("lastfm.user").unwrap())
                    .await
                    .ok()
                    .flatten()
                    .map(|track| MusicActivity::from_lastfm_track(track));
        }

        self.activity = new_activity;
    }

    async fn status_handler(State(api): State<Arc<Mutex<LiveActivityComponent>>>) -> Markup {
        let mut api = api.lock().await;

        api.last_request = Some(std::time::Instant::now());

        api.activity.render(&api.discord_custom_activity_filter)
    }
}

impl Render for LiveActivityComponent {
    fn render(&self) -> Markup {
        section_raw(
            self.activity.render(&self.discord_custom_activity_filter),
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
            .map_err(|_| SimpleError::new("No discord.user_id found in config"))?;
        let discord_music_activity_filter = CONFIG
            .get_array("discord.activity_filters.music")
            .unwrap_or(Vec::new());
        let discord_music_activity_filter = discord_music_activity_filter
            .into_iter()
            .filter_map(|filter| filter.try_deserialize().ok())
            .collect();

        let discord_custom_activity_filter = CONFIG
            .get_array("discord.activity_filters.custom")
            .unwrap_or(Vec::new());
        let discord_custom_activity_filter = discord_custom_activity_filter
            .into_iter()
            .filter_map(|filter| filter.try_deserialize().ok())
            .collect();

        let component = Arc::new(Mutex::new(Self {
            render_endpoint: full_path.to_string(),

            discord_user_id,
            discord_music_activity_filter,
            discord_custom_activity_filter,

            activity: LiveActivity::default(),

            last_request: None,
            last_update: None,
        }));

        let router = Router::new()
            .route("/", get(Self::status_handler))
            .with_state(component.clone());

        Ok(ComponentDescriptor {
            component,
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
