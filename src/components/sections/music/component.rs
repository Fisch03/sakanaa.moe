use super::music_upper::music_upper;

//use crate::api::lastfm::*;
use crate::components::*;
use crate::dyn_component::*;

#[derive(Debug)]
pub struct MusicComponent {}

impl MusicComponent {
    fn update(&mut self) {}
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
    fn new(_full_path: &str) -> Result<ComponentDescriptor> {
        let component = Arc::new(Mutex::new(Self {}));

        Ok(ComponentDescriptor {
            component,
            router: None,
            script_paths: None,
        })
    }

    async fn run(&mut self) -> tokio::time::Duration {
        tokio::time::Duration::from_secs(60 * 60)
    }
}
