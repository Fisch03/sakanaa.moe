use anyhow::Result;
use axum::{routing::get, Router};
use maud::{html, Markup, DOCTYPE};
use std::path::{Path, PathBuf};

use crate::components::colorfilter;
use crate::dyn_component::{DynamicComponentConstructor, SharedDynamicComponent};

#[allow(dead_code)]
struct Runner {
    inner: SharedDynamicComponent,
    runner: tokio::task::JoinHandle<()>,
}

impl Runner {
    fn new(inner: SharedDynamicComponent) -> Self {
        let runner;

        {
            let inner = inner.clone();
            runner = tokio::spawn(async move {
                loop {
                    let sleep_duration;
                    {
                        let mut inner = inner.lock().await;
                        sleep_duration = inner.run().await;
                    }
                    tokio::time::sleep(sleep_duration).await;
                }
            });
        }

        Self { inner, runner }
    }
}

pub struct Website {
    title: String,
    content: Markup,
    script_paths: Vec<PathBuf>,

    router: Router,
    router_path: String,

    runners: Vec<Runner>,
}

impl Website {
    pub fn new(title: &str, router_path: &str) -> Self {
        let router = Router::new();

        let runners = Vec::new();

        Self {
            title: title.to_string(),
            content: html! {},
            script_paths: vec![
                "js/!palettes.js".into(),
                "js/colors.js".into(),
                "js/htmx.min.js".into(),
            ],

            router,
            router_path: router_path.to_string(),

            runners,
        }
    }

    pub fn set_content(&mut self, content: Markup) {
        self.content = content;
    }

    pub fn add_dynamic_component(
        &mut self,
        name: &str,
        constructor: DynamicComponentConstructor,
    ) -> Result<SharedDynamicComponent> {
        let path = format!("/{}", name.trim_start_matches("/"));
        let full_path = format!("{}{}", &self.router_path, &path);
        let descriptor = constructor(&full_path)?;

        if let Some(component_router) = descriptor.router {
            self.router = self.router.clone().nest(&path, component_router);
        }

        if let Some(component_script_paths) = descriptor.script_paths {
            self.script_paths.extend(component_script_paths);
        }

        let runner = Runner::new(descriptor.component.clone());
        self.runners.push(runner);

        Ok(descriptor.component)
    }

    pub fn add_scripts(&mut self, paths: Vec<PathBuf>) {
        self.script_paths.extend(paths);
    }

    fn render(&mut self, script_src: &str) -> Markup {
        html! {
            (DOCTYPE)
            html lang="en" {
                head {
                    meta charset="utf-8";
                    meta name="viewport" content="width=device-width, initial-scale=1";
                    title { (self.title) }
                    link rel="stylesheet" href="css/style.css";
                }

                body style="background-image: url('assets/dither/bgdither.png')" class="ditherbg onex" {
                    (colorfilter())
                    (self.content)
                }
                script src=(script_src) {}
            }
        }
    }

    fn merge_scripts(&self) -> Vec<u8> {
        let mut out = Vec::new();
        for path in &self.script_paths {
            let path = Path::new("static/").join(path);
            let script = std::fs::read_to_string(path).unwrap();
            out.extend(script.as_bytes());
        }

        out
    }
}

pub type WebsiteRouter = Router<()>;
pub trait AttachWebsite {
    fn attach_website(self, website: Website) -> Self;
}

impl AttachWebsite for WebsiteRouter {
    fn attach_website(self, mut website: Website) -> Self {
        let minified_scripts = website.merge_scripts();

        self.route("/script.js", get(minified_scripts))
            .route("/", get(Website::render(&mut website, "/script.js")))
            .nest(&website.router_path, website.router.clone())
    }
}
