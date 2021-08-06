use std::net::SocketAddr;

use config::{Config, ConfigError, File};

#[derive(serde::Deserialize)]
pub struct Settings {
    pub database: DatabaseSettings,
    pub app_addr: SocketAddr,
}

#[derive(serde::Deserialize)]
pub struct DatabaseSettings {
    pub database_name: String,
    pub username: String,
    pub password: String,
    pub host: String,
    pub port: u16,
}

pub fn settings() -> Result<Settings, ConfigError> {
    let mut settings = Config::default();
    settings.merge(File::with_name("config"))?;
    settings.try_into()
}
