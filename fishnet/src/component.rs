pub mod htmx;
pub mod prelude;
pub mod render_context;

use crate::page::ScriptType;
use crate::routes::ComponentRoute;

use axum::{
    body::Body, http::Request, response::IntoResponse, routing::method_routing::MethodRouter,
    Extension, Router,
};
use core::convert::Infallible;
use futures::future::BoxFuture;
use futures::future::FutureExt;
use maud::{html, Markup};
use std::{fmt::Debug, marker::PhantomData, ops::Deref};
use tower_service::Service;
use tracing::{instrument, trace};

use nanoid::nanoid;
const ID_ALPHABET: [char; 26] = [
    'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm', 'n', 'o', 'p', 'q', 'r', 's',
    't', 'u', 'v', 'w', 'x', 'y', 'z',
];

#[derive(Debug, Clone)]
pub struct ComponentState<ST>
where
    ST: Clone + Send + Sync,
{
    api_route: ComponentRoute,
    state: ST,
}
impl<ST> ComponentState<ST>
where
    ST: Clone + Send + Sync,
{
    pub fn endpoint(&self) -> &str {
        self.api_route.as_str()
    }
}
impl<ST> Deref for ComponentState<ST>
where
    ST: Clone + Send + Sync,
{
    type Target = ST;
    fn deref(&self) -> &Self::Target {
        &self.state
    }
}

pub struct ComponentBuildResult {
    pub built_component: BuiltComponent,
    pub scripts: Vec<ScriptType>,
    pub runner: Option<BoxFuture<'static, ()>>,
    pub router: Option<(ComponentRoute, Router)>,
}

pub trait BuildableComponent {
    fn name(&self) -> &str;
    fn id(&self) -> &str;

    fn build(self: Self, base_route: &str) -> ComponentBuildResult;
}

pub type ContentRenderer<ST> = Box<dyn Fn(ComponentState<ST>) -> Markup + Send + Sync>;
pub type ComponentRunner<ST> =
    Box<dyn FnOnce(ComponentState<ST>) -> BoxFuture<'static, ()> + 'static>;

struct StatefulContentRenderer<ST>
where
    ST: Clone + Send + Sync,
{
    renderer: ContentRenderer<ST>,
    state: ComponentState<ST>,
}
pub trait StatefulRenderer: Send + Sync {
    fn render(&self) -> Markup;
}
impl<ST> StatefulRenderer for StatefulContentRenderer<ST>
where
    ST: Clone + Send + Sync,
{
    fn render(&self) -> Markup {
        (self.renderer)(self.state.clone())
    }
}

pub enum ContentType {
    Dynamic(Box<dyn StatefulRenderer>),
    Static(Markup),
}
impl ContentType {
    fn render(&self) -> Markup {
        match self {
            ContentType::Dynamic(renderer) => renderer.render(),
            ContentType::Static(content) => content.clone(),
        }
    }
}
impl std::fmt::Debug for ContentType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ContentType::Dynamic(_) => write!(f, "Dynamic"),
            ContentType::Static(_) => write!(f, "Static"),
        }
    }
}

#[doc(hidden)]
pub struct NoRenderer;
#[doc(hidden)]
pub struct HasRenderer;
#[doc(hidden)]
pub struct NoState;
#[doc(hidden)]
pub struct FixedNoState;
#[doc(hidden)]
pub struct HasState;
pub struct Component<R, S, ST>
where
    ST: Clone + Send + Sync,
{
    name: String,
    id: String,
    is_dynamic: bool,

    renderer: Option<ContentRenderer<ST>>,

    state: ST,
    router: Option<Router>,

    runner: Option<ComponentRunner<ST>>,
    scripts: Vec<ScriptType>,

    _renderer_state: PhantomData<R>,
    _state_state: PhantomData<S>,
}

#[derive(Debug)]
pub struct BuiltComponent {
    name: String,
    id: String,

    content: ContentType,
}
impl BuiltComponent {
    pub fn name(&self) -> &str {
        &self.name
    }
}

impl<ST, S> BuildableComponent for Component<HasRenderer, S, ST>
where
    ST: Clone + Send + Sync + 'static,
{
    fn name(&self) -> &str {
        &self.name
    }
    fn id(&self) -> &str {
        &self.id
    }

    #[instrument(name = "build_component", skip_all, fields(name = %self.name))]
    fn build(self, base_route: &str) -> ComponentBuildResult {
        let api_route = ComponentRoute::new(base_route, &self.name, &self.id);
        let state = ComponentState {
            api_route: api_route.clone(),
            state: self.state,
        };

        trace!("building routes");
        let router = self.router.map(|r| r.layer(Extension(state.clone())));

        trace!("rendering component");
        let renderer = self.renderer.unwrap();
        let render = renderer(state.clone());

        let runner = self.runner.map(|runner| {
            let runner = (runner)(state.clone());
            runner.boxed()
        });

        let content;
        if !self.is_dynamic {
            content = ContentType::Static(render);
        } else {
            content = ContentType::Dynamic(Box::new(StatefulContentRenderer { renderer, state }));
        }

        ComponentBuildResult {
            built_component: BuiltComponent {
                name: self.name,
                id: self.id,
                content,
            },
            scripts: self.scripts,
            runner,
            router: router.map(|r| (api_route, r)),
        }
    }
}

// ---- constructor ----
impl Component<NoRenderer, NoState, ()> {
    pub fn new(name: &str) -> Component<NoRenderer, NoState, ()> {
        let id = nanoid!(5, &ID_ALPHABET);

        Self {
            name: name.to_string(),
            id,
            is_dynamic: false,

            state: (),
            router: None,

            renderer: None,
            runner: None,
            scripts: Vec::new(),

            _renderer_state: PhantomData,
            _state_state: PhantomData,
        }
    }
}

// ---- on all components ----
impl<R, S, ST> Component<R, S, ST>
where
    ST: Clone + Send + Sync,
{
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn add_script(mut self, script: ScriptType) -> Self {
        self.scripts.push(script);
        self
    }

    pub fn route(mut self, path: &str, route: MethodRouter) -> Component<R, S, ST> {
        let router = self.router.unwrap_or_else(|| Router::new());
        self.router = Some(router.route(path, route));
        self
    }

    pub fn nest_service<T>(mut self, path: &str, service: T) -> Self
    where
        T: Service<Request<Body>, Error = Infallible> + Clone + Send + 'static,
        T::Response: IntoResponse,
        T::Future: Send + 'static,
    {
        let router = self.router.unwrap_or_else(|| Router::new());
        self.router = Some(router.nest_service(path, service));
        self
    }
}

// ---- adding a renderer ----
impl<S, ST> Component<NoRenderer, S, ST>
where
    ST: Clone + Send + Sync,
{
    pub fn render<C>(self, renderer: C) -> impl BuildableComponent
    where
        ST: Clone + Send + Sync + 'static,
        C: Fn(ComponentState<ST>) -> Markup + Send + Sync + 'static,
    {
        Component::<HasRenderer, S, ST> {
            name: self.name,
            id: self.id,

            is_dynamic: false,

            state: self.state,
            router: self.router,

            renderer: Some(Box::new(renderer)),
            runner: self.runner,
            scripts: self.scripts,

            _renderer_state: PhantomData,
            _state_state: PhantomData,
        }
    }

    pub fn render_dynamic<C>(self, renderer: C) -> impl BuildableComponent
    where
        ST: Clone + Send + Sync + 'static,
        C: Fn(ComponentState<ST>) -> Markup + Send + Sync + 'static,
    {
        Component::<HasRenderer, S, ST> {
            name: self.name,
            id: self.id,
            is_dynamic: true,
            state: self.state,
            router: self.router,
            renderer: Some(Box::new(renderer)),
            runner: self.runner,
            scripts: self.scripts,
            _renderer_state: PhantomData,
            _state_state: PhantomData,
        }
    }
}

// ---- adding a runner ----
impl<R> Component<R, FixedNoState, ()> {
    pub fn with_runner<F>(mut self, runner: ComponentRunner<()>) -> Self {
        self.runner = Some(Box::new(runner));
        self
    }
}
impl<R, ST> Component<R, HasState, ST>
where
    ST: Clone + Send + Sync + 'static,
{
    pub fn with_runner<F>(mut self, runner: F) -> Self
    where
        F: FnOnce(ComponentState<ST>) -> BoxFuture<'static, ()> + 'static,
    {
        self.runner = Some(Box::new(runner));
        self
    }
}

// ---- setting state ----
impl<R> Component<R, NoState, ()> {
    /// Add state to the component.
    /// Since the state will be passed around a lot, it should be cheap to clone. This usually
    /// means wrapping it in an Arc or similar.
    pub fn with_state<ST>(self, state: ST) -> Component<NoRenderer, HasState, ST>
    where
        ST: Clone + Send + Sync + 'static,
    {
        Component::<NoRenderer, HasState, ST> {
            name: self.name,
            id: self.id,

            is_dynamic: self.is_dynamic,

            state,
            router: self.router,

            renderer: None, // this is fine because there is no renderer on the component yet
            runner: None,   // runners can also only be added after with_state
            scripts: self.scripts,

            _renderer_state: PhantomData,
            _state_state: PhantomData,
        }
    }
}
