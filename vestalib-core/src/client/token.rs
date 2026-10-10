use http::{HeaderName, HeaderValue};
use std::fmt;

pub const API_KEY_HEADER: HeaderName = HeaderName::from_static("x-vestaboard-local-api-key");
pub const ENABLEMENT_TOKEN_HEADER: HeaderName =
    HeaderName::from_static("x-vestaboard-local-api-enablement-token");
pub const CLOUD_TOKEN_HEADER: HeaderName = HeaderName::from_static("x-vestaboard-token");

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
        sensitive_header_value(key).map(|value| Self { value })
    }

    pub fn as_header_value(&self) -> &HeaderValue {
        &self.value
    }

    pub fn expose_secret(&self) -> &str {
        expose_secret(&self.value)
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
        sensitive_header_value(token).map(|value| Self { value })
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
        sensitive_header_value(token).map(|value| Self { value })
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

fn sensitive_header_value(token: &str) -> Result<HeaderValue, InvalidToken> {
    if token.trim().is_empty() {
        return Err(InvalidToken::Empty);
    }

    if !token.bytes().all(|byte| byte.is_ascii_graphic()) {
        return Err(InvalidToken::InvalidCharacters);
    }

    let mut value = HeaderValue::from_str(token).map_err(|_| InvalidToken::InvalidCharacters)?;
    value.set_sensitive(true);

    Ok(value)
}

fn expose_secret(value: &HeaderValue) -> &str {
    value
        .to_str()
        .expect("tokens are validated to only contain visible ASCII")
}

// The token itself must not be part of the error, so it can't leak to logs and traces
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum InvalidToken {
    #[error("The token is empty")]
    Empty,
    #[error("The token may only contain visible ASCII characters")]
    InvalidCharacters,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[yare::parameterized(
        empty = { "", InvalidToken::Empty },
        whitespace = { " \t", InvalidToken::Empty },
        newline = { "abc\ndef", InvalidToken::InvalidCharacters },
        inner_space = { "abc def", InvalidToken::InvalidCharacters },
        surrounding_space = { " abc ", InvalidToken::InvalidCharacters },
        non_ascii = { "abcé", InvalidToken::InvalidCharacters },
        delete = { "abc\u{7f}", InvalidToken::InvalidCharacters },
    )]
    fn rejects_invalid_tokens(token: &str, expected: InvalidToken) {
        assert_eq!(ApiKey::new(token).unwrap_err(), expected);
        assert_eq!(EnablementToken::new(token).unwrap_err(), expected);
        assert_eq!(CloudToken::new(token).unwrap_err(), expected);
    }

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
    fn cloud_token_is_sensitive_and_redacted() {
        let token = CloudToken::new("secret").unwrap();

        assert!(token.as_header_value().is_sensitive());
        assert_eq!(token.as_header_value(), "secret");
        assert!(!format!("{:?}", token).contains("secret"));
    }

    #[test]
    fn deserialize_api_key() {
        let key: ApiKey = serde_json::from_str(r#""secret""#).unwrap();
        assert_eq!(key.expose_secret(), "secret");

        assert!(serde_json::from_str::<ApiKey>(r#""""#).is_err());
    }
}
