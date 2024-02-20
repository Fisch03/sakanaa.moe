use axum::{async_trait, Router};
use maud::Render;
use simple_error::SimpleError;
use std::sync::Arc;
use tokio::sync::Mutex;

pub type SharedDynamicComponent = Arc<Mutex<dyn DynamicComponent>>;
pub type DynamicComponentConstructor = fn(&str) -> Result<ComponentDescriptor, SimpleError>;
pub struct ComponentDescriptor {
    pub component: SharedDynamicComponent,
    pub router: Option<Router>,
}

#[async_trait]
pub trait DynamicComponent: Send + Render {
    fn new(full_path: &str) -> Result<ComponentDescriptor, SimpleError>
    where
        Self: Sized;

    async fn run(&mut self) -> tokio::time::Duration;
}
