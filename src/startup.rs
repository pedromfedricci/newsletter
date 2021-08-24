use actix_web::{dev::Server, web, App, HttpServer};
use reqwest::Url;
use sqlx::{postgres::PgPoolOptions, PgPool};
use std::net::TcpListener;
use tracing_actix_web::TracingLogger;

use crate::{
    config::{DatabaseSettings, Settings},
    email_client::EmailClient,
    routes::{health_check::health_check, subscriptions::subscribe},
};

#[derive(Debug)]
pub struct Application {
    port: u16,
    server: Server,
}

impl Application {
    pub async fn build(config: Settings) -> Result<Self, std::io::Error> {
        let connection_pool = get_connection_pool(&config.database)
            .await
            .expect("Failed to connect to Postgres.");

        let sender_email = config
            .email_client
            .sender()
            .expect("Invalid sender email address.");

        let email_client = EmailClient::new(
            Url::parse(&config.email_client.base_url).unwrap(),
            sender_email,
            config.email_client.authorization_token,
        );

        let listener = TcpListener::bind(config.application)?;
        let port = listener.local_addr().unwrap().port();

        let server = run(listener, email_client, connection_pool)?;

        Ok(Self { port, server })
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub async fn run_until_stopped(self) -> Result<(), std::io::Error> {
        self.server.await
    }
}

pub fn run(
    listener: TcpListener,
    email_client: EmailClient,
    db_pool: PgPool,
) -> std::io::Result<Server> {
    let db_pool = web::Data::new(db_pool);
    let email_client = web::Data::new(email_client);

    let server = HttpServer::new(move || {
        App::new()
            .wrap(TracingLogger::default())
            .app_data(db_pool.clone())
            .app_data(email_client.clone())
            .route("/health_check", web::get().to(health_check))
            .route("/subscriptions", web::post().to(subscribe))
    })
    .listen(listener)?
    .run();

    Ok(server)
}

pub async fn get_connection_pool(database: &DatabaseSettings) -> Result<PgPool, sqlx::Error> {
    PgPoolOptions::new()
        .connect_timeout(std::time::Duration::from_secs(2))
        .connect_with(database.connection_with_db())
        .await
}
