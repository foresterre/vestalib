use crate::client::InvalidToken;
use crate::client::token::use_sensitive_header_value;
use http::{HeaderName, HeaderValue};
use std::fmt;

pub const CLOUD_TOKEN_HEADER: HeaderName = HeaderName::from_static("x-vestaboard-token");

/// A Vestaboard Cloud API token
///
/// The token is never printed by its `Debug` implementation. Similarly, it is
/// marked as a sensitive [`HeaderValue`].
#[derive(Clone, PartialEq, Eq)]
pub struct CloudToken {
    value: HeaderValue,
}

impl CloudToken {
    pub fn new(token: &str) -> Result<Self, InvalidToken> {
        use_sensitive_header_value(token).map(|value| Self { value })
    }

    pub fn as_header_value(&self) -> &HeaderValue {
        &self.value
    }
}

impl fmt::Debug for CloudToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("CloudToken(<redacted>)")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cloud_token_is_sensitive_and_redacted() {
        let token = CloudToken::new("secret").unwrap();

        assert!(token.as_header_value().is_sensitive());
        assert_eq!(token.as_header_value(), "secret");
        assert!(!format!("{:?}", token).contains("secret"));
    }

    #[test]
    fn rejects_invalid_tokens() {
        assert_eq!(CloudToken::new(" ").unwrap_err(), InvalidToken::Empty);
    }
}
