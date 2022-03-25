#[derive(Debug)]
pub(crate) struct IdempotencyKey(String);

impl IdempotencyKey {
    const MAX_LENGHT: u16 = 50;
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum IdempotencyKeyError {
    #[error("idempotency key must be shorter than {}", IdempotencyKey::MAX_LENGHT)]
    MaxLenght,
    #[error("idempotency key cannot be empty")]
    Empty,
}

impl TryFrom<String> for IdempotencyKey {
    type Error = IdempotencyKeyError;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        if s.is_empty() {
            Err(IdempotencyKeyError::Empty)
        } else if s.len() as u16 >= IdempotencyKey::MAX_LENGHT {
            Err(IdempotencyKeyError::MaxLenght)
        } else {
            Ok(IdempotencyKey(s))
        }
    }
}

impl From<IdempotencyKey> for String {
    fn from(key: IdempotencyKey) -> String {
        key.0
    }
}

impl AsRef<str> for IdempotencyKey {
    fn as_ref(&self) -> &str {
        &self.0
    }
}
