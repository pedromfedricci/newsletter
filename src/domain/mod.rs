mod new_subscriber;
mod subscriber_email;
mod subscriber_name;

pub(crate) use new_subscriber::{NewSubscriber, SubscriberParseError};
pub use subscriber_email::{SubscriberEmail, SubscriberEmailParseError};
pub(crate) use subscriber_name::SubscriberName;
