use super::types::*;
use crate::api::{discord::*, lastfm::*};
use crate::components::*;
use crate::config::config;
use crate::db::music::audio_processing::metadata::CoverArt;
use crate::dyn_component::*;
use crate::response_helpers::BinaryResource;

use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
};
use std::io::{BufWriter, Cursor};

use serde::Deserialize;
#[derive(Debug, Deserialize, Clone)]
pub struct LiveActivityConfig {
    #[serde(default)]
    music_filters: Vec<MusicActivityFilter>,
    #[serde(default)]
    custom_filters: Vec<CustomActivityFilter>,
}

#[derive(Debug)]
pub struct LiveActivityComponent {
    config: LiveActivityConfig,

    activity: LiveActivity,
    cover_art: Option<BinaryResource>,

    render_endpoint: String,
    cover_art_endpoint: String,

    last_request: Option<std::time::Instant>,
    last_update: Option<std::time::Instant>,
}

impl CoverArt {
    fn to_jpg_thumb(&self) -> Vec<u8> {
        let thumb = self.0.thumbnail(250, 250);
        let mut thumb_bytes = BufWriter::new(Cursor::new(Vec::new()));
        thumb
            .write_to(&mut thumb_bytes, image::ImageFormat::Jpeg)
            .unwrap();
        thumb_bytes.into_inner().unwrap().into_inner()
    }
}

impl LiveActivityComponent {
    async fn fetch_lanyard_live_activity(&self) -> Option<LiveActivity> {
        let response = LanyardResponse::fetch().await.ok()?;

        LiveActivity::from_lanyard_response(response, &self.config.music_filters).ok()
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
            new_activity.music_activity = get_current_track().await.ok().flatten().map(|track| {
                self.cover_art = track.cover.as_ref().map(|cover| {
                    BinaryResource::new(cover.to_jpg_thumb(), &track.name, "image/jpeg")
                });
                MusicActivity::from(track, &self.cover_art_endpoint)
            });
        }

        self.activity = new_activity;
    }

    async fn status_handler(State(api): State<Arc<Mutex<LiveActivityComponent>>>) -> Markup {
        let mut api = api.lock().await;

        api.last_request = Some(std::time::Instant::now());

        api.activity.render(&api.config.custom_filters)
    }

    async fn cover_art_handler(
        State(api): State<Arc<Mutex<LiveActivityComponent>>>,
        req_headers: HeaderMap,
    ) -> Response {
        let api = api.lock().await;

        if let Some(cover_art) = &api.cover_art {
            cover_art.respond(&req_headers).await
        } else {
            StatusCode::NOT_FOUND.into_response()
        }
    }
}

impl Render for LiveActivityComponent {
    fn render(&self) -> Markup {
        section_raw(
            self.activity.render(&self.config.custom_filters),
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
        let config = config().page.live_activity.clone();

        let render_endpoint = full_path.to_string();
        let cover_art_endpoint = full_path.to_string() + "/cover_art";

        let component = Arc::new(Mutex::new(Self {
            config,

            activity: LiveActivity::default(),
            cover_art: None,

            render_endpoint,
            cover_art_endpoint,

            last_request: None,
            last_update: None,
        }));

        let router = Router::new()
            .route("/", get(Self::status_handler))
            .route("/cover_art", get(Self::cover_art_handler))
            .with_state(component.clone());

        Ok(ComponentDescriptor {
            component,
            router: Some(router),
            script_paths: None,
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
