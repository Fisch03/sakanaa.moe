use serde::Serialize;
use sqlx::Type;
use std::path::{Path, PathBuf};
use tokio::sync::mpsc;
use walkdir::WalkDir;
use lofty::prelude::*;

use crate::AppState;
use super::images::generate_cover;

const MUSIC_DIR: &str = "/home/sakanaa/nas/Audio/Music/";

#[derive(Clone, Copy, Debug, Type, Serialize)]
#[sqlx(transparent)]
pub struct ArtistId(pub i64);

#[derive(Clone, Debug)]
pub struct Artist {
    pub name: String,
    pub mbid: Option<String>,
}

#[derive(Clone, Copy, Debug, Type, Serialize)]
#[sqlx(transparent)]
pub struct AlbumId(pub i64);

#[derive(Clone, Debug)]
pub struct Album {
    pub title: String,
    pub path: String,
    pub has_embedded_cover: bool,
    pub mbid: Option<String>,
    pub album_artist: Option<Artist>,
}

#[derive(Clone, Copy, Debug, Type, Serialize)]
#[sqlx(transparent)]
pub struct TrackId(pub i64);

#[derive(Clone, Debug)]
pub struct Track {
    pub title: String,
    pub path: String,
    pub has_embedded_cover: bool,
    pub mbid: Option<String>,
    pub album: Option<Album>,
    pub artist: Option<Artist>,
}

pub fn scan_music_library(track_tx: mpsc::Sender<Track>) {
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
            log::info!("scanned {} files...", scanned_files);
        }
    }

    log::info!("finished scanning music library. total files scanned: {}", scanned_files);
}

fn process_file(path: &Path, track_tx: mpsc::Sender<Track>) {
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

// DB Extension methods
impl AppState {
    pub async fn find_or_create_track(&self, track: Track) -> sqlx::Result<Option<TrackId>> {
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
