use libnewsletter::{
    config::{self, DatabaseSettings},
    startup, telemetry,
};
use once_cell::sync::Lazy;
use reqwest::Url;
use sqlx::{Connection, Executor, PgConnection, PgPool};
use std::net::{SocketAddr, TcpListener};
use uuid::Uuid;

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

    let (addr, listener) = {
        let mut addr = SocketAddr::from(([127, 0, 0, 1], 0));
        let listener = TcpListener::bind(&addr).expect("Failed to bind to random port");
        // set addr port to OS's random given port
        addr.set_port(listener.local_addr().unwrap().port());
        (addr, listener)
    };

    let db_settings = {
        let mut config = config::settings().expect("Failed to read configuration");
        config.database.database_name = Uuid::new_v4().to_string();
        config.database
    };
    let db_pool = configure_database(&db_settings).await;

    let server = startup::run(listener, db_pool.clone()).expect("Failed to bind address");
    tokio::spawn(server);

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
