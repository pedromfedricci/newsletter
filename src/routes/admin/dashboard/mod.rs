use actix_web::http::header::{ContentType, LOCATION};
use actix_web::web::Data;
use actix_web::HttpResponse;
use anyhow::Context;
use sqlx::PgPool;
use uuid::Uuid;

use crate::session_state::TypedSession;
use crate::utils::err500;

pub(crate) async fn admin_dashboard(
    session: TypedSession,
    db_pool: Data<PgPool>,
) -> Result<HttpResponse, actix_web::Error> {
    let username = if let Some(user_id) = session.get_user_id().map_err(err500)? {
        get_username(user_id, &db_pool).await.map_err(err500)?
    } else {
        return Ok(HttpResponse::SeeOther().insert_header((LOCATION, "/login")).finish());
    };

    let html = format!(include_str!("dashboard.html"), username = username);
    Ok(HttpResponse::Ok().content_type(ContentType::html()).body(html))
}

#[inline]
#[tracing::instrument(name = "Get username", skip(db_pool))]
pub(crate) async fn get_username(user_id: Uuid, db_pool: &PgPool) -> Result<String, anyhow::Error> {
    let row = sqlx::query!("SELECT username FROM users WHERE user_id = $1", user_id)
        .fetch_one(db_pool)
        .await
        .context("Failed to perform a query to retrieve a username")?;

    Ok(row.username)
}
