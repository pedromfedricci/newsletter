use crate::helpers::{spawn_app, url_from};
//use libnewsletter::config;
//use sqlx::{Connection, PgConnection};

//
#[actix_rt::test]
async fn test_subscribe_valid_data() {
    let test_app = spawn_app().await;

    let test_name = "le guin";
    let test_email = "ursula_le_guin@gmail.com";
    let test_body = format!(
        "name={}&email={}",
        test_name.replace(" ", "%20"),
        test_email.replace("@", "%40")
    );

    let response = reqwest::Client::new()
        .post(url_from(&test_app.addr, "/subscriptions"))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(test_body)
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(200, response.status().as_u16());

    let query = sqlx::query!("SELECT email, name FROM subscriptions")
        .fetch_one(&test_app.db_pool)
        .await
        .expect("Failed to fetch subscription");

    assert_eq!(query.name, test_name);
    assert_eq!(query.email, test_email);
}

#[actix_rt::test]
async fn test_subscribe_missing_data() {
    let test_app = spawn_app().await;

    let test_cases = vec![
        ("name=le%20guin", "Missing the email."),
        ("email=ursula_le_guin%40gmail.com", "Missing the name."),
        ("", "Missing both name and email."),
    ];

    let client = reqwest::Client::new();
    for (invalid_body, error_message) in test_cases {
        let response = client
            .post(url_from(&test_app.addr, "/subscriptions"))
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
