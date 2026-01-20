use axum::{
    Json, Router,
    extract::Path,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
};
use common::gfx::{DitherImage, DitherPattern, DitherSettings, Palette};
use serde::Serialize;
use tower_http::cors::{Any, CorsLayer};

pub mod music;

use super::AppState;

pub async fn v1(state: AppState) -> Router<AppState> {
    let cors = CorsLayer::new().allow_methods(Any).allow_origin(Any);

    Router::new()
        .nest("/music", music::init_api(state).await)
        .route("/health", get("OK"))
        .route("/dither/random/{pattern}/{level}", get(dither_handler))
        .layer(cors)
}

pub async fn dither_handler(Path((pattern, level)): Path<(String, String)>) -> Response {
    let (Ok(pattern), Ok(level)) = (pattern.parse::<usize>(), level.parse::<usize>()) else {
        return (StatusCode::BAD_REQUEST, "Invalid parameters").into_response();
    };

    let Ok(pattern) = DitherPattern::new(pattern) else {
        return (StatusCode::BAD_REQUEST, "Invalid dither pattern size").into_response();
    };

    let settings = DitherSettings::new(pattern, level);
    let palette = Palette::random();

    let img = DitherImage::new(palette, settings);

    #[derive(Debug, Serialize)]
    struct DitherResponse {
        data_url: String,
        palette: Palette,
    }

    let response = DitherResponse {
        data_url: img.to_data_url(),
        palette,
    };

    (StatusCode::OK, Json(response)).into_response()
}
