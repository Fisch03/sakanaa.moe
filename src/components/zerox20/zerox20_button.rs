use crate::components::*;
use crate::db::music::BeatEvent;
use fishnet::component::prelude::*;

use axum::Json;
use serde::Serialize;
//use tower_http::services::ServeFile;

#[derive(Debug)]
pub struct Zerox20ButtonComponentState {
    beat_info: Vec<BeatEvent>,
}

#[derive(Debug, Serialize)]
struct ZeroX20TrackInfo {
    track_location: String,
    beat_info: Vec<BeatEvent>,
}

impl Zerox20ButtonComponentState {}

impl Render for Zerox20ButtonComponentState {
    fn render(&self) -> Markup {
        html! {
            button id="0x20Btn"  class="music_reactive" { (filtered_image("assets/music.png")) }
        }
    }
}

#[component]
pub fn zerox20_button() {
    // TODO: automatically encode currently playing file to mp3, keep track of encoded files to
    //       discard them after a while
    //
    //fs::write("test_audio/output.mp3", &processed.mp3_data).expect("Failed to write mp3 file");
    let state = state!(Arc<Mutex<Vec<BeatEvent>>>);

    /*
    let serve_file = ServeFile::new("test_audio/output.mp3");
    route_service!("/music", serve_file);
    */

    #[route("/", GET)]
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

    script_external!("js/howler.min.js");
    script_external!("js/0x20.js");

    html! {
        div class="zerox20_button" {
            button id="0x20Btn"  class="music_reactive" { (filtered_image("assets/music.png")) }
        }
    }
}
