use config::{Config, ConfigError, File};
use sqlx::postgres::PgConnectOptions;
use std::net::SocketAddr;

#[derive(Debug, serde::Deserialize)]
pub struct Settings {
    pub database: DatabaseSettings,
    pub app_addr: SocketAddr,
}

#[derive(Debug, serde::Deserialize)]
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

impl DatabaseSettings {
    pub fn connection_with_host(&self) -> PgConnectOptions {
        PgConnectOptions::new()
            .host(&self.host)
            .username(&self.username)
            .password(&self.password)
            .port(self.port)
    }

    pub fn connection_with_db(&self) -> PgConnectOptions {
        self.connection_with_host().database(&self.database_name)
    }
}
