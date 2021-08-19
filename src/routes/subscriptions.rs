use actix_web::{
    web::{Data, Form},
    HttpResponse,
};
use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::{
    subscriber_email::SubscriberEmail, subscriber_name::SubscriberName, NewSubscriber,
};

#[derive(Debug, serde::Deserialize)]
pub(crate) struct FormData {
    #[serde(rename(serialize = "email", deserialize = "email"))]
    email: String,
    #[serde(rename(serialize = "name", deserialize = "name"))]
    name: String,
}

#[tracing::instrument(
    name = "Adding a new subscriber",
    skip(form, db_pool),
    fields(
        subscriber_email = %form.email,
        subscriber_name = %form.name
    )
)]
pub(crate) async fn subscribe(form: Form<FormData>, db_pool: Data<PgPool>) -> HttpResponse {
    let new_subscriber = NewSubscriber {
        email: SubscriberEmail::parse(form.0.email).expect(""),
        name: SubscriberName::parse(form.0.name).expect(""),
    };

    match insert_subscriber(&db_pool, &new_subscriber).await {
        Ok(_) => HttpResponse::Ok().finish(),
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
}

#[tracing::instrument(
    name = "Saving new subscriber details in the database",
    skip(new_subscriber, db_pool)
)]
pub(crate) async fn insert_subscriber(
    db_pool: &PgPool,
    new_subscriber: &NewSubscriber,
) -> Result<(), sqlx::Error> {
    let query = sqlx::query!(
        r#"
        INSERT INTO subscriptions (id, email, name, subscribed_at)
        VALUES ($1, $2, $3, $4)
        "#,
        Uuid::new_v4(),
        new_subscriber.email.as_ref(),
        new_subscriber.name.as_ref(),
        Utc::now()
    );

    query.execute(db_pool).await.map_err(|err| {
        tracing::error!("Failed to execute query: {:?}", err);
        err
    })?;

    Ok(())
}
