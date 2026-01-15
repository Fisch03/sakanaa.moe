use axum::{Router, routing::get};
use tokio::net::TcpListener;
use tower_http::{compression::CompressionLayer, services::ServeDir};

use common::pages;

#[tokio::main]
async fn main() {
    let serve_static = ServeDir::new("dist");

    let app = Router::new()
        .route("/", get(pages::main))
        .fallback_service(serve_static)
        .layer(CompressionLayer::new());
    // .with_state(state);

    let listener = TcpListener::bind("0.0.0.0:8080").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
