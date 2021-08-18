use libnewsletter::{config, startup, telemetry};
use sqlx::postgres::PgPoolOptions;
use std::net::TcpListener;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let subscriber = telemetry::get_subscriber("newsletter".into(), "info".into(), std::io::stdout);
    telemetry::init_subscriber(subscriber);

    let config = config::settings().expect("Failed to read configuration");
    let listener = TcpListener::bind(&config.application)?;
    let db_pool = PgPoolOptions::new()
        .connect_timeout(std::time::Duration::from_secs(10))
        .connect_with(config.database.connection_with_db())
        .await
        .expect("Failed to connect to database");

    startup::run(listener, db_pool)?.await
}
