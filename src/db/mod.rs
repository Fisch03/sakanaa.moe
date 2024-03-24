use crate::config::config;
use anyhow::Result;
use tokio::sync::OnceCell;
use tokio_rusqlite::{params, Connection};

pub async fn db() -> &'static ConnectedDB {
    static DB: OnceCell<ConnectedDB> = OnceCell::const_new();
    DB.get_or_init(|| async { ConnectedDB::new().await.unwrap() })
        .await
}

pub mod music;

pub struct ConnectedDB {
    conn: Connection,
}

impl ConnectedDB {
    async fn new() -> Result<Self> {
        let db_path = config().get::<String>("files.db_path")?;

        let conn = Connection::open(db_path).await?;

        conn.call(|db| {
            db.execute(
                "CREATE TABLE IF NOT EXISTS artists (
                    id   INTEGER PRIMARY KEY,
                    mbid TEXT UNIQUE,

                    name TEXT NOT NULL
                )",
                params![],
            )?;
            db.execute(
                "CREATE TABLE IF NOT EXISTS albums (
                    id       INTEGER PRIMARY KEY,
                    mbid     TEXT UNIQUE,

                    name     TEXT NOT NULL, 

                    artistId INTEGER NOT NULL,

                    UNIQUE (name, artistId),
                    FOREIGN  KEY (artistId) REFERENCES artists(id)
                )",
                params![],
            )?;
            db.execute(
                "CREATE TABLE IF NOT EXISTS tracks (
                    id       INTEGER PRIMARY KEY,
                    mbid     TEXT UNIQUE,

                    name       TEXT NOT NULL,
                    beatevents BLOB,
                    
                    artistId INTEGER NOT NULL, 
                    albumId  INTEGER,

                    file     TEXT,

                    UNIQUE (name, artistId, albumId),
                    FOREIGN KEY (artistId) REFERENCES artists(id)
                    FOREIGN KEY (albumId)  REFERENCES albums(id)
                )",
                params![],
            )?;

            Ok(())
        })
        .await?;

        Ok(Self { conn })
    }
}
