use libnewsletter::{configuration, startup};
use std::net::TcpListener;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let config = configuration::settings().expect("Failed to read configuration");
    let addr = config.app_addr;
    let listener = TcpListener::bind(&addr)?;

    startup::run(listener)?.await
}
