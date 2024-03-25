use sakanaa_web::config::config;
use sakanaa_web::root_page;
use sakanaa_web::website::{AttachWebsite, Website, WebsiteRouter};

use tower_http::compression::CompressionLayer;
use tower_http::services::ServeDir;

#[tokio::main]
async fn main() {
    let serve_dir = ServeDir::new("static").append_index_html_on_directories(true);

    let mut root = Website::new("sakanaa :)", "/api");
    let content = root_page(&mut root).await;
    root.set_content(content);

    let compression = CompressionLayer::new()
        .gzip(true)
        .zstd(true)
        .br(true)
        .deflate(true);

    let router = WebsiteRouter::new()
        .attach_website(root)
        .fallback_service(serve_dir)
        .layer(compression);

    let port = config().server.port;

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port))
        .await
        .unwrap();

    axum::serve(listener, router).await.unwrap();
}
