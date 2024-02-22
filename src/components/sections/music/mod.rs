mod music_upper;
use music_upper::music_upper;

use crate::api::lastfm::*;
use crate::components::*;
use crate::config::CONFIG;
use crate::dyn_component::*;

#[derive(Debug)]
pub struct MusicComponent {
    lastfm_user: String,
    lastfm_api_key: String,
}

impl Render for MusicComponent {
    fn render(&self) -> Markup {
        let sections = vec![
            music_upper(),
            //music_lower(),
        ];

        split_section(&sections)
    }
}

#[async_trait]
impl DynamicComponent for MusicComponent {
    fn new(_full_path: &str) -> Result<ComponentDescriptor, SimpleError> {
        let lastfm_user = CONFIG
            .get::<String>("lastfm.user")
            .map_err(|_| SimpleError::new("Missing lastfm.user in config"))?;

        let lastfm_api_key = CONFIG
            .get::<String>("lastfm.api_key")
            .map_err(|_| SimpleError::new("Missing lastfm.api_key in config"))?;

        let component = Arc::new(Mutex::new(MusicComponent {
            lastfm_user,
            lastfm_api_key,
        }));

        Ok(ComponentDescriptor {
            component,
            router: None,
        })
    }

    async fn run(&mut self) -> tokio::time::Duration {
        tokio::time::Duration::from_secs(60 * 60)
    }
}
