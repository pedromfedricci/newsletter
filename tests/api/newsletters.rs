use serde_json::Value;
use wiremock::{
    matchers::{any, method, path},
    Mock, ResponseTemplate,
};

use crate::helpers::{spawn_app, url_from, ConfirmationLinks, TestApp};

#[actix_rt::test]
async fn newsletters_are_not_sent_to_unconfirmed_subscribers() {
    let test_app = spawn_app().await;

    create_unconfirmed_subscriber(&test_app).await;

    Mock::given(any())
        .respond_with(ResponseTemplate::new(200))
        .expect(0)
        .mount(&test_app.email_server)
        .await;

    let response = test_app.post_newsletters(&newsletter_test_body()).await;

    assert_eq!(response.status().as_u16(), 200);
    // Assert no request were sent to MockServer on shutdown.
}

#[actix_rt::test]
async fn newsletter_are_delivered_to_confirmed_subscribers() {
    let test_app = spawn_app().await;

    create_confirmed_subscriber(&test_app).await;

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&test_app.email_server)
        .await;

    let response = test_app.post_newsletters(&newsletter_test_body()).await;

    assert_eq!(response.status().as_u16(), 200);
    // Assert that one request was sent to MockServer on shutdown.
}

#[actix_rt::test]
async fn newsletters_return_400_for_invalid_data() {
    let test_app = spawn_app().await;

    let test_cases = vec![
        (newsletter_test_body_content(), "missing title field"),
        (newsletter_test_body_title(), "missing content field"),
    ];

    for (invalid_body, msg) in test_cases {
        let response = test_app.post_newsletters(&invalid_body).await;

        assert_eq!(
            response.status().as_u16(),
            400,
            "The newsletters API did not fail with 400 Bad Request when payload was {}",
            msg
        )
    }
}

async fn create_unconfirmed_subscriber(test_app: &TestApp) -> ConfirmationLinks {
    let test_body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    let _mock_guard = Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .named("Create unconfirmed subscriber")
        .expect(1)
        .mount_as_scoped(&test_app.email_server)
        .await;

    test_app
        .post_subscriptions(test_body.into())
        .await
        .error_for_status()
        .unwrap();

    let email_request = &test_app
        .email_server
        .received_requests()
        .await
        .unwrap()
        .pop()
        .unwrap();

    test_app.get_confirmation_links(&email_request)
}

async fn create_confirmed_subscriber(test_app: &TestApp) {
    let confirmation_links = create_unconfirmed_subscriber(test_app).await;

    reqwest::get(confirmation_links.html)
        .await
        .unwrap()
        .error_for_status()
        .unwrap();
}

#[actix_rt::test]
async fn request_missing_authorization_is_rejected() {
    let test_app = spawn_app().await;

    let response = reqwest::Client::new()
        .post(url_from(&test_app.addr, "/newsletters"))
        .json(&newsletter_test_body())
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(401, response.status().as_u16());
    assert_eq!(
        r#"Basic realm="publish""#,
        response.headers()["WWW-Authenticate"]
    );
}

fn newsletter_test_body() -> Value {
    serde_json::json!({
        "title": newsletter_test_body_title(),
        "content": newsletter_test_body_content(),
    })
}

fn newsletter_test_body_title() -> Value {
    serde_json::json!("Newsletter title")
}

fn newsletter_test_body_content() -> Value {
    serde_json::json!({
        "text": "Newsletter body as plain text",
        "html": "<p>Newsletter body as HTML<p>",
    })
}
