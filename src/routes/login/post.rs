use actix_web::error::InternalError;
use actix_web::http::header::LOCATION;
use actix_web::http::StatusCode;
use actix_web::web::{Data, Form};
use actix_web::{HttpResponse, ResponseError};
use actix_web_flash_messages::FlashMessage;
use secrecy::Secret;
use sqlx::PgPool;

use crate::authentication::{validate_credentials, AuthError, Credentials};
use crate::routes::error_chain_fmt;
use crate::session_state::TypedSession;

#[derive(serde::Deserialize)]
pub(crate) struct LoginFormData {
    username: String,
    password: Secret<String>,
}

impl From<LoginFormData> for Credentials {
    fn from(LoginFormData { username, password }: LoginFormData) -> Credentials {
        Credentials { username, password }
    }
}

#[tracing::instrument(
    skip(form, db_pool, session),
    fields(username=tracing::field::Empty, user_id=tracing::field::Empty)
)]
pub(crate) async fn login(
    form: Form<LoginFormData>,
    db_pool: Data<PgPool>,
    session: TypedSession,
) -> Result<HttpResponse, InternalError<LoginError>> {
    let credentials: Credentials = form.0.into();
    tracing::Span::current().record("username", &tracing::field::display(&credentials.username));

    match validate_credentials(credentials, &db_pool).await {
        Err(err) => Err(login_error_redirect(err)),
        Ok(user_id) => {
            tracing::Span::current().record("user_id", &tracing::field::display(&user_id));
            session.renew();
            session.insert_user_id(user_id).map_err(login_error_redirect)?;
            Ok(HttpResponse::SeeOther().insert_header((LOCATION, "/admin/dashboard")).finish())
        }
    }
}

#[derive(thiserror::Error)]
pub(crate) enum LoginError {
    #[error("Authentication failed")]
    AuthError(#[source] anyhow::Error),
    #[error("Something went wrong")]
    UnexpectedError(#[from] anyhow::Error),
}

impl std::fmt::Debug for LoginError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl ResponseError for LoginError {
    fn status_code(&self) -> StatusCode {
        match self {
            LoginError::UnexpectedError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            LoginError::AuthError(_) => StatusCode::UNAUTHORIZED,
        }
    }
}

impl From<AuthError> for LoginError {
    fn from(err: AuthError) -> LoginError {
        match err {
            AuthError::InvalidCredentials(_) => LoginError::AuthError(err.into()),
            AuthError::UnexpectedError(_) => LoginError::UnexpectedError(err.into()),
        }
    }
}

impl From<serde_json::Error> for LoginError {
    fn from(err: serde_json::Error) -> LoginError {
        LoginError::UnexpectedError(err.into())
    }
}

#[inline]
fn login_error_redirect(err: impl Into<LoginError>) -> InternalError<LoginError> {
    let err = err.into();
    FlashMessage::error(err.to_string()).send();
    let response = HttpResponse::SeeOther().insert_header((LOCATION, "/login")).finish();
    InternalError::from_response(err, response)
}
