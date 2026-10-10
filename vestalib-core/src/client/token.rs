use http::HeaderValue;

pub(crate) fn use_sensitive_header_value(token: &str) -> Result<HeaderValue, InvalidToken> {
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
        assert_eq!(use_sensitive_header_value(token).unwrap_err(), expected);
    }

    #[test]
    fn token_is_sensitive() {
        let value = use_sensitive_header_value("secret").unwrap();

        assert!(value.is_sensitive());
        assert_eq!(value, "secret");
    }
}
