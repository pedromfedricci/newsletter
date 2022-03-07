use crate::helpers::{spawn_app, url_from};

// Test the health_check endpoint for requirements:
// * the health check is exposed at /health_check;
// * the health check is behind a GET method;
// * the health check response has no body.
#[tokio::test]
async fn test_health_check() {
    let test_app = spawn_app().await;

    let response = reqwest::Client::new()
        .get(url_from(&test_app.addr, "/health_check"))
        .send()
        .await
        .expect("Failed to execute request");

    assert!(response.status().is_success());
    assert_eq!(Some(0), response.content_length());
}
