use axum::Router;
use tokio::net::TcpListener;
use tower_http::{compression::CompressionLayer, services::ServeDir};

mod api;
mod wasm;

use wasm::WasmService;

#[tokio::main]
async fn main() {
    colog::init();

    let serve_static = ServeDir::new("dist");
    let wasm_service = WasmService::new("dist/site/site_bg.wasm");

    let app = Router::new()
        .nest("/api/v1/", api::v1())
        .merge(wasm_service.router())
        .fallback_service(serve_static)
        .layer(CompressionLayer::new());
    // .with_state(state);

    let listener = TcpListener::bind("0.0.0.0:8080").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
