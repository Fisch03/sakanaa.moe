use crate::page::{BuiltPage, ScriptType};
use crate::routes::ComponentRoute;
use axum::routing::Router;
use futures::future::BoxFuture;
use maud::Markup;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tracing::{debug, trace};

use super::{BuildableComponent, BuiltComponent};
pub use const_random::const_random;

static RENDER_CONTEXT: Mutex<Option<RenderContext>> = Mutex::new(None);

type ComponentPathFragment = u64;
type ComponentPath = Vec<ComponentPathFragment>;

pub struct ComponentStore(HashMap<ComponentPath, BuiltComponent>);
impl ComponentStore {
    pub fn new() -> Self {
        Self(HashMap::new())
    }
}

struct RenderContext {
    base_route: String,

    components: Arc<Mutex<ComponentStore>>,
    current_path: ComponentPath,

    new_scripts: Vec<ScriptType>,
    new_runners: Vec<BoxFuture<'static, ()>>,
    new_routers: Vec<(ComponentRoute, Router)>,
}
impl RenderContext {
    fn new(base_route: &str, components: Arc<Mutex<ComponentStore>>) -> RenderContext {
        Self {
            base_route: base_route.to_string(),

            current_path: Vec::new(),
            components,

            new_scripts: Vec::new(),
            new_runners: Vec::new(),
            new_routers: Vec::new(),
        }
    }

    fn finish(self) -> RenderResult {
        RenderResult {
            scripts: self.new_scripts,
            runners: self.new_runners,
            routers: self.new_routers,
        }
    }
}

pub struct RenderResult {
    pub scripts: Vec<ScriptType>,
    pub runners: Vec<BoxFuture<'static, ()>>,
    pub routers: Vec<(ComponentRoute, Router)>,
}

pub fn enter_page(page: &mut BuiltPage) {
    let mut context = RENDER_CONTEXT.lock().unwrap();

    if context.is_some() {
        panic!("tried to render a page while another page is already being rendered");
    }

    context.replace(RenderContext::new(&page.api_path, page.components.clone()));
}

pub fn exit_page() -> RenderResult {
    let mut context = RENDER_CONTEXT.lock().unwrap();
    context
        .take()
        .expect("tried to exit a page while no page is being rendered")
        .finish()
}

pub fn render_component<F, C>(context_id: u64, lazy_component: F) -> Markup
where
    F: FnOnce() -> C,
    C: BuildableComponent,
{
    debug!("rendering component, id: {}", context_id);

    let mut context_guard = RENDER_CONTEXT.lock().unwrap();
    let mut context = context_guard
        .as_mut()
        .expect("tried to add a component while no page is being rendered");
    let mut components_guard = context.components.lock().unwrap();

    context.current_path.push(context_id);

    let render;
    if let Some(component) = components_guard.0.get(&context.current_path) {
        trace!("component already existed, rendering");
        render = component.content.render();
        context.current_path.pop();
    } else {
        let base_route = context.base_route.clone();

        drop(components_guard);
        drop(context_guard);

        // IMPORTANT: Since may lead to recursive calls, all the locks need to be dropped before calling
        let new_component = lazy_component().build(&base_route);

        context_guard = RENDER_CONTEXT.lock().unwrap();
        context = context_guard
            .as_mut()
            .expect("page render exited while a component was still being rendered");
        components_guard = context.components.lock().unwrap();

        context.new_scripts.extend(new_component.scripts);
        if let Some(router) = new_component.router {
            context.new_routers.push(router)
        }
        if let Some(runner) = new_component.runner {
            context.new_runners.push(runner);
        }

        render = new_component.built_component.content.render();
        components_guard
            .0
            .insert(context.current_path.clone(), new_component.built_component);
        context.current_path.pop();
    }

    render
}

/// Macro that lets you add components to the page.
/// This is done by wrapping the component in a `c!` macro. The component will then be
/// automatically built and rendered when needed.
/// ```rust
/// Page::new("example").with_content(|| {
///     html!{
///         (c!(Component::new("my awesome component").render(|_| {
///             html!{ "Hello World!" }
///         })))
///     }
/// });
/// ```
#[macro_export]
macro_rules! c {
    ($component:expr) => {{
        let component = || $component;

        $crate::component::render_context::render_component(
            $crate::component::render_context::const_random!(u64),
            component,
        )
    }};
}
