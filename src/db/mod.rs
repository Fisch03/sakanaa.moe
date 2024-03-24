mod connected_db;
pub use connected_db::{db, ConnectedDB};

pub mod music;

use music::MusicDBConfig;
use serde::Deserialize;
use std::path::PathBuf;
#[derive(Debug, Deserialize)]
pub struct DBConfig {
    path: PathBuf,
    music: MusicDBConfig,
}
