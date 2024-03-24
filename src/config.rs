use std::sync::OnceLock;

use config::Config;

pub fn config() -> &'static Config {
    static CONFIG: OnceLock<Config> = OnceLock::new();
    CONFIG.get_or_init(|| {
        Config::builder()
            .add_source(config::File::with_name("config/default"))
            .add_source(config::File::with_name("config/local").required(false))
            .build()
            .expect("Failed to build config")
    })
}

pub fn client() -> &'static reqwest::Client {
    // Global client for making requests that already has the correct user agent
    pub static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        let user_agent = config()
            .get::<String>("server.user_agent")
            .expect("Missing server.user_agent in config")
            .replace("{version}", env!("CARGO_PKG_VERSION"))
            .replace(
                "{contact}",
                &config()
                    .get::<String>("server.contact")
                    .expect("Missing server.contact in config"),
            );

        reqwest::Client::builder()
            .user_agent(&user_agent)
            .build()
            .expect("Failed to build reqwest client")
    })
}
