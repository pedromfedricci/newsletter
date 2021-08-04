use std::net::{SocketAddr, TcpListener};

// Test the health_check endpoint for requirements:
// * the health check is exposed at /health_check;
// * the health check is behind a GET method;
// * the health check response has no body.
#[actix_rt::test]
async fn test_health_check() {
    let addr = spawn_app();

    let response = reqwest::Client::new()
        .get(&format!("http://{}/health_check", addr.to_string()))
        .send()
        .await
        .expect("Failed to execute request.");

    assert!(response.status().is_success());
    assert_eq!(Some(0), response.content_length());
}

// Runs the server to test the public APIs.
fn spawn_app() -> SocketAddr {
    let mut addr = SocketAddr::from(([127, 0, 0, 1], 0));
    let listener = TcpListener::bind(&addr).expect("Failed to bind to random port");
    let given_port = listener.local_addr().unwrap().port();
    addr.set_port(given_port);

    let server = newsletter::run(listener).expect("Failed to bind address");
    tokio::spawn(server);

    addr
}
