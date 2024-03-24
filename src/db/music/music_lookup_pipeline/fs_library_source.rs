use super::super::audio_processing::get_metadata;
use super::super::types::{UnprocessedAlbum, UnprocessedArtist, UnprocessedTrack};
use super::MusicDataSource;
use crate::config::config;

use anyhow::{Context, Result};
use axum::async_trait;
use futures::future;
use serde::Deserialize;
use std::path::PathBuf;
use tokio::fs::{self, DirEntry};

#[derive(Debug, Deserialize)]
pub struct MusicLibConfig {
    path: PathBuf,
    path_replacements: Vec<PathReplaceFilter>,
    #[serde(with = "serde_regex")]
    file_regex: regex::Regex,
}

#[derive(Debug, Deserialize, Clone)]
pub struct PathReplaceFilter {
    #[serde(with = "serde_regex")]
    from: regex::Regex,
    to: String,
}
impl PathReplaceFilter {
    fn apply(&self, path: &str) -> String {
        self.from.replace_all(path, &self.to).to_string()
    }
}

pub struct FsLibrarySource {}

impl FsLibrarySource {
    pub fn new() -> Self {
        Self {}
    }

    fn apply_replacements(&self, path: &str) -> String {
        let path_replacements = &config().db.music.pipeline.fs_library.path_replacements;

        path_replacements
            .iter()
            .fold(path.to_string(), |path, filter| filter.apply(&path))
    }

    async fn find_within_dir<F>(
        &self,
        dir: &PathBuf,
        find_files: bool,
        filter: F,
    ) -> Result<Vec<DirEntry>>
    where
        F: Fn(&str) -> bool,
    {
        let mut read_dir = fs::read_dir(&dir)
            .await
            .context("Failed to read source path")?;

        let mut entries = Vec::new();

        while let Some(entry) = read_dir.next_entry().await? {
            let file_type = entry.file_type().await?;

            if file_type.is_dir() && find_files {
                continue;
            }
            if file_type.is_file() && !find_files {
                continue;
            }

            let entry_name = entry.file_name();
            let entry_name = entry_name.to_string_lossy();
            let entry_name = entry_name.to_lowercase();
            if filter(&entry_name) {
                entries.push(entry);
            }
        }

        Ok(entries)
    }
}

#[async_trait]
impl MusicDataSource for FsLibrarySource {
    async fn lookup_track(
        &self,
        mut track: UnprocessedTrack,
        replace: bool,
    ) -> Result<UnprocessedTrack> {
        let source_path = &config().db.music.pipeline.fs_library.path;
        let filename_regex = &config().db.music.pipeline.fs_library.file_regex;

        let name = self.apply_replacements(&track.name);

        let path;
        if track.file.is_some() {
            // if the path is already known, just use it
            path = track.file.as_ref().unwrap().clone();
        } else {
            // else go through the (quite expensive) process of finding the track
            let name_lower = name.to_lowercase();

            let artist = self.apply_replacements(&track.artist.as_ref().context("No artist")?.name);
            let artist_lower = artist.to_lowercase();

            let dir = PathBuf::from(source_path);

            let artist_dirs = self
                .find_within_dir(&dir, false, |entry_name| match entry_name.as_ref() {
                    "various artists" => true,
                    _ if entry_name == artist_lower => true,

                    // try to avoid cases like "ARTIST feat SOMEONEELSE"
                    _ if artist_lower.starts_with(&entry_name) => true,
                    _ if entry_name.starts_with(&artist_lower) => true,

                    _ => false,
                })
                .await?;

            let album_dirs = if let Some(ref album) = track.album {
                let album = self.apply_replacements(&album.name);
                let album_lower = album.to_lowercase();

                let tasks = artist_dirs.iter().map(|artist_dir| async {
                    self.find_within_dir(&artist_dir.path(), false, |entry_name| {
                        entry_name == album_lower
                    })
                    .await
                });

                let tasks = future::join_all(tasks).await;

                tasks.into_iter().collect::<Result<Vec<_>>>()?
            } else {
                let tasks = artist_dirs.iter().map(|artist_dir| async {
                    self.find_within_dir(&artist_dir.path(), false, |_| true)
                        .await
                });

                let tasks = future::join_all(tasks).await;

                tasks.into_iter().collect::<Result<Vec<_>>>()?
            };

            let tasks = album_dirs.iter().flatten().map(|album_dir| async {
                self.find_within_dir(&album_dir.path(), true, |entry_name| {
                    // regex is slow as fuck so this cheaper test first
                    // in my very limited testing this more than doubles the speed
                    if !entry_name.contains(&name_lower) {
                        return false;
                    }

                    filename_regex.captures(entry_name).map_or_else(
                        || false,
                        |captures| {
                            if let Some(entry_name) =
                                captures.get(1).map(|m| m.as_str().to_lowercase())
                            {
                                entry_name == name_lower
                            } else {
                                false
                            }
                        },
                    )
                })
                .await
            });

            let tasks = future::join_all(tasks).await;

            let found_track = tasks
                .into_iter()
                .flatten()
                .next()
                .context("No track found")?
                .into_iter()
                .next()
                .context("No track found")?;

            path = found_track.path();
            track.file = Some(path.clone());
        }

        // actually get the metadata and merge it into the track
        if let Ok(metadata) = get_metadata(path) {
            let artist = metadata.artist.map(|name| UnprocessedArtist {
                mbid: metadata.mb_artist,
                name,
            });

            let album = metadata.album.map(|name| UnprocessedAlbum {
                mbid: metadata.mb_album,
                name,
                artist: metadata.album_artist.map(|name| UnprocessedArtist {
                    mbid: metadata.mb_album_artist,
                    name,
                }),
            });

            let metadata_track = UnprocessedTrack {
                name: metadata.track.unwrap_or(name),
                mbid: metadata.mb_track,

                album,
                artist,

                beatevents: None,
                file: None,
            };

            if replace {
                track = metadata_track.merge(track);
            } else {
                track = track.merge(metadata_track);
            }
        }

        Ok(track)
    }
}
