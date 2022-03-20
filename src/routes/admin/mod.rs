mod dashboard;
mod logout;
mod newsletter;
mod password;

pub(crate) use dashboard::admin_dashboard;
pub(crate) use logout::logout;
pub(crate) use newsletter::{publish_newsletter, publish_newsletter_form};
pub(crate) use password::{change_password, change_password_form};
