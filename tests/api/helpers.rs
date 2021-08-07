use libnewsletter::{
    config::{self, DatabaseSettings},
    startup,
};
use reqwest::Url;
use sqlx::{Connection, Executor, PgConnection, PgPool};
use std::net::{SocketAddr, TcpListener};
use uuid::Uuid;

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
    let mut addr = SocketAddr::from(([127, 0, 0, 1], 0));
    let listener = TcpListener::bind(&addr).expect("Failed to bind to random port");
    let given_port = listener.local_addr().unwrap().port();
    addr.set_port(given_port);

    let mut config = config::settings().expect("Failed to read configuration");
    config.database.database_name = Uuid::new_v4().to_string();
    let db_pool = configure_database(&config.database).await;

    let server = startup::run(listener, db_pool.clone()).expect("Failed to bind address");
    tokio::spawn(server);

    TestApp { addr, db_pool }
}

async fn configure_database(config: &DatabaseSettings) -> PgPool {
    // Single connection to database.
    let mut conn = PgConnection::connect(&config.connection_string_without_db_name())
        .await
        .expect("Failed to connect to database");

    // Create new database.
    conn.execute(&*format!(r#"CREATE DATABASE "{}";"#, config.database_name))
        .await
        .expect("Failed to create database");

    // Create database connection pool.
    let db_pool = PgPool::connect(&config.connection_string())
        .await
        .expect("Failed to connect to database");

    // Migrate database.
    sqlx::migrate!("./migrations")
        .run(&db_pool)
        .await
        .expect("Failed to migrate the database");

    db_pool
}
