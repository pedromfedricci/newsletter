use actix_web::web::{Data, Form, ReqData};
use actix_web::HttpResponse;
use actix_web_flash_messages::FlashMessage;
use anyhow::Context;
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use crate::authentication::UserId;
use crate::idempotency::{save_response, try_processing, IdempotencyKey, NextAction};
use crate::utils::{err400, err500, see_other};

#[derive(serde::Deserialize)]
pub(crate) struct NewsletterFormData {
    title: String,
    text_content: String,
    html_content: String,
    idempotency_key: String,
}

#[inline]
fn success_message() -> FlashMessage {
    FlashMessage::info("The newsletter issue has been accepted - emails will go out shortly.")
}

#[tracing::instrument(
    name = "Publish a newsletter issue",
    skip_all,
    fields(user_id=%&*user_id)
)]
pub(crate) async fn publish_newsletter(
    form: Form<NewsletterFormData>,
    user_id: ReqData<UserId>,
    db_pool: Data<PgPool>,
) -> Result<HttpResponse, actix_web::Error> {
    let user_id = user_id.into_inner();
    let NewsletterFormData { title, text_content, html_content, idempotency_key } = form.0;
    let idempotency_key: IdempotencyKey = idempotency_key.try_into().map_err(err400)?;

    let mut transaction =
        match try_processing(&db_pool, &idempotency_key, *user_id).await.map_err(err500)? {
            NextAction::StartProcessing(transaction) => transaction,
            NextAction::ReturnSavedResponse(saved_response) => {
                success_message().send();
                return Ok(saved_response);
            }
        };

    let issue_id = insert_newsletter_issue(&mut transaction, &title, &text_content, &html_content)
        .await
        .context("failed to store newsletter issue details")
        .map_err(err500)?;

    enqueue_delivery_tasks(&mut transaction, issue_id)
        .await
        .context("falied to enqueue delivery tasks")
        .map_err(err500)?;

    let response = {
        let response = see_other("/admin/newsletters");
        save_response(*transaction, &idempotency_key, *user_id, response).await.map_err(err500)?
    };
    success_message().send();

    Ok(response)
}

#[tracing::instrument(skip_all)]
async fn insert_newsletter_issue(
    transaction: &mut Transaction<'_, Postgres>,
    title: &str,
    text_content: &str,
    html_content: &str,
) -> Result<Uuid, sqlx::Error> {
    let newsletter_issue_id = Uuid::new_v4();
    sqlx::query!(
        r#"
        INSERT INTO newsletter_issues (
            newsletter_issue_id,
            title,
            text_content,
            html_content,
            published_at
        )
        VALUES ($1, $2, $3, $4, now())
        "#,
        newsletter_issue_id,
        title,
        text_content,
        html_content,
    )
    .execute(transaction)
    .await?;

    Ok(newsletter_issue_id)
}

#[tracing::instrument(skip_all)]
async fn enqueue_delivery_tasks(
    transaction: &mut Transaction<'static, Postgres>,
    newsletter_issue_id: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        INSERT INTO issue_delivery_queue (
            newsletter_issue_id,
            subscriber_email
        )
        SELECT $1, email
        FROM subscriptions
        WHERE status = 'confirmed'
        "#,
        newsletter_issue_id,
    )
    .execute(transaction)
    .await?;

    Ok(())
}
