use std::fmt::Write;

use actix_web::http::header::ContentType;
use actix_web::HttpResponse;
use actix_web_flash_messages::IncomingFlashMessages;

pub(crate) async fn login_form(flash_messages: IncomingFlashMessages) -> HttpResponse {
    let mut messages = String::new();
    for msg in flash_messages.iter() {
        writeln!(messages, "<p><i>{}</i></p>", msg.content()).unwrap();
    }

    let html = format!(include_str!("login.html"), messages = messages);
    HttpResponse::Ok().content_type(ContentType::html()).body(html)
}
