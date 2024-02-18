mod misskey;
use misskey::MisskeyAPI;

mod discord;
use discord::DiscordAPI;

use axum::{async_trait, Router};
use simple_error::SimpleError;

use std::sync::Arc;
use tokio::sync::Mutex;
type Endpoint = Arc<Mutex<dyn ApiEndpoint>>;
type EndpointDescriptor = (Router, Endpoint);

#[async_trait]
trait ApiEndpoint: Send {
    fn new() -> Result<EndpointDescriptor, SimpleError>
    where
        Self: Sized;

    async fn run(&mut self) -> tokio::time::Duration;
}

pub struct Runner {
    #[allow(dead_code)]
    inner: Endpoint,
    #[allow(dead_code)]
    runner: tokio::task::JoinHandle<()>,
}

impl Runner {
    fn new(inner: Endpoint) -> Self {
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

pub struct API {
    pub router: Router,
    #[allow(dead_code)]
    endpoints: Vec<Runner>,
}

const ENDPOINTS: [(&str, fn() -> Result<EndpointDescriptor, SimpleError>); 2] =
    [("/misskey", MisskeyAPI::new), ("/discord", DiscordAPI::new)];

impl API {
    pub fn new() -> Self {
        let mut router = Router::new();
        let mut endpoints = Vec::new();
        for (path, endpoint_descriptor) in ENDPOINTS.iter() {
            match endpoint_descriptor() {
                Ok(endpoint_descriptor) => {
                    router = router.nest(path, endpoint_descriptor.0);

                    let inner = endpoint_descriptor.1;
                    let runner = Runner::new(inner);
                    endpoints.push(runner);
                }
                Err(e) => eprintln!("Failed to create endpoint '{}': {}", path, e),
            }
        }
        API { router, endpoints }
    }
}
