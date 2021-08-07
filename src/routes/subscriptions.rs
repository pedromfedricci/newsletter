use actix_web::{
    web::{Data, Form},
    HttpResponse,
};
use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(serde::Deserialize)]
pub(crate) struct FormData {
    #[serde(rename(serialize = "email", deserialize = "email"))]
    email: String,
    #[serde(rename(serialize = "name", deserialize = "name"))]
    name: String,
}

pub(crate) async fn subscribe(form: Form<FormData>, db_pool: Data<PgPool>) -> HttpResponse {
    let query = sqlx::query!(
        r#"
        INSERT INTO subscriptions (id, email, name, subscribed_at)
        VALUES ($1, $2, $3, $4)
        "#,
        Uuid::new_v4(),
        form.email,
        form.name,
        Utc::now()
    );

    match query.execute(db_pool.get_ref()).await {
        Ok(_) => HttpResponse::Ok().finish(),
        Err(err) => {
            println!("Failed to execute query: {}", err);
            HttpResponse::InternalServerError().finish()
        }
    }
}
