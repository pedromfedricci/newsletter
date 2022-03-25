use std::fmt::Write;

use actix_web::http::header::ContentType;
use actix_web::HttpResponse;
use actix_web_flash_messages::IncomingFlashMessages;

pub(crate) async fn publish_newsletter_form(
    flash_messages: IncomingFlashMessages,
) -> Result<HttpResponse, actix_web::Error> {
    let mut messages = String::new();
    for msg in flash_messages.iter() {
        writeln!(messages, "<p><i>{}</i></p>", msg.content()).unwrap();
    }
    let idempotency_key = uuid::Uuid::new_v4();

    let html = format!(
        include_str!("newsletter.html"),
        messages = messages,
        idempotency_key = idempotency_key
    );
    Ok(HttpResponse::Ok().content_type(ContentType::html()).body(html))
}
