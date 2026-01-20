use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
};
use base64::prelude::*;
use fast_image_resize::{FilterType, ResizeAlg, ResizeOptions, Resizer, SrcCropping};
use image::{DynamicImage, ImageBuffer};
use jwalk::WalkDir;
use lofty::prelude::*;
use serde::Serialize;
use sqlx::Type;
use tokio::sync::mpsc;

use crate::AppState;

const MUSIC_DIR: &str = "/home/sakanaa/nas/Audio/Music/";

pub async fn init_api(state: AppState) -> Router<AppState> {
    let (tx, mut rx) = mpsc::channel(128);

    jwalk::rayon::spawn(|| scan_music_library(tx));

    tokio::spawn(async move {
        while let Some(track) = rx.recv().await {
            state.find_or_create_track(track).await.unwrap();
        }
    });

    Router::new().route("/album/{id}", get(get_album))
}

#[derive(Serialize)]
struct AlbumResponse {
    title: String,
    id: AlbumId,
    album_artist: Option<ArtistResponse>,
    cover_data: Option<String>,
}

#[derive(Serialize)]
struct ArtistResponse {
    name: String,
    id: ArtistId,
}

pub async fn get_album(State(state): State<AppState>, Path(id): Path<u64>) -> Response {
    let id = (id % (i64::MAX as u64)) as i64;

    let num_total_albums = sqlx::query!("SELECT COUNT(*) as count FROM albums")
        .fetch_one(&state.db)
        .await
        .map(|record| record.count)
        .unwrap_or(0);
    log::info!("Total albums: {}", num_total_albums);

    if num_total_albums == 0 {
        return StatusCode::NOT_FOUND.into_response();
    }

    let n = id % num_total_albums;

    let Ok(record) = sqlx::query!(
        "SELECT album_id, title, artist_id, path FROM albums ORDER BY album_id LIMIT 1 OFFSET ?",
        n
    )
    .fetch_one(&state.db)
    .await
    else {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    };

    let album_artist = if let Some(artist_id) = record.artist_id {
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

    // encode to webp base64 using the image crate
    let path = record.path.clone();
    let (tx, rx) = tokio::sync::oneshot::channel();
    rayon::spawn(move || {
        let res = get_cover_from_path(path).map(|img| {
            let mut resizer = Resizer::new();
            let img = img.to_rgb8();
            let mut dst_image: DynamicImage = DynamicImage::ImageRgb8(ImageBuffer::new(300, 300));
            let mut opts = ResizeOptions::new();
            opts.algorithm = ResizeAlg::Convolution(FilterType::Hamming);
            opts.cropping = SrcCropping::FitIntoDestination((300.0, 300.0));
            resizer.resize(&img, &mut dst_image, Some(&opts)).unwrap();

            let mut webp_data = Vec::new();

            {
                let encoder = webp::Encoder::from_image(&dst_image).unwrap();
                let webp = encoder.encode(50.0);
                webp_data.extend_from_slice(&webp);
            }

            BASE64_STANDARD.encode(&webp_data)
        });
        let _ = tx.send(res);
    });

    let cover = rx.await.unwrap();

    let album_response = AlbumResponse {
        title: record.title,
        id: AlbumId(record.album_id),
        album_artist,
        cover_data: cover,
    };

    Json(album_response).into_response()
}

fn get_cover_from_path(path: String) -> Option<DynamicImage> {
    let dir_path = std::path::Path::new(&path);

    let cover_filenames = ["cover.jpg", "cover.png"];
    for filename in &cover_filenames {
        let cover_path = dir_path.join(filename);
        if !cover_path.exists() {
            continue;
        }

        if let Ok(img) = image::open(cover_path) {
            return Some(img);
        }
    }

    for entry in std::fs::read_dir(dir_path).ok()? {
        let entry = entry.ok()?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }

        let Ok(tagged_file) = lofty::read_from_path(&path) else {
            continue;
        };

        let Some(picture) = tagged_file
            .primary_tag()
            .and_then(|tag| tag.pictures().iter().min_by_key(|p| p.data().len()))
        else {
            continue;
        };

        if let Ok(img) = image::load_from_memory(picture.data()) {
            return Some(img);
        }
    }

    None
}

fn scan_music_library(track_tx: mpsc::Sender<Track>) {
    for entry in WalkDir::new(MUSIC_DIR) {
        let Ok(entry) = entry else {
            continue;
        };

        let path = entry.path();
        if path.is_file() {
            process_file(&path, track_tx.clone());
        }
    }
}

#[derive(Clone, Copy, Debug, Type, Serialize)]
#[sqlx(transparent)]
struct ArtistId(i64);

#[derive(Clone, Debug)]
struct Artist {
    name: String,
    mbid: Option<String>,
}

#[derive(Clone, Copy, Debug, Type, Serialize)]
#[sqlx(transparent)]
struct AlbumId(i64);

#[derive(Clone, Debug)]
struct Album {
    title: String,
    path: String,
    mbid: Option<String>,
    album_artist: Option<Artist>,
}

#[derive(Clone, Copy, Debug, Type, Serialize)]
#[sqlx(transparent)]
struct TrackId(i64);

#[derive(Clone, Debug)]
struct Track {
    title: String,
    path: String,
    mbid: Option<String>,
    album: Option<Album>,
    artist: Option<Artist>,
}

fn process_file(path: &std::path::Path, track_tx: mpsc::Sender<Track>) {
    let tagged_file = lofty::read_from_path(path);
    if let Ok(tagged_file) = tagged_file
        && let Some(tag) = tagged_file.primary_tag()
    {
        let title = tag
            .get_string(&ItemKey::TrackTitle)
            .map(|s| s.to_string())
            .unwrap_or(path.file_name().unwrap().to_string_lossy().to_string());
        let mbid = tag
            .get_string(&ItemKey::MusicBrainzTrackId)
            .map(|s| s.to_string());

        let album_artist = tag.get_string(&ItemKey::AlbumArtist).map(|s| Artist {
            name: s.to_string(),
            mbid: tag
                .get_string(&ItemKey::MusicBrainzReleaseArtistId)
                .map(|s| s.to_string()),
        });

        let artist = tag
            .get_string(&ItemKey::TrackArtist)
            .map(|s| Artist {
                name: s.to_string(),
                mbid: tag
                    .get_string(&ItemKey::MusicBrainzArtistId)
                    .map(|s| s.to_string()),
            })
            .or_else(|| album_artist.clone());

        let album = tag.get_string(&ItemKey::AlbumTitle).map(|s| Album {
            title: s.to_string(),
            path: path
                .parent()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_default(),
            mbid: tag
                .get_string(&ItemKey::MusicBrainzReleaseId)
                .map(|s| s.to_string()),
            album_artist,
        });

        let track = Track {
            title,
            path: path.to_string_lossy().to_string(),
            mbid,
            album,
            artist,
        };

        let _ = track_tx.blocking_send(track);
    }
}

impl AppState {
    async fn find_or_create_track(&self, track: Track) -> sqlx::Result<Option<TrackId>> {
        let album_id = if let Some(album) = track.album {
            self.find_or_create_album(album).await?
        } else {
            None
        };

        let artist_id = if let Some(artist) = track.artist {
            self.find_or_create_artist(artist).await?
        } else {
            None
        };

        if let Some(ref mbid) = track.mbid
            && let Some(record) = sqlx::query!("SELECT track_id FROM tracks WHERE mbid = ?", mbid)
                .fetch_optional(&self.db)
                .await?
        {
            return Ok(Some(TrackId(record.track_id)));
        }

        if let Some(record) = sqlx::query!(
            "SELECT track_id, mbid FROM tracks WHERE title = ? AND album_id = ? AND artist_id = ?",
            track.title,
            album_id,
            artist_id
        )
        .fetch_optional(&self.db)
        .await?
        {
            if let (None, Some(mbid)) = (record.mbid, track.mbid) {
                sqlx::query!(
                    "UPDATE tracks SET mbid = ? WHERE track_id = ?",
                    mbid,
                    record.track_id
                )
                .execute(&self.db)
                .await?;
            }

            return Ok(Some(TrackId(record.track_id)));
        }

        let track_id = self
            .create_track(&track.title, &track.path, track.mbid, album_id, artist_id)
            .await?;

        Ok(Some(track_id))
    }

    async fn create_track(
        &self,
        title: &str,
        path: &str,
        mbid: Option<String>,
        album_id: Option<AlbumId>,
        artist_id: Option<ArtistId>,
    ) -> sqlx::Result<TrackId> {
        let track_id = sqlx::query!(
            "INSERT INTO tracks (title, path, mbid, album_id, artist_id) VALUES (?, ?, ?, ?, ?) RETURNING track_id",
            title,
            path,
            mbid,
            album_id,
            artist_id
        )
        .fetch_one(&self.db)
        .await?
        .track_id;
        Ok(TrackId(track_id))
    }

    async fn find_or_create_album(&self, album: Album) -> sqlx::Result<Option<AlbumId>> {
        if let Some(ref mbid) = album.mbid
            && let Some(record) = sqlx::query!("SELECT album_id FROM albums WHERE mbid = ?", mbid)
                .fetch_optional(&self.db)
                .await?
        {
            return Ok(Some(AlbumId(record.album_id)));
        }

        let album_artist_id = if let Some(ref artist) = album.album_artist {
            self.find_or_create_artist(artist.clone()).await?
        } else {
            None
        };

        if let Some(album_artist_id) = album_artist_id
            && let Some(record) = sqlx::query!(
                "SELECT album_id, mbid FROM albums WHERE title = ? AND artist_id = ?",
                album.title,
                album_artist_id
            )
            .fetch_optional(&self.db)
            .await?
        {
            if let (None, Some(mbid)) = (record.mbid, album.mbid) {
                sqlx::query!(
                    "UPDATE albums SET mbid = ? WHERE album_id = ?",
                    mbid,
                    record.album_id
                )
                .execute(&self.db)
                .await?;
            }

            return Ok(Some(AlbumId(record.album_id)));
        }

        let album_id = self
            .create_album(&album.title, &album.path, None, album_artist_id)
            .await?;
        Ok(Some(album_id))
    }

    async fn create_album(
        &self,
        title: &str,
        path: &str,
        mbid: Option<String>,
        album_artist_id: Option<ArtistId>,
    ) -> sqlx::Result<AlbumId> {
        let album_id = sqlx::query!(
            "INSERT INTO albums (title, path, mbid, artist_id) VALUES (?, ?, ?, ?) RETURNING album_id",
            title,
            path,
            mbid,
            album_artist_id
        )
        .fetch_one(&self.db)
        .await?
        .album_id;

        Ok(AlbumId(album_id))
    }

    async fn find_or_create_artist(&self, artist: Artist) -> sqlx::Result<Option<ArtistId>> {
        if let Some(ref mbid) = artist.mbid
            && let Some(record) = sqlx::query!("SELECT artist_id FROM artists WHERE mbid = ?", mbid)
                .fetch_optional(&self.db)
                .await?
        {
            return Ok(Some(ArtistId(record.artist_id)));
        }

        if let Some(record) = sqlx::query!(
            "SELECT artist_id, mbid FROM artists WHERE name = ?",
            artist.name
        )
        .fetch_optional(&self.db)
        .await?
        {
            if let (None, Some(mbid)) = (record.mbid, artist.mbid) {
                sqlx::query!(
                    "UPDATE artists SET mbid = ? WHERE artist_id = ?",
                    mbid,
                    record.artist_id
                )
                .execute(&self.db)
                .await?;
            }

            return Ok(Some(ArtistId(record.artist_id)));
        }

        let artist_id = self
            .create_artist(&artist.name, artist.mbid.as_deref())
            .await?;
        Ok(Some(artist_id))
    }

    async fn create_artist(&self, name: &str, mbid: Option<&str>) -> sqlx::Result<ArtistId> {
        let artist_id = sqlx::query!(
            "INSERT INTO artists (name, mbid) VALUES (?, ?) RETURNING artist_id",
            name,
            mbid
        )
        .fetch_one(&self.db)
        .await?
        .artist_id;

        Ok(ArtistId(artist_id))
    }
}
