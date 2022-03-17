use std::convert::{TryFrom, TryInto};
use std::net::{IpAddr, SocketAddr, ToSocketAddrs};

use config::{Config, ConfigError};
use secrecy::{ExposeSecret, Secret};
use serde_aux::field_attributes::deserialize_number_from_string;
use sqlx::postgres::{PgConnectOptions, PgSslMode};

use crate::domain::{SubscriberEmail, SubscriberEmailParseError};
use crate::email_client::EmailClient;

#[derive(Debug, Clone, serde::Deserialize)]
pub struct Settings {
    pub database: DatabaseSettings,
    pub application: ApplicationSettings,
    pub email_client: EmailClientSettings,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct ApplicationSettings {
    pub ip: IpAddr,
    pub port: u16,
    pub base_url: String,
}

impl ToSocketAddrs for ApplicationSettings {
    type Iter = std::array::IntoIter<SocketAddr, 1>;

    fn to_socket_addrs(&self) -> std::io::Result<Self::Iter> {
        let socket = SocketAddr::from((self.ip, self.port));
        Ok([socket; 1].into_iter())
    }
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct EmailClientSettings {
    base_url: String,
    sender_email: String,
    authorization_token: Secret<String>,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    timeout_milliseconds: u64,
}

impl EmailClientSettings {
    pub fn sender(&self) -> Result<SubscriberEmail, SubscriberEmailParseError> {
        SubscriberEmail::parse(self.sender_email.clone())
    }

    pub fn timeout(&self) -> std::time::Duration {
        std::time::Duration::from_millis(self.timeout_milliseconds)
    }

    pub fn authorization_token(self) -> Secret<String> {
        self.authorization_token
    }

    pub fn base_url(&self) -> Result<url::Url, url::ParseError> {
        url::Url::parse(&self.base_url)
    }

    pub fn set_base_url(&mut self, uri: String) {
        self.base_url = uri
    }
}

impl From<EmailClientSettings> for EmailClient {
    fn from(settings: EmailClientSettings) -> EmailClient {
        let sender = settings.sender().expect("valid sender email address from configutation file");
        let base_url = settings.base_url().expect("valid base_url from configuration file");
        let timeout = settings.timeout();
        let authorization_token = settings.authorization_token();

        EmailClient::new(base_url, sender, authorization_token, timeout)
    }
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct DatabaseSettings {
    pub database_name: String,
    pub username: String,
    pub password: Secret<String>,
    pub host: String,
    pub port: u16,
    pub require_ssl: bool,
}

pub fn settings() -> Result<Settings, ConfigError> {
    let mut settings = Config::default();
    let base_path = std::env::current_dir().expect("Could not determine the current directory");
    let config_dir = base_path.join("config");

    // Read the default configuration file
    settings.merge(config::File::from(config_dir.join("base")).required(true))?;

    // Detect the running environment.
    // Default to `local` if unspecified.
    let env: Environment = std::env::var("APP_ENVIRONMENT")
        .unwrap_or_else(|_| "local".into())
        .try_into()
        .expect("Could not parse APP_ENVIRONMENT environment variable");

    // Layer on the environment-specific values.
    settings.merge(config::File::from(config_dir.join(env.as_str())).required(true))?;

    // Add in settings from environment variables (with a prefix of APP and '__' as separator)
    // E.g. `APP_APPLICATION__PORT=5001 would set `Settings.application.port`
    settings.merge(config::Environment::with_prefix("app").separator("__"))?;

    settings.try_into()
}

impl DatabaseSettings {
    pub fn connection_with_host(&self) -> PgConnectOptions {
        let ssl_mode = match self.require_ssl {
            true => PgSslMode::Require,
            false => PgSslMode::Prefer,
        };

        PgConnectOptions::new()
            .host(&self.host)
            .username(&self.username)
            .password(self.password.expose_secret())
            .port(self.port)
            .ssl_mode(ssl_mode)
    }

    pub fn connection_with_db(&self) -> PgConnectOptions {
        self.connection_with_host().database(&self.database_name)
    }
}

pub(crate) enum Environment {
    Local,
    CI,
    DevContainer,
    Production,
}

impl Environment {
    pub fn as_str(&self) -> &'static str {
        match self {
            Environment::Local => "local",
            Environment::DevContainer => "devcontainer",
            Environment::CI => "ci",
            Environment::Production => "production",
        }
    }
}

impl TryFrom<String> for Environment {
    type Error = String;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        match s.to_lowercase().as_str() {
            "local" => Ok(Self::Local),
            "devcontainer" => Ok(Self::DevContainer),
            "ci" => Ok(Self::CI),
            "production" => Ok(Self::Production),
            other => Err(format!(
                "{} is not a supported environment. Use either `local`, `ci`, `devcontainer` or `production`.",
                other
            )),
        }
    }
}
