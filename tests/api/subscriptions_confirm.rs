use fake::faker::internet::en::SafeEmail;
use fake::faker::name::en::Name;
use fake::Fake;
use wiremock::matchers::{method, path};
use wiremock::{Mock, ResponseTemplate};

use crate::helpers::spawn_app;

#[tokio::test]
async fn the_link_returned_by_subscribe_returns_a_200_if_called() {
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
    let response = reqwest::get(confirmation_links.html).await.unwrap();

    assert_eq!(response.status().as_u16(), 200);
}
