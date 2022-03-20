use std::fmt::Write;

use actix_web::http::header::ContentType;
use actix_web::HttpResponse;
use actix_web_flash_messages::IncomingFlashMessages;

use crate::session_state::TypedSession;
use crate::utils::{err500, see_other};

pub(crate) async fn change_password_form(
    session: TypedSession,
    flash_messages: IncomingFlashMessages,
) -> Result<HttpResponse, actix_web::Error> {
    if session.get_user_id().map_err(err500)?.is_none() {
        return Ok(see_other("/login"));
    };

    let mut messages = String::new();
    for msg in flash_messages.iter() {
        writeln!(messages, "<p><i>{}</i></p>", msg.content()).unwrap();
    }

    let html = format!(include_str!("password.html"), messages);
    Ok(HttpResponse::Ok().content_type(ContentType::html()).body(html))
}
