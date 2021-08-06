use actix_web::{web::Form, HttpResponse};

#[derive(serde::Deserialize)]
pub(crate) struct FormData {
    #[serde(rename(serialize = "email", deserialize = "email"))]
    _email: String,
    #[serde(rename(serialize = "name", deserialize = "name"))]
    _name: String,
}

pub(crate) async fn subscribe(_req: Form<FormData>) -> HttpResponse {
    HttpResponse::Ok().finish()
}
