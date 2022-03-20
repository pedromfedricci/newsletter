use uuid::Uuid;

use crate::helpers::{assert_is_redirect_to, spawn_app};

#[tokio::test]
async fn must_be_logged_in_to_see_change_password_form() {
    let test_app = spawn_app().await;

    let response = test_app.get_change_password().await;
    // Must fail to get password form and must redirect to /login.
    assert_is_redirect_to(&response, "/login");
}

#[tokio::test]
async fn must_be_logged_in_to_change_password() {
    let test_app = spawn_app().await;

    let current_password = Uuid::new_v4().to_string();
    let new_password = Uuid::new_v4().to_string();
    let password_form = [
        ("current_password", &current_password),
        ("new_password", &new_password),
        ("new_password_check", &new_password),
    ];

    let response = test_app.post_change_password(&password_form).await;
    // Must fail to change password and must redirect to /login.
    assert_is_redirect_to(&response, "/login");
}

#[tokio::test]
async fn new_password_fields_must_match() {
    let test_app = spawn_app().await;
    // Login as test user.
    test_app.login_test_user().await;

    let new_password = Uuid::new_v4().to_string();
    let different_new_password = Uuid::new_v4().to_string();
    let password_form = [
        ("current_password", &test_app.user.password),
        ("new_password", &new_password),
        ("new_password_check", &different_new_password),
    ];

    // Try change the password for test user.
    let response = test_app.post_change_password(&password_form).await;
    // Must not change password if the two provided ones do not match.
    // Must redirect to /admin/password.
    assert_is_redirect_to(&response, "/admin/password");

    let html_page = test_app.get_change_password_html().await;
    // Must return a error message informing the user
    // that two different passwords were provided.
    assert!(html_page.contains(
        "<p><i>You entered two different new passwords - \
    the field values must match.</i></p>"
    ));
}

#[tokio::test]
async fn current_password_must_be_valid() {
    let test_app = spawn_app().await;
    // Login as test user.
    test_app.login_test_user().await;

    let new_password = Uuid::new_v4().to_string();
    let wrong_password = Uuid::new_v4().to_string();
    let password_form = [
        ("current_password", &wrong_password),
        ("new_password", &new_password),
        ("new_password_check", &new_password),
    ];

    // Try change the password for test user with incorrect current password.
    let response = test_app.post_change_password(&password_form).await;
    // Must not change password if current password is not valid.
    assert_is_redirect_to(&response, "/admin/password");

    let html_page = test_app.get_change_password_html().await;
    // Must return an error message informing the user
    // that current password is not correct.
    assert!(html_page.contains("<p><i>The current password is incorrect.</i></p>"));
}

#[tokio::test]
async fn changing_password_works() {
    let test_app = spawn_app().await;
    // Login as test user.

    let response = test_app.login_test_user().await;
    // Assert login with test user.
    assert_is_redirect_to(&response, "/admin/dashboard");

    let new_password = Uuid::new_v4().to_string();
    let password_form = [
        ("current_password", &test_app.user.password),
        ("new_password", &new_password),
        ("new_password_check", &new_password),
    ];

    // Change the password for test user with valid current password.
    let response = test_app.post_change_password(&password_form).await;
    // Assert post was successful and redirected to /admin/password.
    assert_is_redirect_to(&response, "/admin/password");

    let html_page = test_app.get_change_password_html().await;
    // Must inform user that password change was successful.
    assert!(html_page.contains("<p><i>Your password has been changed.</i></p>"));

    // Logout, must redirect to /login.
    let response = test_app.post_logout().await;
    assert_is_redirect_to(&response, "/login");

    // Inform user that they recently logout.
    let html_page = test_app.get_login_html().await;
    assert!(html_page.contains("<p><i>You have successfully logged out.</i></p>"));

    // Post login with new password for test user.
    let login_form = [("username", &test_app.user.username), ("password", &new_password)];
    let response = test_app.post_login(&login_form).await;
    // Assert that login was successful, redirected to /admin/dashboard.
    assert_is_redirect_to(&response, "/admin/dashboard");
}
