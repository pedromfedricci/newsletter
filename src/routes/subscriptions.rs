use actix_web::{web::Form, HttpResponse};

#[derive(serde::Deserialize)]
pub(crate) struct FormData {
    email: String,
    name: String,
}

pub(crate) async fn subscribe(_req: Form<FormData>) -> HttpResponse {
    HttpResponse::Ok().finish()
}
