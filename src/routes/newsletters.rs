use actix_web::http::header::{self, HeaderMap, HeaderValue};
use actix_web::http::StatusCode;
use actix_web::web::{Data, Json};
use actix_web::{HttpRequest, HttpResponse, ResponseError};
use anyhow::Context;
use argon2::{Argon2, PasswordHash, PasswordVerifier};
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    domain::SubscriberEmail, email_client::EmailClient, routes::error_chain_fmt,
    telemetry::spawn_blocking_with_tracing,
};

#[derive(Debug, serde::Deserialize)]
pub(crate) struct NewsletterBody {
    title: String,
    content: Content,
}

#[derive(Debug, serde::Deserialize)]
pub(crate) struct Content {
    html: String,
    text: String,
}

struct ConfirmedSubscriber {
    email: SubscriberEmail,
}

struct Credentials {
    username: String,
    password: String,
}

#[tracing::instrument(
    name = "Publish a newsletter issue",
    skip(body, db_pool, email_client, request)
    fields(username=tracing::field::Empty, user_id=tracing::field::Empty)
)]
pub(crate) async fn pubish_newsletter(
    body: Json<NewsletterBody>,
    db_pool: Data<PgPool>,
    email_client: Data<EmailClient>,
    request: HttpRequest,
) -> Result<HttpResponse, PublishError> {
    let _user_id = {
        let credentials =
            basic_authentication(request.headers()).map_err(PublishError::AuthError)?;
        validate_credentials(credentials, &db_pool).await?
    };

    let subscribers = get_confirmed_subscribers(&db_pool)
        .await
        .context("Querying the database for subscribers that have `status` set to `confirmed`")?;

    for subscriber in subscribers {
        match subscriber {
            Ok(subscriber) => {
                email_client
                    .send_email(
                        &subscriber.email,
                        &body.title,
                        &body.content.html,
                        &body.content.text,
                    )
                    .await
                    .with_context(|| {
                        format!("Sending newsletter to subscriber: {}", subscriber.email)
                    })?;
            }
            Err(err) => {
                tracing::warn!(
                    err.cause_chain = ?err,
                    "Skipping a confirmed subscriber. \
                    Their stored details are invalid"
                );
            }
        }
    }

    Ok(HttpResponse::Ok().finish())
}

#[tracing::instrument(name = "Adding a new subscriber", skip(db_pool))]
async fn get_confirmed_subscribers(
    db_pool: &PgPool,
) -> Result<Vec<Result<ConfirmedSubscriber, anyhow::Error>>, sqlx::Error> {
    let rows = sqlx::query!(
        r#"
        SELECT email
        FROM subscriptions
        WHERE status = 'confirmed'
        "#,
    )
    .fetch_all(db_pool)
    .await?;

    let confirmed_subscribers = rows
        .into_iter()
        .map(|row| match SubscriberEmail::parse(row.email) {
            Ok(email) => Ok(ConfirmedSubscriber { email }),
            Err(err) => Err(anyhow::anyhow!(err)),
        })
        .collect();

    Ok(confirmed_subscribers)
}

fn basic_authentication(headers: &HeaderMap) -> Result<Credentials, anyhow::Error> {
    let header_value = headers
        .get("Authorization")
        .context("The 'Authorization' header is missing")?
        .to_str()
        .context("The 'Authorization' header was not valid UTF8 string")?;

    let enconded_segment =
        header_value.strip_prefix("Basic ").context("The authorization scheme was not 'Basic'")?;

    let decoded_bytes = base64::decode_config(enconded_segment, base64::STANDARD)
        .context("Failed to decode 'Basic' credentials with base64")?;

    let decoded_credentials =
        String::from_utf8(decoded_bytes).context("The decoded credential is not a valid UTF8")?;

    let mut credentials = decoded_credentials.splitn(2, ':');

    let username = credentials
        .next()
        .ok_or_else(|| anyhow::anyhow!("A username must be provided in 'Basic' auth"))?
        .to_string();

    let password = credentials
        .next()
        .ok_or_else(|| anyhow::anyhow!("A password must be provided in 'Basic' auth"))?
        .to_string();

    Ok(Credentials { username, password })
}

static DUMMY_HASH: &str = "$argon2id$v=19$m=15000,t=2,p=1$\
    gZiV/M1gPc22ElAH/Jh1Hw$\
    CWOrkoo7oJBQ/iyh7uJ0LO2aLEfrHwTWllSAxT0zRno";

#[tracing::instrument(
    name = "Validate credentials"
    skip(credentials, db_pool)
)]
async fn validate_credentials(
    credentials: Credentials,
    db_pool: &PgPool,
) -> Result<Uuid, PublishError> {
    let mut user_id = None;
    let mut expected_password_hash = DUMMY_HASH.to_string();

    if let Some((stored_user_id, stored_password_hash)) =
        get_stored_credentials(&credentials.username, db_pool)
            .await
            .map_err(PublishError::UnexpectedError)?
    {
        user_id = Some(stored_user_id);
        expected_password_hash = stored_password_hash;
    }

    spawn_blocking_with_tracing(move || {
        verify_password_hash(expected_password_hash, credentials.password)
    })
    .await
    .context("Failed to spawn blocking task")
    .map_err(PublishError::UnexpectedError)?
    .await?;

    user_id.ok_or_else(|| PublishError::AuthError(anyhow::anyhow!("Unknown username.")))
}

#[tracing::instrument(
    name = "Get store credentials"
    skip(username, db_pool)
)]
async fn get_stored_credentials(
    username: &str,
    db_pool: &PgPool,
) -> Result<Option<(Uuid, String)>, anyhow::Error> {
    let row = sqlx::query!(
        r#"
        SELECT user_id, password_hash
        FROM users
        WHERE username = $1
        "#,
        username,
    )
    .fetch_optional(db_pool)
    .await
    .context("Failed to perform a query to validate auth credentials")?
    .map(|row| (row.user_id, row.password_hash));

    Ok(row)
}

#[tracing::instrument(
    name = "Verify password hash"
    skip(expected_password_hash, password_candidate)
)]
async fn verify_password_hash(
    expected_password_hash: String,
    password_candidate: String,
) -> Result<(), PublishError> {
    let expected_password_hash = PasswordHash::new(&expected_password_hash)
        .context("Failed to parse hash in PHC string format")
        .map_err(PublishError::UnexpectedError)?;

    Argon2::default()
        .verify_password(password_candidate.as_bytes(), &expected_password_hash)
        .context("Invalid password")
        .map_err(PublishError::AuthError)
}

#[derive(thiserror::Error)]
pub(crate) enum PublishError {
    #[error("Authentication failed")]
    AuthError(#[source] anyhow::Error),
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl std::fmt::Debug for PublishError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl ResponseError for PublishError {
    fn error_response(&self) -> HttpResponse {
        match self {
            PublishError::UnexpectedError(_) => {
                HttpResponse::new(StatusCode::INTERNAL_SERVER_ERROR)
            }
            PublishError::AuthError(_) => {
                let mut response = HttpResponse::new(StatusCode::UNAUTHORIZED);
                let header_value = HeaderValue::from_str(r#"Basic realm="publish""#).unwrap();
                response.headers_mut().insert(header::WWW_AUTHENTICATE, header_value);

                response
            }
        }
    }
}
