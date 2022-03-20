mod admin;
mod health_check;
mod home;
mod login;
mod subscriptions;
mod subscriptions_confirm;

pub(crate) use admin::{
    admin_dashboard, change_password, change_password_form, logout, publish_newsletter,
    publish_newsletter_form,
};
pub(crate) use health_check::health_check;
pub(crate) use home::home;
pub(crate) use login::{login, login_form};
pub(crate) use subscriptions::subscribe;
pub(crate) use subscriptions_confirm::confirm;

fn error_chain_fmt(
    e: &impl std::error::Error,
    f: &mut std::fmt::Formatter<'_>,
) -> std::fmt::Result {
    writeln!(f, "{}\n", e)?;

    let mut current = e.source();
    while let Some(cause) = current {
        writeln!(f, "Caused by:\n\t{}", cause)?;
        current = cause.source();
    }

    Ok(())
}
