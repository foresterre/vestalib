use crate::client::InvalidToken;
use crate::client::token::use_sensitive_header_value;
use http::{HeaderName, HeaderValue};
use std::fmt;

pub const API_KEY_HEADER: HeaderName = HeaderName::from_static("x-vestaboard-local-api-key");
pub const ENABLEMENT_TOKEN_HEADER: HeaderName =
    HeaderName::from_static("x-vestaboard-local-api-enablement-token");

/// A Vestaboard Local API key
///
/// The key is never printed by its `Debug` implementation. Similarly, it is
/// marked as a sensitive [`HeaderValue`].
#[derive(Clone, PartialEq, Eq, serde::Deserialize)]
#[serde(try_from = "String")]
pub struct ApiKey {
    value: HeaderValue,
}

impl ApiKey {
    pub fn new(key: &str) -> Result<Self, InvalidToken> {
        use_sensitive_header_value(key).map(|value| Self { value })
    }

    pub fn as_header_value(&self) -> &HeaderValue {
        &self.value
    }

    pub fn expose_secret(&self) -> &str {
        self.value
            .to_str()
            .expect("tokens are validated to only contain visible ASCII")
    }
}

impl TryFrom<String> for ApiKey {
    type Error = InvalidToken;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(&value)
    }
}

impl fmt::Debug for ApiKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("ApiKey(<redacted>)")
    }
}

/// A one-time token to enable the Vestaboard Local API
///
/// The token is never printed by its `Debug` implementation. Similarly, it is
/// marked as a sensitive [`HeaderValue`].
#[derive(Clone, PartialEq, Eq)]
pub struct EnablementToken {
    value: HeaderValue,
}

impl EnablementToken {
    pub fn new(token: &str) -> Result<Self, InvalidToken> {
        use_sensitive_header_value(token).map(|value| Self { value })
    }

    pub fn as_header_value(&self) -> &HeaderValue {
        &self.value
    }
}

impl fmt::Debug for EnablementToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("EnablementToken(<redacted>)")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn api_key_is_sensitive_and_redacted() {
        let key = ApiKey::new("secret").unwrap();

        assert!(key.as_header_value().is_sensitive());
        assert_eq!(key.as_header_value(), "secret");
        assert_eq!(key.expose_secret(), "secret");
        assert!(!format!("{:?}", key).contains("secret"));
    }

    #[test]
    fn enablement_token_is_sensitive_and_redacted() {
        let token = EnablementToken::new("secret").unwrap();

        assert!(token.as_header_value().is_sensitive());
        assert_eq!(token.as_header_value(), "secret");
        assert!(!format!("{:?}", token).contains("secret"));
    }

    #[test]
    fn rejects_invalid_tokens() {
        assert_eq!(ApiKey::new(" ").unwrap_err(), InvalidToken::Empty);
        assert_eq!(EnablementToken::new(" ").unwrap_err(), InvalidToken::Empty);
    }

    #[test]
    fn deserialize_api_key() {
        let key: ApiKey = serde_json::from_str(r#""secret""#).unwrap();
        assert_eq!(key.expose_secret(), "secret");

        assert!(serde_json::from_str::<ApiKey>(r#""""#).is_err());
    }
}
