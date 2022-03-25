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
    pub redis_uri: Secret<String>,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct ApplicationSettings {
    pub ip: IpAddr,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub port: u16,
    pub base_url: String,
    pub hmac_secret: Secret<String>,
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

    pub fn authorization_token(&self) -> Secret<String> {
        self.authorization_token.clone()
    }

    pub fn base_url(&self) -> Result<url::Url, url::ParseError> {
        url::Url::parse(&self.base_url)
    }

    pub fn set_base_url(&mut self, uri: String) {
        self.base_url = uri
    }

    pub fn client(self) -> EmailClient {
        let sender_email = self.sender().expect("invalid sender email address");
        let base_url = self.base_url().expect("invalid base_url");
        EmailClient::new(base_url, sender_email, self.authorization_token(), self.timeout())
    }
}

impl From<EmailClientSettings> for EmailClient {
    fn from(settings: EmailClientSettings) -> EmailClient {
        let sender =
            settings.sender().expect("expected valid sender email address from configutation file");
        let base_url =
            settings.base_url().expect("expected valid base_url from configuration file");
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
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub port: u16,
    pub require_ssl: bool,
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

pub fn settings() -> Result<Settings, ConfigError> {
    let mut settings = Config::default();
    let base_path = std::env::current_dir().expect("Could not determine the current directory");
    let config_dir = base_path.join("config");

    // Read the default configuration file
    settings.merge(config::File::from(config_dir.join("base")).required(true))?;

    // Detect the running environment.
    // Default to `local` if unspecified.
    let env = if let Ok(env_str) = std::env::var(Environment::ENV_VAR_NAME) {
        env_str.as_str().try_into().unwrap()
    } else {
        Environment::Local
    };

    // Layer on the environment-specific values.
    settings.merge(config::File::from(config_dir.join(env.as_ref())).required(true))?;

    // Add in settings from environment variables (with a prefix of APP and '__' as separator)
    // E.g. `APP_APPLICATION__PORT=5001 would set `Settings.application.port`
    settings.merge(config::Environment::with_prefix("app").separator("__"))?;

    settings.try_into()
}

#[derive(Debug)]
enum Environment {
    Local,
    Ci,
    DevContainer,
    Production,
}

impl Environment {
    const ENV_VAR_NAME: &'static str = "APP_ENVIRONMENT";

    const LOCAL: &'static str = "local";
    const DEVCONTAINER: &'static str = "devcontainer";
    const CI: &'static str = "ci";
    const PRODUCTION: &'static str = "production";
}

impl TryFrom<&str> for Environment {
    type Error = EnvironmentParseError;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        match s.to_lowercase().as_str() {
            Self::LOCAL => Ok(Self::Local),
            Self::DEVCONTAINER => Ok(Self::DevContainer),
            Self::CI => Ok(Self::Ci),
            Self::PRODUCTION => Ok(Self::Production),
            other => Err(EnvironmentParseError { other: other.to_string() }),
        }
    }
}

impl AsRef<str> for Environment {
    fn as_ref(&self) -> &str {
        match self {
            Environment::Local => Self::LOCAL,
            Environment::DevContainer => Self::DEVCONTAINER,
            Environment::Ci => Self::CI,
            Environment::Production => Self::PRODUCTION,
        }
    }
}

#[derive(Debug)]
struct EnvironmentParseError {
    other: String,
}

impl std::fmt::Display for EnvironmentParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "could not parse {env_var_name} environment variable, \
            `{other}` is not supported environment, \
            use either `{local}`, `{dev}`, `{ci}` or `{prod}`",
            env_var_name = Environment::ENV_VAR_NAME,
            other = self.other,
            local = Environment::LOCAL,
            dev = Environment::DEVCONTAINER,
            ci = Environment::CI,
            prod = Environment::PRODUCTION
        )
    }
}

impl std::error::Error for EnvironmentParseError {}
