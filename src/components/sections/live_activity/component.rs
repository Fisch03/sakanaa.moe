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

    async fn update(state: ComponentState<Arc<Mutex<Self>>>) {
        let mut new_activity: LiveActivity;

        if let Some(activity) = state.lock().await.fetch_lanyard_live_activity().await {
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
            let current_track = get_current_track().await.ok().flatten();

            let mut state_guard = state.lock().await;
            new_activity.music_activity = current_track.map(|track| {
                state_guard.cover_art = track.cover.as_ref().map(|cover| {
                    BinaryResource::new(cover.to_jpg_thumb(), &track.name, "image/jpeg")
                });
                MusicActivity::from(track, &format!("{}/cover_art", state.endpoint()))
            });
        }

        let mut state = state.lock().await;
        state.render = new_activity.render(&state.config.custom_filters).await;
        state.last_update = Some(std::time::Instant::now());
    }
}

#[dyn_component]
pub fn live_activity() {
    let state = state_init!(Arc::new(Mutex::new(LiveActivityComponentState {
        config: config().page.live_activity.clone(),
        render: html! {},
        cover_art: None,
        last_request: None,
        last_update: None,
    })));

    #[route("/", GET)]
    async fn status_route(
        state: Extension<ComponentState<Arc<Mutex<LiveActivityComponentState>>>>,
    ) -> Markup {
        let mut state = state.lock().await;

        state.last_request = Some(std::time::Instant::now());

        state.render.clone()
    }

    #[route("/cover_art", GET)]
    async fn cover_art_route(
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

    runner!(loop {
        let state_guard = state.lock().await;
        let last_request = state_guard.last_request;
        let last_update = state_guard.last_update;
        drop(state_guard);

        // If some user on the webpage has requested the status in the last 15 seconds, update the status often
        if let Some(last_request) = last_request {
            if last_request.elapsed().as_secs() < 15 {
                LiveActivityComponentState::update(state.clone()).await;
            }
        // Otherwise update the status slowly
        } else if let Some(last_update) = last_update {
            if last_update.elapsed().as_secs() > 45 {
                LiveActivityComponentState::update(state.clone()).await;
            }
        // If the status has never been updated, update it
        } else {
            LiveActivityComponentState::update(state.clone()).await;
        }

        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
    });

    section_raw(
        html! {
            div hx-get=(state.endpoint()) hx-trigger="every 5s" {
                (state.lock().await.render.clone())
            }
        },
        &SectionConfig {
            id: Some("Discord"),
            ..Default::default()
        },
    )
    .await
}
