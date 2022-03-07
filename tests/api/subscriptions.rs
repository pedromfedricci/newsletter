use wiremock::{
    matchers::{method, path},
    Mock, ResponseTemplate,
};

use crate::helpers::spawn_app;

#[tokio::test]
async fn subscribe_returns_a_400_when_data_is_missing() {
    let test_app = spawn_app().await;

    let test_cases = vec![
        ("name=le%20guin", "Missing the email."),
        ("email=ursula_le_guin%40gmail.com", "Missing the name."),
        ("", "Missing both name and email."),
    ];

    for (invalid_body, error_message) in test_cases {
        let response = test_app.post_subscriptions(invalid_body.into()).await;

        assert_eq!(
            400,
            response.status().as_u16(),
            "The API did not fail with 400 Bad Request when the payload was {}.",
            error_message
        );
    }
}

#[tokio::test]
async fn subscribe_returns_a_400_when_fields_are_present_but_invalid() {
    let test_app = spawn_app().await;

    let test_cases = vec![
        ("name=&email=ursula_le_guin%40gmail.com", "empty name"),
        ("name=Ursula&email=", "empty email"),
        ("name=Ursula&email=definitely-not-an-email", "invalid email"),
    ];

    for (invalid_body, description) in test_cases {
        let response = test_app.post_subscriptions(invalid_body.into()).await;

        assert_eq!(
            400,
            response.status().as_u16(),
            "The API did not return a 400 Bad Request when the payload was {}.",
            description
        );
    }
}

#[tokio::test]
async fn subscribe_returns_a_200_for_valid_form_data() {
    let test_app = spawn_app().await;

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&test_app.email_server)
        .await;

    let test_body = "name=le%20guin&email=ursula_le_guin%40gmail.com";
    let response = test_app.post_subscriptions(test_body.into()).await;

    assert_eq!(200, response.status().as_u16());
}

#[tokio::test]
async fn subscribe_sends_a_confirmation_email_for_valid_data() {
    let test_app = spawn_app().await;

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&test_app.email_server)
        .await;

    let test_body = "name=le%20guin&email=ursula_le_guin%40gmail.com";
    test_app.post_subscriptions(test_body.into()).await;
    // Assert on MockServer drop
}

#[tokio::test]
async fn subscribe_sends_a_confirmation_email_with_a_link() {
    let test_app = spawn_app().await;

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&test_app.email_server)
        .await;

    let test_body = "name=le%20guin&email=ursula_le_guin%40gmail.com";
    test_app.post_subscriptions(test_body.into()).await;

    let email_request = &test_app.email_server.received_requests().await.unwrap()[0];
    let confirmation_links = test_app.get_confirmation_links(&email_request);
    // The two links should be identical
    assert_eq!(confirmation_links.html, confirmation_links.plain_text);
}

#[tokio::test]
async fn subscribe_persists_the_new_subscriber() {
    let test_app = spawn_app().await;

    let test_status = "pending_confirmation";
    let test_name = "le guin";
    let test_email = "ursula_le_guin@gmail.com";
    let test_body =
        format!("name={}&email={}", test_name.replace(" ", "%20"), test_email.replace("@", "%40"));
    test_app.post_subscriptions(test_body.into()).await;

    let saved = sqlx::query!("SELECT email, name, status FROM subscriptions")
        .fetch_one(&test_app.db_pool)
        .await
        .expect("Failed to fetch saved subscription.");

    assert_eq!(saved.email, test_email);
    assert_eq!(saved.name, test_name);
    assert_eq!(saved.status, test_status);
}

#[tokio::test]
async fn subscribe_fails_if_there_is_a_fatal_database_error() {
    let test_app = spawn_app().await;

    sqlx::query!("ALTER TABLE subscriptions DROP COLUMN email")
        .execute(&test_app.db_pool)
        .await
        .unwrap();

    let test_body = "name=le%20guin&email=ursula_le_guin%40gmail.com";
    let response = test_app.post_subscriptions(test_body.into()).await;

    assert_eq!(response.status().as_u16(), 500);
}
