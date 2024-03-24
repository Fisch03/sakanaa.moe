use crate::api::misskey::{fetch_notes, types::Note};
use crate::dyn_component::*;

use axum::{async_trait, extract::State, routing::get};

#[derive(Debug)]
pub struct MicrobloggingComponent {
    notes: Vec<Note>,
}

impl MicrobloggingComponent {
    async fn update(&mut self) {
        self.notes = fetch_notes().await.unwrap_or(Vec::new());
    }

    async fn notes_handler(State(api): State<Arc<Mutex<MicrobloggingComponent>>>) -> String {
        let api = api.lock().await;
        serde_json::to_string(&api.notes).unwrap()
    }
}

impl Render for MicrobloggingComponent {
    fn render(&self) -> Markup {
        todo!();
    }
}

#[async_trait]
impl DynamicComponent for MicrobloggingComponent {
    fn new(_full_path: &str) -> Result<ComponentDescriptor> {
        let component = Arc::new(Mutex::new(Self { notes: Vec::new() }));

        let router = Router::new()
            .route("/notes", get(Self::notes_handler))
            .with_state(component.clone());

        Ok(ComponentDescriptor {
            component,
            router: Some(router),
        })
    }

    async fn run(&mut self) -> tokio::time::Duration {
        println!("Fetching notes from Misskey API");
        self.update().await;

        tokio::time::Duration::from_secs(120)
    }
}
