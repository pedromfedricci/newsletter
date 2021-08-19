use self::{subscriber_email::SubscriberEmail, subscriber_name::SubscriberName};

pub(crate) mod subscriber_email;
pub(crate) mod subscriber_name;

#[derive(Debug)]
pub(crate) struct NewSubscriber {
    pub(crate) email: SubscriberEmail,
    pub(crate) name: SubscriberName,
}
