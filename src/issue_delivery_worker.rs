use std::time::Duration;

use sqlx::{PgPool, Postgres, Transaction};
use tracing::{field::display, Span};
use uuid::Uuid;

use crate::config::Settings;
use crate::domain::SubscriberEmail;
use crate::email_client::EmailClient;
use crate::startup::get_connection_pool;

pub enum ExecutionOutcome {
    TaskCompleted,
    EmptyQueue,
}

const FAILED_TO_DELIVER_MSG: &str = "Failed to deliver issue to a confirmed subscriber, skipping.";
const INVALID_SUBSCRIBER_DETAILS: &str =
    "Skipping a confirmed subscriber, store contact details are invalid";

#[tracing::instrument(
skip_all,
fields(
    newsletter_issue_id=tracing::field::Empty,
    subscriber_email=tracing::field::Empty,
),
err)]
pub async fn try_execute_task(
    db_pool: &PgPool,
    email_client: &EmailClient,
) -> Result<ExecutionOutcome, anyhow::Error> {
    let (transaction, issue_id, email) = match dequeue_task(db_pool).await? {
        None => return Ok(ExecutionOutcome::EmptyQueue),
        Some(task) => task,
    };

    Span::current()
        .record("newsletter_issue_id", &display(issue_id))
        .record("subscriber_email", &display(&email));

    match SubscriberEmail::parse(email.clone()) {
        Ok(email) => {
            let issue = get_issue(db_pool, issue_id).await?;
            if let Err(err) = email_client
                .send_email(&email, &issue.title, &issue.html_content, &issue.text_content)
                .await
            {
                tracing::error!(error.cause_chain = ?err, error.message = %err, FAILED_TO_DELIVER_MSG);
            }
        }
        Err(err) => {
            tracing::error!(error.cause_chain = ?err, error.message = %err, INVALID_SUBSCRIBER_DETAILS);
        }
    }

    delete_task(transaction, issue_id, &email).await?;
    Ok(ExecutionOutcome::TaskCompleted)
}

#[tracing::instrument(skip_all)]
async fn dequeue_task(
    db_pool: &PgPool,
) -> Result<Option<(Transaction<'static, Postgres>, Uuid, String)>, anyhow::Error> {
    let mut transaction = db_pool.begin().await?;
    let record = sqlx::query!(
        r#"
        SELECT newsletter_issue_id, subscriber_email
        FROM issue_delivery_queue
        FOR UPDATE
        SKIP LOCKED
        LIMIT 1  
        "#,
    )
    .fetch_optional(&mut transaction)
    .await?;

    if let Some(record) = record {
        Ok(Some((transaction, record.newsletter_issue_id, record.subscriber_email)))
    } else {
        Ok(None)
    }
}

#[tracing::instrument(skip_all)]
async fn delete_task(
    mut transaction: Transaction<'static, Postgres>,
    issue_id: Uuid,
    email: &str,
) -> Result<(), anyhow::Error> {
    sqlx::query!(
        r#"
        DELETE FROM issue_delivery_queue
        WHERE
            newsletter_issue_id = $1
            AND
            subscriber_email = $2
        "#,
        issue_id,
        email,
    )
    .execute(&mut transaction)
    .await?;

    transaction.commit().await?;
    Ok(())
}

struct NewsletterIssue {
    title: String,
    text_content: String,
    html_content: String,
}

#[tracing::instrument(skip_all)]
async fn get_issue(db_pool: &PgPool, issue_id: Uuid) -> Result<NewsletterIssue, anyhow::Error> {
    let issue = sqlx::query_as!(
        NewsletterIssue,
        r#"
        SELECT title, text_content, html_content
        FROM newsletter_issues
        WHERE newsletter_issue_id = $1
        "#,
        issue_id
    )
    .fetch_one(db_pool)
    .await?;

    Ok(issue)
}

pub async fn run_worker_until_stopped(config: Settings) -> Result<(), anyhow::Error> {
    let db_pool = get_connection_pool(&config.database);
    let email_client = config.email_client.client();
    worker_loop(&db_pool, &email_client).await
}

async fn worker_loop(db_pool: &PgPool, email_client: &EmailClient) -> Result<(), anyhow::Error> {
    loop {
        match try_execute_task(db_pool, email_client).await {
            Ok(ExecutionOutcome::TaskCompleted) => {}
            Ok(ExecutionOutcome::EmptyQueue) => tokio::time::sleep(Duration::from_secs(10)).await,
            Err(_) => tokio::time::sleep(Duration::from_secs(1)).await,
        }
    }
}
