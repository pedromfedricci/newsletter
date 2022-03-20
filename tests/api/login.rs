use crate::helpers::{assert_is_redirect_to, spawn_app};

#[tokio::test]
async fn an_error_flash_message_is_set_on_failure() {
    let test_app = spawn_app().await;

    let login_body = [("username", "someusername"), ("password", "somepassword")];

    let response = test_app.post_login(&login_body).await;
    assert_is_redirect_to(&response, "/login");

    let html_page = test_app.get_login_html().await;
    assert!(html_page.contains("<p><i>Authentication failed</i></p>"));

    let html_page = test_app.get_login_html().await;
    assert!(!html_page.contains("<p><i>Authentication failed</i></p>"));
}
