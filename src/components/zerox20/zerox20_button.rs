use crate::components::*;
use crate::db::music::BeatEvent;
use fishnet::component::prelude::*;

use axum::{routing::get, Json};
use serde::Serialize;
use tower_http::services::ServeFile;

#[derive(Debug)]
pub struct Zerox20ButtonComponentState {
    beat_info: Vec<BeatEvent>,
}

#[derive(Debug, Serialize)]
struct ZeroX20TrackInfo {
    track_location: String,
    beat_info: Vec<BeatEvent>,
}

impl Zerox20ButtonComponentState {
    async fn stream_provider(
        component: Extension<ComponentState<Arc<Mutex<Zerox20ButtonComponentState>>>>,
    ) -> Json<ZeroX20TrackInfo> {
        let endpoint = component.endpoint();
        let component = component.lock().await;

        Json(ZeroX20TrackInfo {
            track_location: format!("{}/music", endpoint),
            beat_info: component.beat_info.clone(),
        })
    }
}

impl Render for Zerox20ButtonComponentState {
    fn render(&self) -> Markup {
        html! {
            button id="0x20Btn"  class="music_reactive" { (filtered_image("assets/music.png")) }
        }
    }
}

impl Zerox20ButtonComponentState {
    pub fn new() -> impl BuildableComponent {
        // TODO: automatically encode currently playing file to mp3, keep track of encoded files to
        //       discard them after a while
        //
        //fs::write("test_audio/output.mp3", &processed.mp3_data).expect("Failed to write mp3 file");
        let state = Arc::new(Mutex::new(Self {
            beat_info: Vec::new(), //processed.beat_data,
        }));

        let serve_file = ServeFile::new("test_audio/output.mp3");

        Component::new("zerox20_button")
            .with_state(state)
            .add_script(ScriptType::External("js/howler.min.js".into()))
            .add_script(ScriptType::External("js/0x20.js".into()))
            .route("/", get(Self::stream_provider))
            .nest_service("/music", serve_file)
            .render(|_| {
                html! {
                    div class="zerox20_button" {
                        button id="0x20Btn"  class="music_reactive" { (filtered_image("assets/music.png")) }
                    }
                }
            })
    }
}
