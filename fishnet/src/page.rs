//! A visitable page on the [`Website`](crate::website::Website).

use axum::{routing::get, Extension, Router};
use maud::{html, Markup, DOCTYPE};
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, info, instrument};

mod bundler;
pub use bundler::ScriptType;

use crate::component::render_context::{self, ComponentStore};
use crate::routes::APIRouter;

use nanoid::nanoid;
const ID_ALPHABET: [char; 26] = [
    'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm', 'n', 'o', 'p', 'q', 'r', 's',
    't', 'u', 'v', 'w', 'x', 'y', 'z',
];

pub struct BuiltPage {
    name: String,
    id: String,

    renderer: Box<dyn Fn() -> Markup + Send + Sync>,
    pub components: Arc<std::sync::Mutex<ComponentStore>>,

    pub api_path: String,
    api_router: APIRouter,

    script_path: String,
    scripts: HashSet<ScriptType>,
    bundled_script: String,
}

impl BuiltPage {
    #[instrument(name = "Page::build", skip_all, fields(name = %page.name))]
    async fn new(page: Page, path: &str) -> Router {
        let base_path = path.trim_end_matches('/');
        let script_path = format!("{}/script.js", base_path);
        let api_path = format!("{}/api", base_path);

        let mut bundled_script = String::new();
        for script in &page.extra_scripts {
            bundled_script.push_str(&bundler::bundle_script(script).await);
        }

        let built_page = Self {
            name: page.name,
            id: page.id,

            renderer: page.content_renderer,
            components: Arc::new(std::sync::Mutex::new(ComponentStore::new())),

            api_path,
            api_router: APIRouter::new(&format!("{}/api", base_path)),

            script_path,
            scripts: page.extra_scripts,
            bundled_script,
        };

        debug!("building router");
        let api_router = built_page.api_router.make_router().await;
        Router::new()
            .route("/", get(BuiltPage::render))
            .route("/script.js", get(BuiltPage::script))
            .merge(api_router)
            .layer(Extension(Arc::new(Mutex::new(built_page))))
    }

    async fn render(page: Extension<Arc<Mutex<Self>>>) -> Markup {
        let mut page = page.lock().await;

        render_context::enter_page(&mut page);
        let render = (page.renderer)();
        let mut result = render_context::exit_page();

        for runner in result.runners {
            tokio::spawn(runner);
        }

        for (route, router) in result.routers.drain(..) {
            page.api_router.add_component(route, router).await;
        }

        for script in result.scripts {
            if !page.scripts.contains(&script) {
                page.bundled_script
                    .push_str(&bundler::bundle_script(&script).await);
                page.scripts.insert(script);
            }
        }

        html! {
            (DOCTYPE)
            html lang="en" {
                (render)
                script src=(page.script_path) {}
            }
        }
    }

    async fn script(page: Extension<Arc<Mutex<Self>>>) -> String {
        let page = page.lock().await;

        page.bundled_script.clone()
    }
}

/// A page represents a visitable route on the website.
///
/// It manages rendering of the content, preparing [scripts](ScriptType) and running components.
pub struct Page {
    name: String,
    id: String,

    content_renderer: Box<dyn Fn() -> Markup + Send + Sync>,

    extra_scripts: HashSet<ScriptType>,
}

impl Page {
    /// Create a new page.
    ///
    /// The name is only used for logging purposes.
    pub fn new(name: &str) -> Self {
        let mut extra_scripts = HashSet::new();
        extra_scripts.insert(ScriptType::External("js/htmx.min.js".into()));

        Self {
            name: name.into(),
            id: nanoid!(5, &ID_ALPHABET),

            content_renderer: Box::new(|| html! {}),

            extra_scripts,
        }
    }

    /// Add content to the page.
    ///
    /// This function takes in a closure that returns a rendered page.
    pub fn with_content<C>(mut self, content_renderer: C) -> Self
    where
        C: Fn() -> Markup + Send + Sync + 'static,
    {
        self.content_renderer = Box::new(content_renderer);
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
