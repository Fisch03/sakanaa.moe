use config::Config;
use serde::Deserialize;
use std::sync::OnceLock;

use crate::api::ApiConfig;
use crate::components::PageConfig;
use crate::db::DBConfig;

#[derive(Debug, Deserialize)]
pub struct ConfigRoot {
    pub server: ServerConfig,
    pub db: DBConfig,
    pub page: PageConfig,
    pub api: ApiConfig,
}

#[derive(Debug, Deserialize)]
pub struct ServerConfig {
    pub port: u16,

    user_agent: String,
    contact: String,

    #[serde(skip)]
    client: Option<reqwest::Client>,
}

pub fn config() -> &'static ConfigRoot {
    static CONFIG: OnceLock<ConfigRoot> = OnceLock::new();
    CONFIG.get_or_init(|| {
        let config = Config::builder()
            .add_source(config::File::with_name("config/default"))
            .add_source(config::File::with_name("config/local").required(false))
            .build()
            .expect("Failed to build config");

        let mut config: ConfigRoot = config
            .try_deserialize()
            .expect("Failed to deserialize config");

        config.server.build_client();

        config
    })
}

impl ServerConfig {
    pub fn client(&self) -> &reqwest::Client {
        self.client.as_ref().expect("Client not built")
    }

    fn build_client(&mut self) {
        let user_agent = &self
            .user_agent
            .replace("{version}", env!("CARGO_PKG_VERSION"))
            .replace("{contact}", &self.contact);

        self.client = Some(
            reqwest::Client::builder()
                .user_agent(user_agent)
                .build()
                .expect("Failed to build reqwest client"),
        );
        self.user_agent = user_agent.clone();
    }
}
