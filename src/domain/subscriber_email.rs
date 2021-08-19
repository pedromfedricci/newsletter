#[derive(Debug)]
pub(crate) struct SubscriberEmail(String);

impl SubscriberEmail {
    pub(crate) fn parse(email: String) -> Result<Self, EmailParseError> {
        Ok(Self(email))
    }
}

impl AsRef<str> for SubscriberEmail {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[derive(Debug)]
pub(crate) struct EmailParseError;

impl std::fmt::Display for EmailParseError {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fmt.write_str("could not parse provided String as a SubscriberEmail")
    }
}
