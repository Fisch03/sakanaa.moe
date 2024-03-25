use anyhow::{Context, Result};
use image::DynamicImage;
use std::path::Path;
use symphonia::core::{
    formats::{FormatOptions, FormatReader},
    io::MediaSourceStream,
    meta::*,
    probe::{Hint, ProbeResult},
};

fn probe_file<P: AsRef<Path>>(path: P) -> Result<ProbeResult> {
    let file = std::fs::File::open(path)?;
    let stream = MediaSourceStream::new(Box::new(file), Default::default());

    let hint: Hint = Default::default();
    let meta_opts: MetadataOptions = Default::default();
    let fmt_opts: FormatOptions = Default::default();

    Ok(symphonia::default::get_probe().format(&hint, stream, &fmt_opts, &meta_opts)?)
}

trait ProbeResultExt {
    fn map_metadata<T, F>(&mut self, callback: F) -> Option<T>
    where
        F: FnOnce(&MetadataRevision) -> T;
}
impl ProbeResultExt for ProbeResult {
    fn map_metadata<T, F>(&mut self, callback: F) -> Option<T>
    where
        F: FnOnce(&MetadataRevision) -> T,
    {
        if let Some(mut metadata) = self.metadata.get() {
            if let Some(revision) = metadata.skip_to_latest() {
                Some(callback(&revision))
            } else {
                None
            }
        } else if let Some(metadata) = self.format.metadata().current() {
            Some(callback(metadata))
        } else {
            None
        }
    }
}

#[derive(Debug, Default)]
pub struct Metadata {
    pub track: Option<String>,
    pub mb_track: Option<String>,

    pub artist: Option<String>,
    pub mb_artist: Option<String>,

    pub album_artist: Option<String>,
    pub mb_album_artist: Option<String>,

    pub album: Option<String>,
    pub mb_album: Option<String>,
}

impl From<&MetadataRevision> for Metadata {
    fn from(revision: &MetadataRevision) -> Self {
        let mut metadata = Self::default();

        revision.tags().iter().for_each(|tag| {
            if let Some(key) = tag.std_key {
                let value = match &tag.value {
                    Value::String(value) => value.to_string(),
                    _ => return,
                };

                match key {
                    StandardTagKey::TrackTitle => metadata.track = Some(value),
                    StandardTagKey::MusicBrainzTrackId => metadata.mb_track = Some(value),

                    StandardTagKey::Artist => metadata.artist = Some(value),
                    StandardTagKey::MusicBrainzArtistId => metadata.mb_artist = Some(value),

                    StandardTagKey::AlbumArtist => metadata.album_artist = Some(value),
                    StandardTagKey::MusicBrainzAlbumArtistId => {
                        metadata.mb_album_artist = Some(value)
                    }

                    StandardTagKey::Album => metadata.album = Some(value),
                    StandardTagKey::MusicBrainzAlbumId => metadata.mb_album = Some(value),
                    _ => {}
                }
            }
        });

        metadata
    }
}

impl Metadata {
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let found_metadata = probe_file(path)?.map_metadata(|metadata| Metadata::from(metadata));

        Ok(found_metadata.context("No metadata found")?)
    }
}

#[derive(Clone)]
pub struct CoverArt(pub DynamicImage);
impl TryFrom<Visual> for CoverArt {
    type Error = image::ImageError;

    fn try_from(visual: Visual) -> Result<Self, Self::Error> {
        let format = image::ImageFormat::from_mime_type(visual.media_type);

        let image;
        if let Some(format) = format {
            image = image::load_from_memory_with_format(&visual.data, format);
        } else {
            image = image::load_from_memory(&visual.data);
        }

        Ok(Self(image?))
    }
}

// i really dont want a fuckton of bytes printed everytime i debug print anything that uses a cover
impl std::fmt::Debug for CoverArt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "CoverArt {{ ... }}")
    }
}

impl CoverArt {
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let cover = probe_file(path)?
            .map_metadata(|metadata| {
                let visuals = metadata.visuals();

                let mut chosen_visual = visuals.iter().find(|visual| match visual.usage {
                    Some(StandardVisualKey::FrontCover) => true,
                    _ => false,
                });

                if chosen_visual.is_none() {
                    chosen_visual = visuals.get(0);

                    if chosen_visual.is_none() {
                        return None;
                    }
                }

                let visual = chosen_visual.unwrap();
                CoverArt::try_from(visual.clone()).ok()
            })
            .flatten();

        Ok(cover.context("No cover art found")?)
    }
}
