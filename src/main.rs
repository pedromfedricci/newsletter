use libnewsletter::{config, startup};
use sqlx::PgPool;
use std::net::TcpListener;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let config = config::settings().expect("Failed to read configuration");
    let listener = TcpListener::bind(&config.app_addr)?;
    let db_pool = PgPool::connect_with(config.database.connection_with_db())
        .await
        .expect("Failed to connect to database");

    startup::run(listener, db_pool)?.await
}
