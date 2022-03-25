use actix_web::body::to_bytes;
use actix_web::http::StatusCode;
use actix_web::HttpResponse;
use sqlx::postgres::PgHasArrayType;
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use super::IdempotencyKey;

#[derive(Debug, sqlx::Type)]
#[sqlx(type_name = "header_pair")]
struct HeaderPairRecord {
    name: String,
    value: Vec<u8>,
}

impl PgHasArrayType for HeaderPairRecord {
    fn array_type_info() -> sqlx::postgres::PgTypeInfo {
        sqlx::postgres::PgTypeInfo::with_name("_header_pair")
    }
}

async fn get_saved_response(
    db_pool: &PgPool,
    idempotency_key: &IdempotencyKey,
    user_id: Uuid,
) -> Result<Option<HttpResponse>, anyhow::Error> {
    let saved_response = sqlx::query!(
        r#"
        SELECT
            response_status_code as "response_status_code!",
            response_headers as "response_headers!: Vec<HeaderPairRecord>",
            response_body as "response_body!"
        FROM idempotency
        WHERE
            user_id = $1
            AND
            idempotency_key = $2
        "#,
        user_id,
        idempotency_key.as_ref()
    )
    .fetch_optional(db_pool)
    .await?;

    if let Some(saved_response) = saved_response {
        let status_code = StatusCode::from_u16(saved_response.response_status_code.try_into()?)?;
        let mut response = HttpResponse::build(status_code);
        for HeaderPairRecord { name, value } in saved_response.response_headers {
            response.append_header((name, value));
        }
        Ok(Some(response.body(saved_response.response_body)))
    } else {
        Ok(None)
    }
}

pub(crate) async fn save_response(
    mut transaction: Transaction<'static, Postgres>,
    idempotency_key: &IdempotencyKey,
    user_id: Uuid,
    response: HttpResponse,
) -> Result<HttpResponse, anyhow::Error> {
    let (head, body) = {
        let (head, body) = response.into_parts();
        let body = to_bytes(body).await.map_err(|err| anyhow::anyhow!("{}", err))?;
        (head, body)
    };
    let status_code = head.status().as_u16() as i16;
    let headers = {
        let mut headers = Vec::with_capacity(head.headers().len());
        for (name, value) in head.headers().iter() {
            let name = name.as_str().to_owned();
            let value = value.as_bytes().to_owned();
            headers.push(HeaderPairRecord { name, value });
        }
        headers
    };

    sqlx::query_unchecked!(
        r#"
        UPDATE idempotency
        SET
            response_status_code = $3,
            response_headers = $4,
            response_body = $5
        WHERE
            user_id = $1
            AND
            idempotency_key = $2
        "#,
        user_id,
        idempotency_key.as_ref(),
        status_code,
        headers,
        body.as_ref()
    )
    .execute(&mut transaction)
    .await?;
    transaction.commit().await?;

    let response = head.set_body(body).map_into_boxed_body();
    Ok(response)
}

pub(crate) enum NextAction {
    StartProcessing(Box<Transaction<'static, Postgres>>),
    ReturnSavedResponse(HttpResponse),
}

pub(crate) async fn try_processing(
    db_pool: &PgPool,
    idempotency_key: &IdempotencyKey,
    user_id: Uuid,
) -> Result<NextAction, anyhow::Error> {
    let mut transaction = db_pool.begin().await?;
    let inserted_rows = sqlx::query!(
        r#"
        INSERT INTO idempotency (
            user_id,
            idempotency_key,
            created_at
        )
        VALUES ($1, $2, now())
        ON CONFLICT DO NOTHING
        "#,
        user_id,
        idempotency_key.as_ref()
    )
    .execute(&mut transaction)
    .await?
    .rows_affected();

    if inserted_rows > 0 {
        Ok(NextAction::StartProcessing(Box::new(transaction)))
    } else {
        let saved_response = get_saved_response(db_pool, idempotency_key, user_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("expected a saved response but it was not found"))?;
        Ok(NextAction::ReturnSavedResponse(saved_response))
    }
}
