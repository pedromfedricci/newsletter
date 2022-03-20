// use actix_web::{web, HttpResponse};
// use actix_web_flash_messages::FlashMessage;
// use secrecy::{ExposeSecret, Secret};
// use sqlx::PgPool;

// use crate::authentication::{
//     change_password as auth_change_password, validate_credentials, AuthError, Credentials, UserId,
// };
// use crate::routes::admin::dashboard::get_username;
// use crate::utils::{err500, see_other};

// #[derive(serde::Deserialize)]
// pub(crate) struct PasswordFormData {
//     current_password: Secret<String>,
//     new_password: Secret<String>,
//     new_password_check: Secret<String>,
// }

// pub(crate) async fn change_password(
//     form: web::Form<PasswordFormData>,
//     db_pool: web::Data<PgPool>,
//     user_id: web::ReqData<UserId>,
// ) -> Result<HttpResponse, actix_web::Error> {
//     if form.new_password.expose_secret() != form.new_password_check.expose_secret() {
//         FlashMessage::error(
//             "You entered two different new passwords - the field values must match.",
//         )
//         .send();
//         return Ok(see_other("/admin/password"));
//     }

//     let user_id = user_id.into_inner();
//     let credentials = {
//         let username = get_username(*user_id, &db_pool).await.map_err(err500)?;
//         Credentials { username, password: form.0.current_password }
//     };

//     if let Err(err) = validate_credentials(credentials, &db_pool).await {
//         return match err {
//             AuthError::InvalidCredentials(_) => {
//                 FlashMessage::error("The current password is incorrect.").send();
//                 Ok(see_other("/admin/password"))
//             }
//             AuthError::UnexpectedError(_) => Err(err500(err)),
//         };
//     }

//     auth_change_password(*user_id, form.0.new_password, &db_pool).await.map_err(err500)?;
//     FlashMessage::info("Your password has been changed.").send();

//     Ok(see_other("/admin/password"))
// }

use crate::authentication::{validate_credentials, AuthError, Credentials, UserId};
use crate::routes::admin::dashboard::get_username;
use crate::utils::{err500, see_other};
use actix_web::{web, HttpResponse};
use actix_web_flash_messages::FlashMessage;
use secrecy::{ExposeSecret, Secret};
use sqlx::PgPool;

#[derive(serde::Deserialize)]
pub struct FormData {
    current_password: Secret<String>,
    new_password: Secret<String>,
    new_password_check: Secret<String>,
}

#[tracing::instrument(skip(form, pool))]
pub(crate) async fn change_password(
    form: web::Form<FormData>,
    pool: web::Data<PgPool>,
    user_id: web::ReqData<UserId>,
) -> Result<HttpResponse, actix_web::Error> {
    let user_id = user_id.into_inner();
    if form.new_password.expose_secret() != form.new_password_check.expose_secret() {
        FlashMessage::error(
            "You entered two different new passwords - the field values must match.",
        )
        .send();
        tracing::debug!("DIFFERENT PASSWORDS PROVIDED");
        return Ok(see_other("/admin/password"));
    }
    let username = get_username(*user_id, &pool).await.map_err(err500)?;
    println!("FOUND USERNAME");
    let credentials = Credentials { username, password: form.0.current_password };
    if let Err(e) = validate_credentials(credentials, &pool).await {
        return match e {
            AuthError::InvalidCredentials(_) => {
                FlashMessage::error("The current password is incorrect.").send();
                tracing::debug!("THE CURRENT PASSWORD IS INCORRECT");
                Ok(see_other("/admin/password"))
            }
            AuthError::UnexpectedError(_) => {
                tracing::debug!("UNEXPECTED ERROR");
                Err(err500(e))
            }
        };
    }
    crate::authentication::change_password(*user_id, form.0.new_password, &pool)
        .await
        .map_err(err500)?;
    tracing::debug!("YOUR PASSWORD HAS BEEN CHANGED");
    FlashMessage::error("Your password has been changed.").send();
    Ok(see_other("/admin/password"))
}
