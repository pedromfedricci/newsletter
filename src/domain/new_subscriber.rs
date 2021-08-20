use super::subscriber_email::{SubscriberEmail, SubscriberEmailParseError};
use super::subscriber_name::{SubscriberName, SubscriberNameParseError};

#[derive(Debug)]
pub(crate) struct NewSubscriber {
    pub(crate) email: SubscriberEmail,
    pub(crate) name: SubscriberName,
}

#[derive(Debug)]
pub(crate) enum SubscriberParseError {
    Email(SubscriberEmailParseError),
    Name(SubscriberNameParseError),
}

impl From<SubscriberEmailParseError> for SubscriberParseError {
    fn from(err: SubscriberEmailParseError) -> Self {
        Self::Email(err)
    }
}

impl From<SubscriberNameParseError> for SubscriberParseError {
    fn from(err: SubscriberNameParseError) -> Self {
        Self::Name(err)
    }
}

impl std::fmt::Display for SubscriberParseError {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self {
            Self::Email(err) => write!(fmt, "{}", err),
            Self::Name(err) => write!(fmt, "{}", err),
        }
    }
}
