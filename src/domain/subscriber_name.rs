use unicode_segmentation::UnicodeSegmentation;

#[derive(Debug)]
pub(crate) struct SubscriberName(String);

impl SubscriberName {
    pub(crate) fn parse(name: String) -> Result<Self, SubscriberNameParseError> {
        let is_empty_or_whitespace = name.trim().is_empty();

        let is_too_long = name.graphemes(true).count() > 256;

        let forbidden_characters = ['/', '(', ')', '"', '<', '>', '\\', '{', '}'];
        let has_forbidden_characters = name.chars().any(|g| forbidden_characters.contains(&g));

        if is_empty_or_whitespace || is_too_long || has_forbidden_characters {
            Err(SubscriberNameParseError)
        } else {
            Ok(Self(name))
        }
    }
}

impl AsRef<str> for SubscriberName {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, thiserror::Error)]
#[error("Failed to parse provided String as a SubscriberName")]
pub(crate) struct SubscriberNameParseError;

#[cfg(test)]
mod tests {
    use crate::domain::SubscriberName;
    use claim::{assert_err, assert_ok};

    #[test]
    fn a_256_grapheme_long_name_is_valid() {
        let name = "a̐".repeat(256);
        assert_ok!(SubscriberName::parse(name));
    }

    #[test]
    fn a_name_longer_than_256_graphemes_is_rejected() {
        let name = "a".repeat(257);
        assert_err!(SubscriberName::parse(name));
    }

    #[test]
    fn whitespace_only_names_are_rejected() {
        let name = " ".to_string();
        assert_err!(SubscriberName::parse(name));
    }

    #[test]
    fn empty_string_is_rejected() {
        let name = "".to_string();
        assert_err!(SubscriberName::parse(name));
    }

    #[test]
    fn names_containing_an_invalid_character_are_rejected() {
        for name in vec!['/', '(', ')', '"', '<', '>', '\\', '{', '}'] {
            let name = name.to_string();
            assert_err!(SubscriberName::parse(name));
        }
    }

    #[test]
    fn a_valid_name_is_parsed_successfully() {
        let name = "Ursula Le Guin".to_string();
        assert_ok!(SubscriberName::parse(name));
    }
}
