use super::types::*;
use crate::api::{discord::*, lastfm::*};
use crate::components::*;
use crate::config::config;
use crate::dyn_component::*;

use axum::{extract::State, routing::get};

#[derive(Debug)]
pub struct LiveActivityComponent {
    discord_user_id: String,
    discord_music_activity_filter: Vec<MusicActivityFilter>,
    discord_custom_activity_filter: Vec<CustomActivityFilter>,

    lastfm_user: String,

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
            new_activity.music_activity = get_current_track(&self.lastfm_user)
                .await
                .ok()
                .flatten()
                .map(|track| track.into());
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
                    trigger: "every 5s",
                }),
                ..Default::default()
            },
        )
    }
}

#[async_trait]
impl DynamicComponent for LiveActivityComponent {
    fn new(full_path: &str) -> Result<ComponentDescriptor> {
        let discord_user_id = config().get::<String>("discord.user_id")?;
        let discord_music_activity_filter = config()
            .get_array("discord.activity_filters.music")
            .unwrap_or(Vec::new());
        let discord_music_activity_filter = discord_music_activity_filter
            .into_iter()
            .filter_map(|filter| filter.try_deserialize().ok())
            .collect();

        let discord_custom_activity_filter = config()
            .get_array("discord.activity_filters.custom")
            .unwrap_or(Vec::new());
        let discord_custom_activity_filter = discord_custom_activity_filter
            .into_iter()
            .filter_map(|filter| filter.try_deserialize().ok())
            .collect();

        let lastfm_user = config().get::<String>("lastfm.user")?;

        let component = Arc::new(Mutex::new(Self {
            render_endpoint: full_path.to_string(),

            discord_user_id,
            discord_music_activity_filter,
            discord_custom_activity_filter,

            lastfm_user,

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
            if last_update.elapsed().as_secs() > 45 {
                self.update().await;
            }
        // If the status has never been updated, update it
        } else {
            self.update().await;
        }

        tokio::time::Duration::from_secs(5)
    }
}
