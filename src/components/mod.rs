mod filtered_image;
pub use filtered_image::*;

mod section;
pub use section::*;
pub mod sections;

mod colorfilter;
pub use colorfilter::*;

mod big_waifu;
pub use big_waifu::*;

mod zerox20;
pub use zerox20::*;

use sections::AboutMeConfig;
use sections::LiveActivityConfig;
use serde::Deserialize;
#[derive(Debug, Deserialize)]
pub struct PageConfig {
    about_me: AboutMeConfig,
    live_activity: LiveActivityConfig,
}
