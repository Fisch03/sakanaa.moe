pub mod types;
pub use types::*;

pub mod music_db_ext;
pub use music_db_ext::MusicDBExt;

pub mod audio_processing;
mod music_lookup_pipeline;
use music_lookup_pipeline::LookupPipelineConfig;

use serde::Deserialize;
#[derive(Debug, Deserialize)]
pub struct MusicDBConfig {
    #[serde(flatten)]
    pipeline: LookupPipelineConfig,
}
