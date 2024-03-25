use std::path::PathBuf;
use anyhow::{Result, bail};
use tokio_rusqlite::params;
use dlhn::{Deserializer, Serializer};
use serde::{Deserialize, Serialize};
use futures::executor;
use axum::async_trait;

use super::super::{ConnectedDB, db};
use super::types::*;
use super::audio_processing;
use super::music_lookup_pipeline::MusicLookupPipeline;

#[async_trait]
pub trait MusicDBExt {
    async fn upsert_track(&self, track: UnprocessedTrack) -> Result<Track>;
}

#[async_trait]
impl MusicDBExt for ConnectedDB { 
    /// upserts a track into the db, and returns it.
    /// this function takes a mininally populated [UnprocessedTrack]. if it already is found in the
    /// db, it is returned. otherwise, the track is first populated through various sources, then
    /// inserted into the db and returned. 
    /// in the second case, the bpm analysis will be started, but run in the background. this means
    /// that there is no guarantee that beat events for the returned track are available.
    async fn upsert_track(&self, track: UnprocessedTrack) -> Result<Track> {
        let start = std::time::Instant::now();

        // check if track is already in db
        let found_track = lookup_track_in_db(&self, &track).await;

        if let Some(track) = found_track {
            println!("Track already in db");
            return Ok(track);
        }

        // track not found in db, try to find more metadata 
        let track = MusicLookupPipeline::new().lookup_track(track).await;

        
        // insert newly found metadata into db
        let track = insert_track_into_db(&self, track).await?;
 
        // start analyzing the track in the background
        if let Some(file) = &track.file {
            let id = track.id;
            let file = file.clone();
            rayon::spawn(move || analyze_and_store_file(file, id));
        }
        

        println!("Track inserted into db, took {:?}", start.elapsed());
        Ok(dbg!(track))
    }
}

/// analyzes the given file.
/// when done, store [BeatData] in the db
/// this is a very cpu intensive task and should be run on a seperate thread
fn analyze_and_store_file(path: PathBuf, track_id: i64) {
    if let Ok(processed) = audio_processing::bpm::analyze_file(path) {
        let mut serialized = Vec::new();
        let mut serializer = Serializer::new(&mut serialized);
        processed.beat_data.serialize(&mut serializer).unwrap();
        executor::block_on(async move {
            db().await.conn.call(move |db| {
                db.execute(
                    "UPDATE tracks SET beatevents = ? WHERE id = ?",
                    params![serialized, track_id]
                )?;
                
                Ok(())
            }).await.unwrap();
        });
    } 
}

/// try to find a track in the db.
/// lookup is done using the [Unprocessed Tracks](UnprocessedTrack) name and artist.
/// furthermore, the lookup tries to be smart about handling multiple artists in the artist name.
async fn lookup_track_in_db(db: &ConnectedDB, track: &UnprocessedTrack) -> Option<Track> {
    let track_name = track.name.clone();
    let artist_name_like = track.artist.as_ref().map(|a| {
        let mut name = a.name.clone();

        // trim of feat. part of the name and make a wildcard search
        name = format!("{}%", &name[..name.to_lowercase().find(" feat.").unwrap_or_else(|| name.len())]);

        name
    });
    
    db.conn.call(move |db| {
        Ok(db.query_row(
            "
                SELECT * FROM tracks 
                LEFT JOIN artists ON tracks.artistId = artists.id
                LEFT JOIN albums ON tracks.albumId = albums.id
                LEFT JOIN artists aartist ON albums.artistId = aartist.id
                WHERE tracks.name = ? COLLATE NOCASE AND artists.name LIKE ? COLLATE NOCASE  
            ", 
            params![track_name, artist_name_like],
            |row| {
                let file = row.get::<_, Option<String>>(6)?;

                let album: Option<Album> = if let Some(album_id) = row.get::<_, Option<i64>>(10)? {
                    Some(Album {
                        id: album_id,
                        mbid: row.get(11)?,
                        name: row.get(12)?,
                        // artistId: row.get(13)?,
                        artist: Artist {
                            id: row.get(14)?,
                            mbid: row.get(15)?,
                            name: row.get(16)?,
                        }
                    })
                } else {
                    None
                };

                let beat_events = row.get::<_, Option<Vec<u8>>>(3)?;

            
                let beat_events = beat_events.map_or_else(|| vec![], |beat_events| { 
                    let mut beat_events = beat_events.as_slice();
                    let mut deserializer = Deserializer::new(&mut beat_events);
                    Vec::<BeatEvent>::deserialize(&mut deserializer).unwrap() 
                }); 

                let file = file.map(|file| PathBuf::from(file));
                let cover = if let Some(file) = &file {
                    audio_processing::metadata::CoverArt::from_file(file).ok()
                } else {
                    None
                };


                Ok(Track {
                    id: row.get(0)?,
                    mbid: row.get(1)?,
                    name: row.get(2)?,
                    beatevents: beat_events,
                    // artistID: row.get(4)?,
                    // albumID: row.get(5)?,
                    file,
                    cover,
                    artist: Artist {
                        id: row.get(7)?,
                        mbid: row.get(8)?,
                        name: row.get(9)?,
                    },
                    album                            
                })
            }
        )?)

    }).await.ok()

}

// insert a [UnprocessedTrack] into the db, turing it into a fully usable [Track] in the process.
// for a track to be inserted, it must at least have an artist.
async fn insert_track_into_db(db: &ConnectedDB, track: UnprocessedTrack) -> Result<Track> {
    if track.artist.is_none() {
        bail!("No artist found for track");
    }

    let track = db.conn.call(move |db| {
            let artist = track.artist.as_ref().unwrap();

            let artist_id = db.query_row(
                "
                 INSERT INTO artists (name, mbid) VALUES (?, ?) 
                 ON CONFLICT(mbid) DO UPDATE SET name = excluded.name 
                 RETURNING id
                ",
                params![artist.name, artist.mbid],
                |row| row.get::<_, i64>(0),
            )?;

            let artist = Artist::new(artist.clone(), artist_id);
            
            let album = track.album.as_ref().map(|album| {
                let album_artist = album.artist.as_ref().map(|album_artist| {
                    db.query_row(
                        "
                         INSERT INTO artists (name, mbid) VALUES (?, ?) 
                         ON CONFLICT(mbid) DO UPDATE SET name = excluded.name 
                         RETURNING id
                        ",
                        params![album_artist.name, album_artist.mbid],
                        |row| row.get::<_, i64>(0),
                    ).ok().map(|album_artist_id| Artist::new(album_artist.clone(), album_artist_id))
                }).flatten().unwrap_or(artist.clone());
 
                db.query_row(
                    "
                     INSERT INTO albums (name, mbid, artistId) VALUES (?, ?, ?) 
                     ON CONFLICT(mbid) DO UPDATE SET name = excluded.name 
                     RETURNING id
                    ",
                    params![album.name, album.mbid, artist_id],
                    |row| row.get::<_, i64>(0),
                ).ok().map(|album_id| Album::new(album.clone(), album_id, album_artist))
            }).flatten();

            let file = track.file.as_ref().map(|file| file.to_string_lossy().to_string());
            let track_id = db.query_row(
                "INSERT INTO tracks (name, mbid, artistId, albumId, file) VALUES (?, ?, ?, ?, ?) ON CONFLICT(mbid) DO UPDATE SET name = excluded.name RETURNING id",
                params![track.name, track.mbid, artist.id, album.as_ref().map(|album| album.id), file],
                |row| row.get::<_, i64>(0),
            )?;

            Ok(Track::new(track, track_id, artist, album))
        }).await?;

    Ok(track)
}

