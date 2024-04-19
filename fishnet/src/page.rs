//! A visitable page on the [`Website`](crate::website::Website).

use axum::{http::header, response::IntoResponse, routing::get, Extension, Router};
use futures::future::{BoxFuture, FutureExt};
use maud::{html, Markup, DOCTYPE};
use std::any::TypeId;
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, instrument};

use crate::js::{self, ScriptType};
use crate::routes::APIRouter;

pub mod render_context;
use render_context::ComponentStore;

use nanoid::nanoid;
const ID_ALPHABET: [char; 26] = [
    'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm', 'n', 'o', 'p', 'q', 'r', 's',
    't', 'u', 'v', 'w', 'x', 'y', 'z',
];

pub struct BuiltPage {
    #[allow(dead_code)]
    name: String,
    #[allow(dead_code)]
    id: String,

    head: Markup,
    body_renderer: Box<dyn Fn() -> BoxFuture<'static, Markup> + Send + Sync>,

    pub used_components: HashSet<TypeId>,
    pub components: Arc<Mutex<ComponentStore>>,

    pub api_path: String,
    api_router: APIRouter,

    script_path: String,
    bundled_script: String,

    style_path: String,
    stylesheet: String,
}

impl BuiltPage {
    #[instrument(name = "Page::build", skip_all, fields(name = %page.name))]
    async fn new(page: Page, path: &str) -> Router {
        let base_path = path.trim_end_matches('/');
        let script_path = format!("{}/script.js", base_path);
        let style_path = format!("{}/style.css", base_path);

        let api_path = format!("{}/api", base_path);

        let mut bundled_script = String::new();
        for script in &page.extra_scripts {
            bundled_script.push_str(&js::minify_script(script).await);
        }

        let built_page = Self {
            name: page.name,
            id: page.id,

            head: page.head,
            body_renderer: page.body_renderer,

            used_components: HashSet::new(),
            components: Arc::new(Mutex::new(ComponentStore::new())),

            api_path,
            api_router: APIRouter::new(&format!("{}/api", base_path)),

            script_path,
            bundled_script,

            style_path,
            stylesheet: String::new(),
        };

        let api_router = built_page.api_router.make_router().await;
        let page_extension = Extension(Arc::new(Mutex::new(built_page)));

        // pre-render the page to save request time. this is obviously not guaranteed to prerender all the components, but it should get most of them.
        debug!("performing page pre-render");
        let _ = Self::render(page_extension.clone()).await;

        debug!("building router");
        Router::new()
            .route("/", get(BuiltPage::render))
            .route("/script.js", get(BuiltPage::script))
            .route("/style.css", get(BuiltPage::style))
            .merge(api_router)
            .layer(page_extension)
    }

    async fn render(page: Extension<Arc<Mutex<Self>>>) -> Markup {
        let start = std::time::Instant::now();

        let mut page = page.lock().await;

        render_context::enter_page(&mut page).await;
        let render = (page.body_renderer)().await;
        let mut result = render_context::exit_page().await;

        //dbg!(&page.components.lock().unwrap());

        for type_id in result.new_components.drain() {
            if page.used_components.contains(&type_id) {
                continue;
            }

            if let Some(component_globals) = render_context::global_store().get(type_id).await {
                if let Some(style) = &component_globals.style {
                    page.stylesheet.push_str(style);
                }

                for script in &component_globals.scripts {
                    page.bundled_script
                        .push_str(&js::minify_script(script).await);
                }
            }
        }

        for runner in result.runners {
            tokio::spawn(runner);
        }

        for (route, router) in result.routers.drain(..) {
            page.api_router.add_component(route, router).await;
        }

        debug!("page render took {:?}", start.elapsed());

        html! {
            (DOCTYPE)
            html lang="en" {
                head {
                    (page.head)
                    link rel="stylesheet" href=(page.style_path) {}
                }
                (render)
                script src=(page.script_path) {}
            }
        }
    }

    // Endpoint for serving the bundled script.
    async fn script(page: Extension<Arc<Mutex<Self>>>) -> impl IntoResponse {
        let page = page.lock().await;

        (
            [(header::CONTENT_TYPE, "application/javascript")],
            page.bundled_script.clone(),
        )
    }

    // Endpoint for serving the stylesheet.
    async fn style(page: Extension<Arc<Mutex<Self>>>) -> impl IntoResponse {
        let page = page.lock().await;

        (
            [(header::CONTENT_TYPE, "text/css")],
            page.stylesheet.clone(),
        )
    }
}

/// A page represents a visitable route on the website.
///
/// It manages rendering of the content, preparing [scripts](ScriptType) and running components.
pub struct Page {
    name: String,
    id: String,

    head: Markup,
    body_renderer: Box<dyn Fn() -> BoxFuture<'static, Markup> + Send + Sync>,

    extra_scripts: HashSet<ScriptType>,
}

impl Page {
    /// Create a new page.
    ///
    /// The name is only used for logging purposes.
    pub fn new(name: &str) -> Self {
        let mut extra_scripts = HashSet::new();
        extra_scripts.insert(ScriptType::Inline(include_str!("../htmx/dist/htmx.js")));

        Self {
            name: name.into(),
            id: nanoid!(5, &ID_ALPHABET),

            head: html! {},
            body_renderer: Box::new(|| {
                async {
                    html! {}
                }
                .boxed()
            }),

            extra_scripts,
        }
    }

    pub fn with_head(mut self, head: Markup) -> Self {
        self.head = head;
        self
    }

    /// Add content to the page.
    ///
    /// This function takes in a closure that returns a rendered page.
    pub fn with_body<C>(mut self, content_renderer: C) -> Self
    where
        C: Fn() -> BoxFuture<'static, Markup> + Send + Sync + 'static,
    {
        self.body_renderer = Box::new(content_renderer);
        self
    }
}

/// Allows attaching a page to a router.
pub trait RouterPageExt {
    /// Attach the given page to the router. This involves building the page and adding multiple routes for the api, scripts and content.
    fn attach_page(self, path: &str, page: Page) -> Self;
}

impl RouterPageExt for Router {
    fn attach_page(self, path: &str, page: Page) -> Self {
        futures::executor::block_on(async { BuiltPage::new(page, path).await })
    }
}
