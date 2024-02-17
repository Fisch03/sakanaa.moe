use lazy_static::lazy_static;

use config::Config;

lazy_static! {
    static ref CONFIG: Config = Config::builder()
        .add_source(config::File::with_name("config/default"))
        .add_source(config::File::with_name("config/local").required(false))
        .build()
        .expect("Failed to build config");
}

#[allow(dead_code)]
pub fn get<'de, T: serde::de::Deserialize<'de>>(key: &str) -> Option<T> {
    CONFIG.get(key).ok()
}
