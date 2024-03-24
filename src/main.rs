use maud::Render;
use sakanaa_web::config::config;
use sakanaa_web::root_page;
use sakanaa_web::website::{AttachWebsite, Website, WebsiteRouter};

use axum::routing::get;
use tower_http::services::ServeDir;

#[tokio::main]
async fn main() {
    let serve_dir = ServeDir::new("static").append_index_html_on_directories(true);

    let mut root = Website::new("sakanaa :)", "/api");
    root.content = root_page(&mut root).await;

    let router = WebsiteRouter::new()
        .route("/", get(Website::render(&root)))
        .attach_website(&mut root)
        .fallback_service(serve_dir);

    let port = config().server.port;

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port))
        .await
        .unwrap();

    axum::serve(listener, router).await.unwrap();
}
