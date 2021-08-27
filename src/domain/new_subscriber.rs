use super::subscriber_email::{SubscriberEmail, SubscriberEmailParseError};
use super::subscriber_name::{SubscriberName, SubscriberNameParseError};

#[derive(Debug)]
pub(crate) struct NewSubscriber {
    pub(crate) email: SubscriberEmail,
    pub(crate) name: SubscriberName,
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum SubscriberParseError {
    #[error(transparent)]
    Email(#[from] SubscriberEmailParseError),
    #[error(transparent)]
    Name(#[from] SubscriberNameParseError),
}
