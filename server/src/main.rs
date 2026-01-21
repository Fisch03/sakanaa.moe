use axum::Router;
use sqlx::SqlitePool;
use tokio::net::TcpListener;
use tower_http::{compression::CompressionLayer, services::ServeDir};

mod api;

mod wasm;
use wasm::WasmService;

#[derive(Clone)]
struct AppState {
    db: SqlitePool,
}

impl AppState {
    pub async fn new() -> anyhow::Result<Self> {
        let db = SqlitePool::connect("sqlite://db/server.db?mode=rwc").await?;
        sqlx::migrate!("../migrations").run(&db).await?;

        Ok(Self { db })
    }
}

#[tokio::main]
async fn main() {
    // colog::default_builder()
    //     .filter_level(log::LevelFilter::Info)
    //     .format(|f, r| colog::format::Compact::default().format(f, r))
    //     .init();

    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .filter_module("lofty", log::LevelFilter::Error)
        .init();

    let state = AppState::new().await.unwrap();

    let serve_static = ServeDir::new("dist");
    let wasm_service = WasmService::new("dist/site/site_bg.wasm");

    let app = Router::new()
        .nest("/api/v1/", api::v1(state.clone()).await)
        .merge(wasm_service.router())
        .fallback_service(serve_static)
        .layer(CompressionLayer::new())
        .with_state(state);

    let listener = TcpListener::bind("0.0.0.0:8080").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
