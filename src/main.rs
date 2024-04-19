use sakanaa_web::config::config;
use sakanaa_web::root_page;

use tracing_subscriber::{filter::LevelFilter, layer::SubscriberExt, prelude::*, EnvFilter};

use fishnet::website::Website;

#[tokio::main]
async fn main() {
    let _ = maud::html! {
        h1 { "Hello, world!" }

        p { "This is a test." }
    };

    let filter = EnvFilter::builder()
        .with_default_directive(LevelFilter::INFO.into())
        .from_env()
        .expect("failed to create filter");
    let fmt_subscriber = tracing_subscriber::fmt::layer()
        .with_thread_ids(true)
        .with_target(false)
        .with_filter(filter);
    let registry = tracing_subscriber::registry().with(fmt_subscriber);
    tracing::subscriber::set_global_default(registry).expect("failed to set subscriber");

    let website = Website::new()
        .compression(true)
        .serve_dir("static")
        .add_page("/", root_page());

    let port = config().server.port;
    website.serve(port).await;
}
