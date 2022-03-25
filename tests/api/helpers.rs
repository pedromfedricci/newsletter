use argon2::{password_hash::SaltString, Algorithm, Argon2, Params, PasswordHasher, Version};
use libnewsletter::email_client::EmailClient;
use once_cell::sync::Lazy;
use reqwest::Url;
use sqlx::{Connection, Executor, PgConnection, PgPool};
use std::net::SocketAddr;
use uuid::Uuid;
use wiremock::MockServer;

use libnewsletter::config::{self, DatabaseSettings};
use libnewsletter::issue_delivery_worker::{try_execute_task, ExecutionOutcome};
use libnewsletter::startup::{get_connection_pool, Application};
use libnewsletter::telemetry;

static TRACING: Lazy<()> = Lazy::new(|| {
    let default_filter = "info".to_string();
    let name = "test".to_string();

    if std::env::var("TEST_LOG").is_ok() {
        let subscriber = telemetry::get_subscriber(name, default_filter, std::io::stdout);
        telemetry::init_subscriber(subscriber);
    } else {
        let subscriber = telemetry::get_subscriber(name, default_filter, std::io::sink);
        telemetry::init_subscriber(subscriber);
    }
});

/// Confirmation links embedded in the request to the email API.
pub(crate) struct ConfirmationLinks {
    pub(crate) html: reqwest::Url,
    pub(crate) plain_text: reqwest::Url,
}

pub(crate) struct TestApp {
    pub(crate) addr: SocketAddr,
    pub(crate) db_pool: PgPool,
    pub(crate) email_server: MockServer,
    pub(crate) user: TestUser,
    pub(crate) client: reqwest::Client,
    pub(crate) email_client: EmailClient,
}

impl TestApp {
    /// Extract the confirmation links embedded in the request to the email API.
    pub(crate) fn get_confirmation_links(
        &self,
        email_request: &wiremock::Request,
    ) -> ConfirmationLinks {
        let body: serde_json::Value = serde_json::from_slice(&email_request.body).unwrap();

        // Extract the link from one of the request fields.
        let get_link = |s: &str| {
            let links: Vec<_> = linkify::LinkFinder::new()
                .links(s)
                .filter(|l| *l.kind() == linkify::LinkKind::Url)
                .collect();
            assert_eq!(links.len(), 1);
            let raw_link = links[0].as_str().to_owned();
            let mut confirmation_link = reqwest::Url::parse(&raw_link).unwrap();
            // Let's make sure we don't call random APIs on the web
            assert_eq!(confirmation_link.host_str().unwrap(), "localhost");
            confirmation_link.set_port(Some(self.port())).unwrap();
            confirmation_link
        };

        let html = get_link(&body["HtmlBody"].as_str().unwrap());
        let plain_text = get_link(&body["TextBody"].as_str().unwrap());
        ConfirmationLinks { html, plain_text }
    }

    pub(crate) async fn post_subscriptions<Body>(&self, body: &Body) -> reqwest::Response
    where
        Body: serde::Serialize,
    {
        self.client
            .post(self.with_path("/subscriptions"))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .form(body)
            .send()
            .await
            .expect("Failed to execute the request")
    }

    pub(crate) async fn get_publish_newsletter(&self) -> reqwest::Response {
        self.client
            .get(self.with_path("/admin/newsletters"))
            .send()
            .await
            .expect("Failed to execute request.")
    }

    pub(crate) async fn get_publish_newsletter_html(&self) -> String {
        self.get_publish_newsletter().await.text().await.unwrap()
    }

    pub async fn post_publish_newsletter<Body>(&self, body: &Body) -> reqwest::Response
    where
        Body: serde::Serialize,
    {
        self.client
            .post(self.with_path("/admin/newsletters"))
            .form(body)
            .send()
            .await
            .expect("Failed to execute request.")
    }

    pub(crate) async fn get_login(&self) -> reqwest::Response {
        self.client
            .get(self.with_path("/login"))
            .send()
            .await
            .expect("failed to send GET request to /login")
    }

    pub(crate) async fn get_login_html(&self) -> String {
        self.get_login().await.text().await.unwrap()
    }

    pub(crate) async fn post_login<Body>(&self, body: &Body) -> reqwest::Response
    where
        Body: serde::Serialize,
    {
        self.client
            .post(self.with_path("/login"))
            .form(body)
            .send()
            .await
            .expect("failed to send POST request to /login")
    }

    pub async fn post_logout(&self) -> reqwest::Response {
        self.client
            .post(self.with_path("/admin/logout"))
            .send()
            .await
            .expect("Failed to execute request.")
    }

    pub(crate) async fn get_admin_dashboard(&self) -> reqwest::Response {
        self.client
            .get(self.with_path("/admin/dashboard"))
            .send()
            .await
            .expect("failed to send GET request to /admin/dashboard")
    }

    pub(crate) async fn get_admin_dashboard_html(&self) -> String {
        self.get_admin_dashboard().await.text().await.unwrap()
    }

    pub(crate) async fn get_change_password(&self) -> reqwest::Response {
        self.client
            .get(self.with_path("/admin/password"))
            .send()
            .await
            .expect("Failed to execute request.")
    }

    pub(crate) async fn post_change_password<Body>(&self, body: &Body) -> reqwest::Response
    where
        Body: serde::Serialize,
    {
        self.client
            .post(self.with_path("/admin/password"))
            .form(body)
            .send()
            .await
            .expect("Failed to execute request.")
    }

    pub(crate) async fn get_change_password_html(&self) -> String {
        self.get_change_password().await.text().await.unwrap()
    }

    pub(crate) fn port(&self) -> u16 {
        self.addr.port()
    }

    pub(crate) fn base_url(&self) -> String {
        let (host, port) = (self.addr.ip(), self.addr.port());
        format!("http://{host}:{port}")
    }

    // Helper function to create URL from address and path.
    pub(crate) fn with_path(&self, path: &str) -> Url {
        let base_url = self.base_url();
        Url::parse(&format!("{base_url}{path}")).expect("Failed to parse URL from address and path")
    }

    pub(crate) async fn login_test_user(&self) -> reqwest::Response {
        let username = &self.user.username;
        let password = &self.user.password;
        let login_form = [("username", username), ("password", password)];

        self.post_login(&login_form).await
    }

    pub(crate) async fn dispatch_all_pending_emails(&self) {
        loop {
            if let ExecutionOutcome::EmptyQueue =
                try_execute_task(&self.db_pool, &self.email_client).await.unwrap()
            {
                break;
            }
        }
    }
}

pub(crate) struct TestUser {
    pub(crate) user_id: Uuid,
    pub(crate) username: String,
    pub(crate) password: String,
}

impl TestUser {
    pub(crate) fn generate() -> Self {
        Self {
            user_id: Uuid::new_v4(),
            username: Uuid::new_v4().to_string(),
            password: Uuid::new_v4().to_string(),
        }
    }

    pub(crate) async fn store(&self, pool: &PgPool) {
        let password_hash = {
            let salt = SaltString::generate(&mut rand::thread_rng());
            Argon2::new(
                Algorithm::Argon2id,
                Version::V0x13,
                Params::new(15000, 2, 1, None).unwrap(),
            )
            .hash_password(self.password.as_bytes(), &salt)
            .unwrap()
            .to_string()
        };

        sqlx::query!(
            "INSERT INTO users (user_id, username, password_hash) VALUES ($1, $2, $3)",
            self.user_id,
            self.username,
            password_hash,
        )
        .execute(pool)
        .await
        .expect("Failed to store test user.");
    }

    pub(crate) async fn _login(&self, test_app: &TestApp) {
        test_app.post_login(&[("username", &self.username), ("password", &self.password)]).await;
    }
}

// Runs the server to test the public APIs.
pub(crate) async fn spawn_app() -> TestApp {
    Lazy::force(&TRACING);

    // Launch a mock server to stand in for Postmark's API
    let email_server = MockServer::start().await;

    // Randomise configuration to ensure test isolation
    let config = {
        let mut config = config::settings().expect("Failed to read configuration.");
        // Use a different database for each test case
        config.database.database_name = Uuid::new_v4().to_string();
        // Use a random OS port
        config.application.port = 0;
        // Use the mock server as email API
        config.email_client.set_base_url(email_server.uri());
        config
    };

    // Create and migrate the database
    configure_database(&config.database).await;

    let db_pool = get_connection_pool(&config.database);
    let application =
        Application::build(config.clone()).await.expect("Failed to build application");
    let addr = SocketAddr::from(([127, 0, 0, 1], application.port()));
    let user = {
        let user = TestUser::generate();
        user.store(&db_pool).await;
        user
    };
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .cookie_store(true)
        .build()
        .expect("could not build the test client");
    let email_client = config.email_client.client();

    // Spawn application intance
    tokio::spawn(application.run_until_stopped());

    TestApp { addr, db_pool, email_server, user, client, email_client }
}

async fn configure_database(db_settings: &DatabaseSettings) -> PgPool {
    // Single connection to database.
    let mut conn = PgConnection::connect_with(&db_settings.connection_with_host())
        .await
        .expect("Failed to connect to database host");

    // Create new database.
    conn.execute(&*format!(r#"CREATE DATABASE "{}";"#, db_settings.database_name))
        .await
        .expect("Failed to create database");

    // Create database connection pool.
    let db_pool = PgPool::connect_with(db_settings.connection_with_db())
        .await
        .expect("Failed to connect to database");

    // Migrate database.
    sqlx::migrate!("./migrations").run(&db_pool).await.expect("Failed to migrate the database");

    db_pool
}

pub(crate) fn assert_is_redirect_to(response: &reqwest::Response, location: &str) {
    assert_eq!(response.status().as_u16(), 303);
    assert_eq!(response.headers().get("Location").unwrap(), location);
}
