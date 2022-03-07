use actix_web::{http::StatusCode, web, HttpResponse, ResponseError};
use anyhow::Context;
use sqlx::PgPool;
use uuid::Uuid;

use crate::routes::error_chain_fmt;

#[derive(Debug, serde::Deserialize)]
pub(crate) struct Parameters {
    subscription_token: String,
}

#[tracing::instrument(name = "Confirm a pending subscriber", skip(params, db_pool))]
pub(crate) async fn confirm(
    params: web::Query<Parameters>,
    db_pool: web::Data<PgPool>,
) -> Result<HttpResponse, SubscriptionConfirmError> {
    let subscriber_id = get_subscriber_id_from_token(&db_pool, &params.subscription_token)
        .await
        .context("Failed to query subscriber id from database")?
        .ok_or(SubscriptionConfirmError::TokenNotFound)?;

    confirm_subscriber(&db_pool, subscriber_id)
        .await
        .context("Failed to confirm subscription status on database")?;

    Ok(HttpResponse::Ok().finish())
}

#[tracing::instrument(name = "Mark subscriber as confirmed", skip(subscriber_id, db_pool))]
pub async fn confirm_subscriber(db_pool: &PgPool, subscriber_id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query!("UPDATE subscriptions SET status = 'confirmed' WHERE id = $1", subscriber_id,)
        .execute(db_pool)
        .await?;

    Ok(())
}

#[tracing::instrument(name = "Get subscriber_id from token", skip(subscription_token, db_pool))]
pub async fn get_subscriber_id_from_token(
    db_pool: &PgPool,
    subscription_token: &str,
) -> Result<Option<Uuid>, sqlx::Error> {
    let result = sqlx::query!(
        "SELECT subscriber_id FROM subscription_tokens WHERE subscription_token = $1",
        subscription_token,
    )
    .fetch_optional(db_pool)
    .await?;

    Ok(result.map(|r| r.subscriber_id))
}

#[derive(thiserror::Error)]
pub(crate) enum SubscriptionConfirmError {
    #[error("Failed to match provided token, was not found in the database")]
    TokenNotFound,
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl std::fmt::Debug for SubscriptionConfirmError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl ResponseError for SubscriptionConfirmError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::TokenNotFound => StatusCode::UNAUTHORIZED,
            Self::UnexpectedError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}
