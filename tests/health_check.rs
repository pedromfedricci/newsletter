use reqwest::Url;
use std::net::{SocketAddr, TcpListener};

// Helper function to create URL from address and path.
fn url_from(addr: &SocketAddr, path: &str) -> Url {
    let protocol = "http://";
    Url::parse(&format!("{}{}{}", protocol, addr.to_string(), path))
        .expect("Failed to parse URL from address and path")
}

// Runs the server to test the public APIs.
fn spawn_app() -> SocketAddr {
    let mut addr = SocketAddr::from(([127, 0, 0, 1], 0));
    let listener = TcpListener::bind(&addr).expect("Failed to bind to random port");
    let given_port = listener.local_addr().unwrap().port();
    addr.set_port(given_port);

    let server = libnewsletter::startup::run(listener).expect("Failed to bind address");
    tokio::spawn(server);

    addr
}

// Test the health_check endpoint for requirements:
// * the health check is exposed at /health_check;
// * the health check is behind a GET method;
// * the health check response has no body.
#[actix_rt::test]
async fn test_health_check() {
    let addr = spawn_app();

    let response = reqwest::Client::new()
        .get(url_from(&addr, "/health_check"))
        .send()
        .await
        .expect("Failed to execute request");

    assert!(response.status().is_success());
    assert_eq!(Some(0), response.content_length());
}

//
#[actix_rt::test]
async fn test_subscribe_valid_data() {
    let addr = spawn_app();
    let client = reqwest::Client::new();

    let test_body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    let response = client
        .post(url_from(&addr, "/subscriptions"))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(test_body)
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(200, response.status().as_u16());
}

#[actix_rt::test]
async fn test_subscribe_missing_data() {
    let addr = spawn_app();
    let client = reqwest::Client::new();

    let test_cases = vec![
        ("name=le%20guin", "Missing the email."),
        ("email=ursula_le_guin%40gmail.com", "Missing the name."),
        ("", "Missing both name and email."),
    ];

    for (invalid_body, error_message) in test_cases {
        let response = client
            .post(url_from(&addr, "/subscriptions"))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(invalid_body)
            .send()
            .await
            .expect("Failed to execute request");

        assert_eq!(
            400,
            response.status().as_u16(),
            "The API did not fail with 400 Bad Request when the payload was {}.",
            error_message
        );
    }
}
