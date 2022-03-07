use actix_web::http::StatusCode;
use actix_web::web::{Data, Form};
use actix_web::{HttpResponse, ResponseError};
use anyhow::Context;
use chrono::Utc;
use rand::{distributions::Alphanumeric, thread_rng, Rng};
use sqlx::{PgPool, Postgres, Transaction};
use std::convert::TryInto;
use uuid::Uuid;

use crate::domain::{NewSubscriber, SubscriberEmail, SubscriberName, SubscriberParseError};
use crate::email_client::EmailClient;
use crate::routes::error_chain_fmt;
use crate::startup::ApplicationBaseUrl;

#[derive(Debug, serde::Deserialize)]
pub(crate) struct SubscriberForm {
    #[serde(rename(serialize = "email", deserialize = "email"))]
    email: String,
    #[serde(rename(serialize = "name", deserialize = "name"))]
    name: String,
}

impl TryInto<NewSubscriber> for SubscriberForm {
    type Error = SubscriberParseError;
    fn try_into(self) -> Result<NewSubscriber, Self::Error> {
        Ok(NewSubscriber {
            email: SubscriberEmail::parse(self.email)?,
            name: SubscriberName::parse(self.name)?,
        })
    }
}

fn generate_subscription_token() -> String {
    let mut rng = thread_rng();
    std::iter::repeat_with(|| rng.sample(Alphanumeric)).map(char::from).take(25).collect()
}

#[tracing::instrument(
    name = "Adding a new subscriber",
    skip(form, db_pool,email_client, base_url),
    fields(
        subscriber_email = %form.email,
        subscriber_name = %form.name
    )
)]
pub(crate) async fn subscribe(
    form: Form<SubscriberForm>,
    db_pool: Data<PgPool>,
    email_client: Data<EmailClient>,
    base_url: Data<ApplicationBaseUrl>,
) -> Result<HttpResponse, SubscriberError> {
    let new_subscriber = form.0.try_into().map_err(SubscriberError::ValidationError)?;

    let mut transaction =
        db_pool.begin().await.context("Failed to acquire a Postgres connection from the pool")?;

    let subscriber_id = insert_subscriber(&mut transaction, &new_subscriber)
        .await
        .context("Failed to insert new subscriber in the database")?;

    let subscription_token = generate_subscription_token();
    store_token(&mut transaction, subscriber_id, &subscription_token)
        .await
        .context("Failed to store the confirmation token for a new subscriber")?;

    transaction
        .commit()
        .await
        .context("Failed to commit SQL transaction to store a new subscriber")?;

    send_confirmation_email(&email_client, &new_subscriber, &base_url.0, &subscription_token)
        .await
        .context("Failed to send a confirmation email")?;

    Ok(HttpResponse::Ok().finish())
}

#[tracing::instrument(
    name = "Saving new subscriber details in the database",
    skip(new_subscriber, transaction)
)]
pub(crate) async fn insert_subscriber(
    transaction: &mut Transaction<'_, Postgres>,
    new_subscriber: &NewSubscriber,
) -> Result<Uuid, sqlx::Error> {
    let subscriber_id = Uuid::new_v4();

    sqlx::query!(
        "INSERT INTO subscriptions (id, email, name, subscribed_at, status) VALUES ($1, $2, $3, $4, 'pending_confirmation')",
        subscriber_id,
        new_subscriber.email.as_ref(),
        new_subscriber.name.as_ref(),
        Utc::now()
    )
    .execute(transaction)
    .await?;

    Ok(subscriber_id)
}

#[tracing::instrument(
    name = "Send a confirmation email to a new subscriber",
    skip(email_client, new_subscriber, base_url, subscription_token)
)]
pub(crate) async fn send_confirmation_email(
    email_client: &EmailClient,
    new_subscriber: &NewSubscriber,
    base_url: &str,
    subscription_token: &str,
) -> Result<(), reqwest::Error> {
    let confirmation_link =
        format!("{}/subscriptions/confirm?subscription_token={}", base_url, subscription_token);

    let plain_body = format!(
        "Welcome to our newsletter!\nVisit {} to confirm your subscription.",
        confirmation_link
    );

    let html_body = format!(
        "Welcome to our newsletter!<br />Click <a href=\"{}\">here</a> to confirm your subscription.",
        confirmation_link
    );

    email_client.send_email(&new_subscriber.email, "Welcome!", &html_body, &plain_body).await
}

#[tracing::instrument(
    name = "Store subscription token in the database",
    skip(subscription_token, transaction)
)]
pub(crate) async fn store_token(
    transaction: &mut Transaction<'_, Postgres>,
    subscriber_id: Uuid,
    subscription_token: &str,
) -> Result<(), StoreTokenError> {
    sqlx::query!(
        "INSERT INTO subscription_tokens (subscription_token, subscriber_id) VALUES ($1, $2)",
        subscription_token,
        subscriber_id
    )
    .execute(transaction)
    .await?;

    Ok(())
}

#[derive(thiserror::Error)]
#[error("A database failure was encountered while trying to store a subscription token")]
pub(crate) struct StoreTokenError(#[from] sqlx::Error);

impl std::fmt::Debug for StoreTokenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

#[derive(thiserror::Error)]
pub(crate) enum SubscriberError {
    #[error("Failed to validate subscriber's url encoded payload")]
    ValidationError(#[from] SubscriberParseError),
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl std::fmt::Debug for SubscriberError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl ResponseError for SubscriberError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::ValidationError(_) => StatusCode::BAD_REQUEST,
            Self::UnexpectedError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}
