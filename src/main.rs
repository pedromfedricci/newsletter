use newsletter::startup::run;
use std::net::{SocketAddr, TcpListener};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let addr = SocketAddr::from(([127, 0, 0, 1], 8000));
    let listener = TcpListener::bind(&addr)?;

    run(listener)?.await
}
