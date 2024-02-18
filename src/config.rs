use lazy_static::lazy_static;

use config::Config;

lazy_static! {
    pub static ref CONFIG: Config = Config::builder()
        .add_source(config::File::with_name("config/default"))
        .add_source(config::File::with_name("config/local").required(false))
        .build()
        .expect("Failed to build config");
}
