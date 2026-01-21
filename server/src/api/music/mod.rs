use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use serde::Serialize;
use std::path::PathBuf;
use tokio::sync::mpsc;

use crate::AppState;

mod images;
mod library;

pub use library::{AlbumId, ArtistId};

use library::scan_music_library;

const COVER_DIR: &str = "db/covers/";

pub async fn init_api(state: AppState) -> Router<AppState> {
    let (tx, mut rx) = mpsc::channel(128);

    rayon::spawn(|| scan_music_library(tx));

    tokio::spawn(async move {
        while let Some(track) = rx.recv().await {
            state.find_or_create_track(track).await.unwrap();
        }
    });

    Router::new()
        .route("/album/random/{count}", get(get_random_albums))
        .route("/cover/{id}", get(get_cover))
}

#[derive(Serialize)]
struct AlbumResponse {
    title: String,
    id: AlbumId,
    album_artist: Option<ArtistResponse>,
    cover_id: Option<i64>,
}

#[derive(Serialize)]
struct ArtistResponse {
    name: String,
    id: ArtistId,
}

pub async fn get_random_albums(State(state): State<AppState>, Path(count): Path<i64>) -> Response {
    let Ok(albums) = sqlx::query!(
        r#"SELECT album_id, title, artist_id, cover_id FROM albums ORDER BY RANDOM() LIMIT ?"#,
        count
    )
    .fetch_all(&state.db)
    .await
    else {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    };

    let mut responses = Vec::new();
    responses.reserve_exact(albums.len());
    for album in albums {
        let album_artist = if let Some(artist_id) = album.artist_id {
            let Ok(artist_record) = sqlx::query!(
                "SELECT artist_id, name FROM artists WHERE artist_id = ?",
                artist_id
            )
            .fetch_one(&state.db)
            .await
            else {
                return StatusCode::INTERNAL_SERVER_ERROR.into_response();
            };
            Some(ArtistResponse {
                name: artist_record.name,
                id: ArtistId(artist_record.artist_id),
            })
        } else {
            None
        };

        let album_response = AlbumResponse {
            title: album.title,
            id: AlbumId(album.album_id),
            album_artist,
            cover_id: album.cover_id,
        };

        responses.push(album_response);
    }

    Json(responses).into_response()
}

async fn get_cover(Path(id): Path<i64>) -> Response {
    let cover_path = PathBuf::from(COVER_DIR).join(format!("{}.webp", id));
    if let Ok(cover_data) = std::fs::read(cover_path) {
        Response::builder()
            .header("Content-Type", "image/webp")
            .body(cover_data.into())
            .unwrap()
    } else {
        StatusCode::NOT_FOUND.into_response()
    }
}
