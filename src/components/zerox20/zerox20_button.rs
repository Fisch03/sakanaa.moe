use crate::components::*;
use crate::db::music::BeatEvent;
use crate::dyn_component::*;

use axum::{extract::State, routing::get, Json};
use serde::Serialize;
use tower_http::services::ServeFile;

#[derive(Debug)]
pub struct Zerox20ButtonComponent {
    base_url: String,
    beat_info: Vec<BeatEvent>,
}

#[derive(Debug, Serialize)]
struct ZeroX20TrackInfo {
    track_location: String,
    beat_info: Vec<BeatEvent>,
}

impl Zerox20ButtonComponent {
    async fn stream_provider(
        State(component): State<Arc<Mutex<Zerox20ButtonComponent>>>,
    ) -> Json<ZeroX20TrackInfo> {
        let component = component.lock().await;

        Json(ZeroX20TrackInfo {
            track_location: format!("{}/music", component.base_url),
            beat_info: component.beat_info.clone(),
        })
    }
}

impl Render for Zerox20ButtonComponent {
    fn render(&self) -> Markup {
        html! {
            button id="0x20Btn"  class="music_reactive" { (filtered_image("assets/music.png")) }
        }
    }
}

#[async_trait]
impl DynamicComponent for Zerox20ButtonComponent {
    fn new(full_path: &str) -> Result<ComponentDescriptor> {
        // TODO: automatically encode currently playing file to mp3, keep track of encoded files to
        //       discard them after a while
        //
        //fs::write("test_audio/output.mp3", &processed.mp3_data).expect("Failed to write mp3 file");
        let component = Arc::new(Mutex::new(Self {
            base_url: full_path.to_string(),
            beat_info: Vec::new(), //processed.beat_data,
        }));

        let serve_file = ServeFile::new("test_audio/output.mp3");

        let router = Router::new()
            .route("/", get(Self::stream_provider))
            .nest_service("/music", serve_file)
            .with_state(component.clone());

        Ok(ComponentDescriptor {
            component,
            router: Some(router),
            script_paths: Some(vec!["js/0x20.js".into(), "js/howler.min.js".into()]),
        })
    }

    async fn run(&mut self) -> tokio::time::Duration {
        return tokio::time::Duration::from_secs(60 * 60);
    }
}
