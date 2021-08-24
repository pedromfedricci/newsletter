use libnewsletter::{
    config::{self, DatabaseSettings},
    startup::{get_connection_pool, Application},
    telemetry,
};

use once_cell::sync::Lazy;
use reqwest::Url;
use sqlx::{Connection, Executor, PgConnection, PgPool};
use std::net::SocketAddr;
use uuid::Uuid;
use wiremock::MockServer;

static TRACING: Lazy<()> = Lazy::new(|| {
    let default_filter = "info".to_string();
    let name = "test".to_string();

    if std::env::var("TEST_LOG").is_ok() {
        let subscriber = telemetry::get_subscriber(name, default_filter, std::io::stdout);
        telemetry::init_subscriber(subscriber);
    } else {
        let subscriber = telemetry::get_subscriber(name, default_filter, std::io::sink);
        telemetry::init_subscriber(subscriber);
    }
});

pub(crate) struct TestApp {
    pub(crate) addr: SocketAddr,
    pub(crate) db_pool: PgPool,
}

// Helper function to create URL from address and path.
pub(crate) fn url_from(addr: &SocketAddr, path: &str) -> Url {
    Url::parse(&format!("http://{}{}", addr.to_string(), path))
        .expect("Failed to parse URL from address and path")
}

// Runs the server to test the public APIs.
pub(crate) async fn spawn_app() -> TestApp {
    Lazy::force(&TRACING);

    // Launch a mock server to stand in for Postmark's API
    let email_server = MockServer::start().await;

    // Randomise configuration to ensure test isolation
    let config = {
        let mut config = config::settings().expect("Failed to read configuration.");
        // Use a different database for each test case
        config.database.database_name = Uuid::new_v4().to_string();
        // Use a random OS port
        config.application.port = 0;
        // Use the mock server as email API
        config.email_client.base_url = email_server.uri();
        config
    };

    // Create and migrate the database
    configure_database(&config.database).await;

    let db_pool = get_connection_pool(&config.database)
        .await
        .expect("Failed to connect to the database");
    let application = Application::build(config.clone())
        .await
        .expect("Failed to build application");
    let addr = SocketAddr::from(([127, 0, 0, 1], application.port()));

    // Spawn application intance
    tokio::spawn(application.run_until_stopped());

    TestApp { addr, db_pool }
}

async fn configure_database(db_settings: &DatabaseSettings) -> PgPool {
    // Single connection to database.
    let mut conn = PgConnection::connect_with(&db_settings.connection_with_host())
        .await
        .expect("Failed to connect to database host");

    // Create new database.
    conn.execute(&*format!(
        r#"CREATE DATABASE "{}";"#,
        db_settings.database_name
    ))
    .await
    .expect("Failed to create database");

    // Create database connection pool.
    let db_pool = PgPool::connect_with(db_settings.connection_with_db())
        .await
        .expect("Failed to connect to database");

    // Migrate database.
    sqlx::migrate!("./migrations")
        .run(&db_pool)
        .await
        .expect("Failed to migrate the database");

    db_pool
}
