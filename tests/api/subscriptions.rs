use fake::faker::internet::en::SafeEmail;
use fake::faker::name::en::Name;
use fake::Fake;
use wiremock::matchers::{method, path};
use wiremock::{Mock, ResponseTemplate};

use crate::helpers::spawn_app;

#[tokio::test]
async fn subscribe_returns_a_400_when_data_is_missing() {
    let test_app = spawn_app().await;

    let name = || Name().fake::<String>();
    let email = || SafeEmail().fake::<String>();
    let test_cases = [[("name", name())], [("email", email())]];
    let errors = ["missing the email", "mising the name"];

    for (invalid_body, error) in test_cases.into_iter().zip(errors) {
        let response = test_app.post_subscriptions(&invalid_body).await;

        assert_eq!(
            400,
            response.status().as_u16(),
            "The API did not fail with 400 Bad Request when the payload was {}.",
            error
        );
    }
}

#[tokio::test]
async fn subscribe_returns_a_400_when_fields_are_present_but_invalid() {
    let test_app = spawn_app().await;

    let name = || Name().fake();
    let email = || SafeEmail().fake();
    let test_cases = [
        [("name", String::new()), ("email", email())],
        [("name", name()), ("email", String::new())],
        [("name", name()), ("email", "definitely-not-an-email".to_string())],
    ];
    let errors = ["empty name", "empty email", "invalid email"];

    for (invalid_body, error) in test_cases.into_iter().zip(errors) {
        let response = test_app.post_subscriptions(&invalid_body).await;

        assert_eq!(
            400,
            response.status().as_u16(),
            "The API did not return a 400 Bad Request when the payload was {}.",
            error
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

    let name: String = Name().fake();
    let email: String = SafeEmail().fake();
    let body = [("name", name), ("email", email)];
    let response = test_app.post_subscriptions(&body).await;

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

    let name: String = Name().fake();
    let email: String = SafeEmail().fake();
    let body = [("name", name), ("email", email)];
    test_app.post_subscriptions(&body).await;

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

    let name: String = Name().fake();
    let email: String = SafeEmail().fake();
    let body = [("name", name), ("email", email)];
    test_app.post_subscriptions(&body).await;

    let email_request = &test_app.email_server.received_requests().await.unwrap()[0];
    let confirmation_links = test_app.get_confirmation_links(&email_request);
    // The two links should be identical
    assert_eq!(confirmation_links.html, confirmation_links.plain_text);
}

#[tokio::test]
async fn subscribe_persists_the_new_subscriber() {
    let test_app = spawn_app().await;

    let name: String = Name().fake();
    let email: String = SafeEmail().fake();
    let body = [("name", name.clone()), ("email", email.clone())];
    test_app.post_subscriptions(&body).await;

    let saved = sqlx::query!("SELECT email, name, status FROM subscriptions")
        .fetch_one(&test_app.db_pool)
        .await
        .expect("Failed to fetch saved subscription.");

    assert_eq!(saved.email, email);
    assert_eq!(saved.name, name);
    assert_eq!(saved.status, "pending_confirmation");
}

#[tokio::test]
async fn subscribe_fails_if_there_is_a_fatal_database_error() {
    let test_app = spawn_app().await;

    sqlx::query!("ALTER TABLE subscriptions DROP COLUMN email")
        .execute(&test_app.db_pool)
        .await
        .unwrap();

    let name: String = Name().fake();
    let email: String = SafeEmail().fake();
    let body = [("name", name), ("email", email)];
    let response = test_app.post_subscriptions(&body).await;

    assert_eq!(response.status().as_u16(), 500);
}
