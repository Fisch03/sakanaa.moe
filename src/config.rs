use lazy_static::lazy_static;

use config::Config;

lazy_static! {
    pub static ref CONFIG: Config = Config::builder()
        .add_source(config::File::with_name("config/default"))
        .add_source(config::File::with_name("config/local").required(false))
        .build()
        .expect("Failed to build config");

    // Global client for making requests that already has the correct user agent
    pub static ref CLIENT: reqwest::Client = {
        let user_agent = CONFIG
        .get::<String>("server.user_agent")
        .expect("Missing server.user_agent in config")
        .replace("{version}", env!("CARGO_PKG_VERSION"));

        dbg!(&user_agent);

        reqwest::Client::builder()
            .user_agent(&user_agent)
            .build()
            .expect("Failed to build reqwest client")
    };
}
