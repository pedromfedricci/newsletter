use actix_web::{http::header::LOCATION, HttpResponse};

// Return an opaque 500 while preserving the error root cause for logging.
pub(crate) fn err500<T>(err: T) -> actix_web::Error
where
    T: std::fmt::Debug + std::fmt::Display + 'static,
{
    actix_web::error::ErrorInternalServerError(err)
}

pub(crate) fn err400<T>(err: T) -> actix_web::Error
where
    T: std::fmt::Debug + std::fmt::Display + 'static,
{
    actix_web::error::ErrorBadRequest(err)
}

pub(crate) fn see_other(location: &str) -> HttpResponse {
    HttpResponse::SeeOther().insert_header((LOCATION, location)).finish()
}
