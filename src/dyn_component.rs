pub use anyhow::Result;
pub use axum::{async_trait, Router};
pub use maud::{html, Markup, Render};
pub use std::path::PathBuf;
pub use std::sync::Arc;
pub use tokio::sync::Mutex;

use crate::website::Website;

pub struct JSComponent {
    render: Markup,
    script_paths: Vec<PathBuf>,
}
impl JSComponent {
    pub fn new(render: Markup, script_paths: Vec<PathBuf>) -> Self {
        Self {
            render,
            script_paths,
        }
    }

    pub fn render(self, website: &mut Website) -> Markup {
        website.add_scripts(self.script_paths);

        self.render
    }
}

pub type SharedDynamicComponent = Arc<Mutex<dyn DynamicComponent>>;
pub type DynamicComponentConstructor = fn(&str) -> Result<ComponentDescriptor>;
pub struct ComponentDescriptor {
    pub component: SharedDynamicComponent,
    pub router: Option<Router>,
    pub script_paths: Option<Vec<PathBuf>>,
}

#[async_trait]
pub trait DynamicComponent: Send + Render {
    fn new(full_path: &str) -> Result<ComponentDescriptor>
    where
        Self: Sized;

    async fn run(&mut self) -> tokio::time::Duration;
}
