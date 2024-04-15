//! Commonly used imports for building components.

pub use crate::c;

// components itself
pub use super::{BuildableComponent, Component, ComponentState};

// html, js
pub use crate::page::ScriptType;
pub use maud::{html, Markup, Render};

// boxing runner futures
pub use futures::future::FutureExt;

// sharing state
pub use std::sync::Arc;
pub use tokio::sync::Mutex;
// extracting state
pub use axum::Extension;
