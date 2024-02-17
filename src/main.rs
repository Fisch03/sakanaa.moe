mod api;
mod config;

use axum::Router;
use tower_http::services::ServeDir;

#[tokio::main]
async fn main() {
    let serve_dir = ServeDir::new("public").append_index_html_on_directories(true);

    let api = api::API::new();

    let app = Router::new()
        .nest("/api", api.router)
        .fallback_service(serve_dir);

    let port = config::get::<u16>("server.port").unwrap_or_else(|| {
        eprintln!("Failed to read server.port from config, falling back to port 3001");
        3001
    });

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port))
        .await
        .unwrap();

    axum::serve(listener, app).await.unwrap();
}
