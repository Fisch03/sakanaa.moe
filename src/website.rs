use axum::Router;
use maud::{html, Markup, Render, DOCTYPE};
use simple_error::SimpleError;

use crate::components::colorfilter;
use crate::dyn_component::{DynamicComponentConstructor, SharedDynamicComponent};

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
    pub content: Markup,

    router: Router,
    router_path: String,
    #[allow(dead_code)]
    runners: Vec<Runner>,
    frozen: bool,
}

impl Render for Website {
    fn render(&self) -> Markup {
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

                script src="js/htmx.min.js" {}
            }
        }
    }
}

impl Website {
    pub fn new(title: &str, router_path: &str) -> Self {
        let router = Router::new();

        let runners = Vec::new();

        Self {
            title: title.to_string(),
            content: html! {},
            router,
            router_path: router_path.to_string(),
            runners,
            frozen: false,
        }
    }

    pub fn add_dynamic_component(
        &mut self,
        name: &str,
        constructor: DynamicComponentConstructor,
    ) -> Result<SharedDynamicComponent, SimpleError> {
        if self.frozen {
            return Err(SimpleError::new("Website is frozen"));
        }
        let path = format!("/{}", name.trim_start_matches("/"));
        let full_path = format!("{}{}", &self.router_path, &path);
        let descriptor = constructor(&full_path)?;

        if let Some(component_router) = descriptor.router {
            self.router = self.router.clone().nest(&path, component_router);
        }

        let runner = Runner::new(descriptor.component.clone());
        self.runners.push(runner);

        Ok(descriptor.component)
    }
}

pub type WebsiteRouter = Router<()>;
pub trait AttachWebsite {
    fn attach_website(self, website: &mut Website) -> Self;
}

impl AttachWebsite for WebsiteRouter {
    fn attach_website(self, website: &mut Website) -> Self {
        website.frozen = true;
        self.nest(&website.router_path, website.router.clone())
    }
}
