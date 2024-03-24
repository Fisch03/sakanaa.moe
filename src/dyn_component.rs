pub use anyhow::Result;
pub use axum::{async_trait, Router};
pub use maud::{html, Markup, Render};
pub use std::sync::Arc;
pub use tokio::sync::Mutex;

pub type SharedDynamicComponent = Arc<Mutex<dyn DynamicComponent>>;
pub type DynamicComponentConstructor = fn(&str) -> Result<ComponentDescriptor>;
pub struct ComponentDescriptor {
    pub component: SharedDynamicComponent,
    pub router: Option<Router>,
    //TODO: add a way for a component to return used javascript files, so they can be included at the end of the page
}

#[async_trait]
pub trait DynamicComponent: Send + Render {
    fn new(full_path: &str) -> Result<ComponentDescriptor>
    where
        Self: Sized;

    async fn run(&mut self) -> tokio::time::Duration;
}
