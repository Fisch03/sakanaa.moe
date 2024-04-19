use super::types::*;
use crate::api::{discord::*, lastfm::*};
use crate::components::*;
use crate::config::config;
use crate::db::music::audio_processing::metadata::CoverArt;
use crate::response_helpers::BinaryResource;
use fishnet::component::prelude::*;

use axum::{
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
struct LiveActivityComponentState {
    config: LiveActivityConfig,

    render: Markup,
    cover_art: Option<BinaryResource>,

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

impl LiveActivityComponentState {
    async fn fetch_lanyard_live_activity(&self) -> Option<LiveActivity> {
        let response = LanyardResponse::fetch().await.ok()?;

        LiveActivity::from_lanyard_response(response, &self.config.music_filters).ok()
    }

    async fn update(&mut self, endpoint: &str) {
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
                MusicActivity::from(track, &format!("{}/cover_art", endpoint))
            });
        }

        self.render = new_activity.render(&self.config.custom_filters);
    }
}

pub struct LiveActivityComponent {}
impl LiveActivityComponent {
    async fn status_handler(
        api: Extension<ComponentState<Arc<Mutex<LiveActivityComponentState>>>>,
    ) -> Markup {
        let mut api = api.lock().await;

        api.last_request = Some(std::time::Instant::now());

        api.render.clone()
    }

    async fn cover_art_handler(
        state: Extension<ComponentState<Arc<Mutex<LiveActivityComponentState>>>>,
        req_headers: HeaderMap,
    ) -> Response {
        let state = state.lock().await;

        if let Some(cover_art) = &state.cover_art {
            cover_art.respond(&req_headers).await
        } else {
            StatusCode::NOT_FOUND.into_response()
        }
    }

    pub fn new() -> impl BuildableComponent {
        let state = Arc::new(Mutex::new(LiveActivityComponentState {
            config: config().page.live_activity.clone(),
            render: html! {},
            cover_art: None,
            last_request: None,
            last_update: None,
        }));

        component!(LiveActivity)
            .with_state(state.clone())
            .route("/", get(Self::status_handler))
            .route("/cover_art", get(Self::cover_art_handler))
            .with_runner(|component_state| {
                async move {
                    loop {
                        let endpoint = component_state.endpoint();
                        let mut state = component_state.lock().await;

                        // If some user on the webpage has requested the status in the last 15 seconds, update the status often
                        if let Some(last_request) = state.last_request {
                            if last_request.elapsed().as_secs() < 15 {
                                state.update(endpoint).await;
                            }
                        // Otherwise update the status slowly
                        } else if let Some(last_update) = state.last_update {
                            if last_update.elapsed().as_secs() > 45 {
                                state.update(endpoint).await;
                            }
                        // If the status has never been updated, update it
                        } else {
                            state.update(endpoint).await;
                        }
                        drop(state);

                        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                    }
                }
                .boxed()
            })
            .render(|state| {
                section_raw(
                    //TODO: make this render dynamically. for that, the render function needs to be
                    //a future
                    html! {
                        div hx-get=(state.endpoint()) hx-trigger="load, every 5s" {}
                    },
                    //htmx!("every 5s", state.endpoint()),
                    &SectionConfig {
                        id: Some("Discord"),
                        ..Default::default()
                    },
                )
            })
    }
}
