use crate::api::misskey::*;
use crate::config::CONFIG;
use crate::dyn_component::*;

use axum::{async_trait, extract::State, routing::get, Router};

#[derive(Debug)]
pub struct MicrobloggingComponent {
    base_url: String,
    user_id: String,

    notes: Vec<Note>,
}

impl MicrobloggingComponent {
    async fn update(&mut self) {
        self.notes = fetch_notes(&self.base_url, &self.user_id)
            .await
            .unwrap_or(Vec::new());
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
    fn new(_full_path: &str) -> Result<ComponentDescriptor, SimpleError> {
        let base_url = CONFIG
            .get::<String>("misskey.base_url")
            .map_err(|_| SimpleError::new("Failed to read misskey.base_url from config"))?;
        let user_id = CONFIG
            .get::<String>("misskey.user_id")
            .map_err(|_| SimpleError::new("Failed to read misskey.user_id from config"))?;

        let component = Arc::new(Mutex::new(Self {
            base_url,
            user_id,
            notes: Vec::new(),
        }));

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
