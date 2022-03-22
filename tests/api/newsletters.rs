use fake::faker::internet::en::SafeEmail;
use fake::faker::name::en::Name;
use fake::Fake;
use wiremock::matchers::{any, method, path};
use wiremock::{Mock, ResponseTemplate};

use crate::helpers::{assert_is_redirect_to, spawn_app, ConfirmationLinks, TestApp};

async fn create_unconfirmed_subscriber(test_app: &TestApp) -> ConfirmationLinks {
    let name: String = Name().fake();

    let _mock_guard = Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .named("Create unconfirmed subscriber")
        .expect(1)
        .mount_as_scoped(&test_app.email_server)
        .await;

    let email: String = SafeEmail().fake();
    let body = [("name", name), ("email", email)];
    test_app.post_subscriptions(&body).await.error_for_status().unwrap();

    let email_request = &test_app.email_server.received_requests().await.unwrap().pop().unwrap();
    test_app.get_confirmation_links(email_request)
}

async fn create_confirmed_subscriber(test_app: &TestApp) {
    let confirmation_link = create_unconfirmed_subscriber(test_app).await.html;
    reqwest::get(confirmation_link).await.unwrap().error_for_status().unwrap();
}

#[tokio::test]
async fn newsletters_are_not_delivered_to_unconfirmed_subscribers() {
    let test_app = spawn_app().await;
    create_unconfirmed_subscriber(&test_app).await;
    test_app.login_test_user().await;

    Mock::given(any())
        .respond_with(ResponseTemplate::new(200))
        .expect(0)
        .mount(&test_app.email_server)
        .await;

    // Submit newsletter form
    let newsletter_request_body = serde_json::json!({
        "title": "Newsletter title",
        "text_content": "Newsletter body as plain text",
        "html_content": "<p>Newsletter body as HTML</p>",
    });

    let response = test_app.post_publish_newsletter(&newsletter_request_body).await;
    assert_is_redirect_to(&response, "/admin/newsletters");

    // Follow the redirect
    let html_page = test_app.get_publish_newsletter_html().await;
    assert!(html_page.contains("<p><i>The newsletter issue has been published!</i></p>"));
    // Mock verifies on Drop that we haven't sent the newsletter email
}

#[tokio::test]
async fn newsletters_are_delivered_to_confirmed_subscribers() {
    let test_app = spawn_app().await;
    create_confirmed_subscriber(&test_app).await;
    test_app.login_test_user().await;

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&test_app.email_server)
        .await;

    // Submit newsletter form
    let newsletter_request_body = [
        ("title", "Newsletter title"),
        ("text_content", "Newsletter body as plain text"),
        ("html_content", "<p>Newsletter body as HTML</p>"),
    ];
    let response = test_app.post_publish_newsletter(&newsletter_request_body).await;
    assert_is_redirect_to(&response, "/admin/newsletters");

    // Follow the redirect
    let html_page = test_app.get_publish_newsletter_html().await;
    assert!(html_page.contains("<p><i>The newsletter issue has been published!</i></p>"));
    // Mock verifies on Drop that we have sent the newsletter email
}

#[tokio::test]
async fn you_must_be_logged_in_to_see_the_newsletter_form() {
    let test_app = spawn_app().await;

    let response = test_app.get_publish_newsletter().await;
    assert_is_redirect_to(&response, "/login");
}

#[tokio::test]
async fn you_must_be_logged_in_to_publish_a_newsletter() {
    let test_app = spawn_app().await;
    let newsletter_request_body = [
        ("title", "Newsletter title"),
        ("text_content", "Newsletter body as plain text"),
        ("html_content", "<p>Newsletter body as HTML</p>"),
    ];

    let response = test_app.post_publish_newsletter(&newsletter_request_body).await;
    assert_is_redirect_to(&response, "/login");
}
