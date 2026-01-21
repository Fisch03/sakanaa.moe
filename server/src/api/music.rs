use std::path::PathBuf;

use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
};
use fast_image_resize::{FilterType, ResizeAlg, ResizeOptions, Resizer, SrcCropping};
use image::{DynamicImage, ImageBuffer};
use lofty::prelude::*;
use serde::Serialize;
use sqlx::Type;
use tokio::sync::mpsc;
use walkdir::WalkDir;

use crate::AppState;

const MUSIC_DIR: &str = "/home/sakanaa/nas/Audio/Music/";
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

fn scan_music_library(track_tx: mpsc::Sender<Track>) {
    let mut scanned_files = 0;

    for entry in WalkDir::new(MUSIC_DIR) {
        let Ok(entry) = entry else {
            continue;
        };
        let path = entry.path();

        if path.is_file() {

            let track_tx = track_tx.clone();
            let path = path.to_path_buf();
            process_file(&path, track_tx.clone());

            scanned_files += 1;
        }

        if scanned_files % 250 == 0 {
            log::info!("Scanned {} files...", scanned_files);
        }
    }

    log::info!("Finished scanning music library. Total files scanned: {}", scanned_files);
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
    has_embedded_cover: bool,
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
    has_embedded_cover: bool,
    mbid: Option<String>,
    album: Option<Album>,
    artist: Option<Artist>,
}

fn process_file(path: &std::path::Path, track_tx: mpsc::Sender<Track>) {
    let tagged_file = lofty::read_from_path(path);
    if let Ok(tagged_file) = tagged_file
        && let Some(tag) = tagged_file.primary_tag()
    {
        let has_embedded_cover = !tag.pictures().is_empty();

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
            has_embedded_cover,
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
            has_embedded_cover,
            path: path.to_string_lossy().to_string(),
            mbid,
            album,
            artist,
        };

        let _ = track_tx.blocking_send(track);
    }
}

fn get_embedded_cover(path: &std::path::Path) -> Option<DynamicImage> {
    let tagged_file = lofty::read_from_path(path).ok()?;
    let tag = tagged_file.primary_tag()?;
    let picture = tag.pictures().first()?;
    image::load_from_memory(picture.data()).ok()
}

fn get_image_from_file(path: &std::path::Path) -> Option<DynamicImage> {
    image::open(path).ok()
}

fn resize_and_save_cover(cover_id: i64, img: DynamicImage) {
    let img = img.to_rgb8();
    let mut resized_cover = DynamicImage::ImageRgb8(ImageBuffer::new(300, 300));
    let mut resizer = Resizer::new();
    let mut opts = ResizeOptions::new();
    opts.algorithm = ResizeAlg::Convolution(FilterType::Hamming);
    opts.cropping = SrcCropping::FitIntoDestination((300.0, 300.0));
    let _ = resizer.resize(&img, &mut resized_cover, Some(&opts));

    if let Ok(encoder) = webp::Encoder::from_image(&resized_cover) {
        let cover = encoder.encode(50.0);
        let cover_path = PathBuf::from(COVER_DIR).join(format!("{}.webp", cover_id));
        let _ = std::fs::create_dir_all(COVER_DIR);
        let _ = std::fs::write(cover_path, &*cover);
    }
}

fn generate_cover(cover_id: i64, source_path: PathBuf, is_embedded: bool) {
    log::info!(
        "Generating cover {} from {} (embedded: {})",
        cover_id,
        source_path.display(),
        is_embedded
    );

    let img = if is_embedded {
        get_embedded_cover(&source_path)
    } else {
        get_image_from_file(&source_path)
    };

    if let Some(img) = img {
        resize_and_save_cover(cover_id, img);
    }
}

impl AppState {
    async fn find_or_create_track(&self, track: Track) -> sqlx::Result<Option<TrackId>> {
        let album_id = if let Some(ref album) = track.album {
            self.find_or_create_album(album.clone(), &track.path)
                .await?
        } else {
            None
        };

        let artist_id = if let Some(ref artist) = track.artist {
            self.find_or_create_artist(artist.clone()).await?
        } else {
            None
        };

        if let Some(ref mbid) = track.mbid
            && let Some(record) =
                sqlx::query!("SELECT track_id, cover_id FROM tracks WHERE mbid = ?", mbid)
                    .fetch_optional(&self.db)
                    .await?
        {
            if record.cover_id.is_none()
                && let Some(cover_id) = self.resolve_track_cover(&track).await?
            {
                sqlx::query!(
                    "UPDATE tracks SET cover_id = ? WHERE track_id = ?",
                    cover_id,
                    record.track_id
                )
                .execute(&self.db)
                .await?;
            }

            return Ok(Some(TrackId(record.track_id)));
        }

        if let Some(record) = sqlx::query!(
            "SELECT track_id, mbid, cover_id FROM tracks WHERE title = ? AND album_id = ? AND artist_id = ?",
            track.title,
            album_id,
            artist_id
        )
        .fetch_optional(&self.db)
        .await?
        {
            if record.mbid.is_none() && track.mbid.is_some() {
                sqlx::query!(
                    "UPDATE tracks SET mbid = ? WHERE track_id = ?",
                    track.mbid,
                    record.track_id
                )
                .execute(&self.db)
                .await?;
            }

            if record.cover_id.is_none() 
                && let Some(cover_id) = self.resolve_track_cover(&track).await? {
                     sqlx::query!(
                        "UPDATE tracks SET cover_id = ? WHERE track_id = ?",
                        cover_id,
                        record.track_id
                    )
                    .execute(&self.db)
                    .await?;
                }
            

            return Ok(Some(TrackId(record.track_id)));
        }

        let cover_id = self.resolve_track_cover(&track).await?;

        let track_id = self
            .create_track(
                &track.title,
                &track.path,
                cover_id,
                track.mbid,
                album_id,
                artist_id,
            )
            .await?;

        Ok(Some(track_id))
    }

    async fn resolve_track_cover(&self, track: &Track) -> sqlx::Result<Option<i64>> {
        let mut source = None;
        if track.has_embedded_cover {
            source = Some((PathBuf::from(&track.path), true));
        } else {
            let path = PathBuf::from(&track.path);
            if let Some(parent) = path.parent() {
                for name in ["cover.jpg", "cover.png"] {
                    let p = parent.join(name);
                    if p.exists() {
                        source = Some((p, false));
                        break;
                    }
                }
            }
        }

        if let Some((source, is_embedded)) = source {
            let source_str = source.to_string_lossy().to_string();
            let cover_id = sqlx::query!(
                "INSERT INTO covers (source_path) VALUES (?) RETURNING cover_id",
                source_str
            )
            .fetch_one(&self.db)
            .await?
            .cover_id;

            rayon::spawn(move || generate_cover(cover_id, source, is_embedded));
            Ok(Some(cover_id))
        } else {
            Ok(None)
        }
    }

    async fn create_track(
        &self,
        title: &str,
        path: &str,
        cover_id: Option<i64>,
        mbid: Option<String>,
        album_id: Option<AlbumId>,
        artist_id: Option<ArtistId>,
    ) -> sqlx::Result<TrackId> {
        let track_id = sqlx::query!(
            "INSERT INTO tracks (title, path, cover_id, mbid, album_id, artist_id) VALUES (?, ?, ?, ?, ?, ?) RETURNING track_id",
            title,
            path,
            cover_id,
            mbid,
            album_id,
            artist_id
        )
        .fetch_one(&self.db)
        .await?
        .track_id;
        Ok(TrackId(track_id))
    }

    async fn find_or_create_album(
        &self,
        album: Album,
        track_path: &str,
    ) -> sqlx::Result<Option<AlbumId>> {
        if let Some(ref mbid) = album.mbid
            && let Some(record) =
                sqlx::query!("SELECT album_id, cover_id FROM albums WHERE mbid = ?", mbid)
                    .fetch_optional(&self.db)
                    .await?
        {
            if record.cover_id.is_none() 
                && let Some(cover_id) = self.resolve_album_cover(&album, track_path).await? {
                    sqlx::query!(
                        "UPDATE albums SET cover_id = ? WHERE album_id = ?",
                        cover_id,
                        record.album_id
                    )
                    .execute(&self.db)
                    .await?;
                
            }
            return Ok(Some(AlbumId(record.album_id)));
        }

        let album_artist_id = if let Some(ref artist) = album.album_artist {
            self.find_or_create_artist(artist.clone()).await?
        } else {
            None
        };

        if let Some(album_artist_id) = album_artist_id
            && let Some(record) = sqlx::query!(
                "SELECT album_id, mbid, cover_id FROM albums WHERE title = ? AND artist_id = ?",
                album.title,
                album_artist_id
            )
            .fetch_optional(&self.db)
            .await?
        {
            if record.mbid.is_none() && album.mbid.is_some() {
                sqlx::query!(
                    "UPDATE albums SET mbid = ? WHERE album_id = ?",
                    album.mbid,
                    record.album_id
                )
                .execute(&self.db)
                .await?;
            }

            if record.cover_id.is_none() 
                && let Some(cover_id) = self.resolve_album_cover(&album, track_path).await? {
                    sqlx::query!(
                        "UPDATE albums SET cover_id = ? WHERE album_id = ?",
                        cover_id,
                        record.album_id
                    )
                    .execute(&self.db)
                    .await?;
                }
            

            return Ok(Some(AlbumId(record.album_id)));
        }

        let cover_id = self.resolve_album_cover(&album, track_path).await?;

        let album_id = self
            .create_album(
                &album.title,
                &album.path,
                cover_id,
                album.mbid.as_deref(),
                album_artist_id,
            )
            .await?;
        Ok(Some(album_id))
    }

    async fn resolve_album_cover(
        &self,
        album: &Album,
        track_path: &str,
    ) -> sqlx::Result<Option<i64>> {
        let mut source = None;

        // Priority: Folder > Embedded
        let folder_path = PathBuf::from(&album.path); // album.path is the directory
        for name in ["cover.jpg", "cover.png"] {
            let p = folder_path.join(name);
            if p.exists() {
                source = Some((p, false));
                break;
            }
        }

        if source.is_none() && album.has_embedded_cover {
            source = Some((PathBuf::from(track_path), true));
        }

        if let Some((source, is_embedded)) = source {
            let source_str = source.to_string_lossy().to_string();
            let cover_id = sqlx::query!(
                "INSERT INTO covers (source_path) VALUES (?) RETURNING cover_id",
                source_str
            )
            .fetch_one(&self.db)
            .await?
            .cover_id;

            rayon::spawn(move || generate_cover(cover_id, source, is_embedded));
            Ok(Some(cover_id))
        } else {
            Ok(None)
        }
    }

    async fn create_album(
        &self,
        title: &str,
        path: &str,
        cover_id: Option<i64>,
        mbid: Option<&str>,
        album_artist_id: Option<ArtistId>,
    ) -> sqlx::Result<AlbumId> {
        let album_id = sqlx::query!(
            "INSERT INTO albums (title, path, cover_id, mbid, artist_id) VALUES (?, ?, ?, ?, ?) RETURNING album_id",
            title,
            path,
            cover_id,
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
