// temporarly workaround for clippy incorrect
// lint at crate::routes::subscription::subscribe
#![allow(clippy::async_yields_async)]

use actix_web::{
    web::{Data, Form},
    HttpResponse,
};
use chrono::Utc;
use sqlx::PgPool;
use std::convert::TryInto;
use uuid::Uuid;

use crate::domain::{NewSubscriber, SubscriberEmail, SubscriberName, SubscriberParseError};

#[derive(Debug, serde::Deserialize)]
pub(crate) struct FormData {
    #[serde(rename(serialize = "email", deserialize = "email"))]
    email: String,
    #[serde(rename(serialize = "name", deserialize = "name"))]
    name: String,
}

impl TryInto<NewSubscriber> for FormData {
    type Error = SubscriberParseError;
    fn try_into(self) -> Result<NewSubscriber, Self::Error> {
        Ok(NewSubscriber {
            email: SubscriberEmail::parse(self.email)?,
            name: SubscriberName::parse(self.name)?,
        })
    }
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
    let new_subscriber = match form.0.try_into() {
        Ok(new_subscriber) => new_subscriber,
        Err(_) => return HttpResponse::BadRequest().finish(),
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
