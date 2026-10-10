use crate::api::local::{self, Local};
use crate::api::{Connection, MissingCredential};
use crate::client::local::{API_KEY_HEADER, ApiKey, ENABLEMENT_TOKEN_HEADER};
use crate::client::with_trailing_slash;
use http::request::Builder;
use url::Url;

pub const DEFAULT_LOCAL_URL: &str = "http://vestaboard.local:7000/";

#[derive(Clone, Debug)]
pub struct LocalConnection {
    // Doesn't include the path of an endpoint.
    // A full path would be: http://vestaboard.local:7000/local-api/message,
    // just the url: http://vestaboard.local:7000
    url: Url,
    api_key: Option<ApiKey>,
}

impl LocalConnection {
    pub fn new(url: Url) -> Result<Self, UnsupportedScheme> {
        if url.scheme() != "http" {
            return Err(UnsupportedScheme { url });
        }

        Ok(Self {
            url: with_trailing_slash(url),
            api_key: None,
        })
    }

    /// Local API key for endpoints which require authentication.
    pub fn with_api_key(mut self, api_key: ApiKey) -> Self {
        self.api_key = Some(api_key);
        self
    }
}

impl Connection for LocalConnection {
    type Api = Local;

    fn base_url(&self) -> &Url {
        &self.url
    }

    fn authorize(
        &self,
        request_builder: Builder,
        authentication: local::Authentication<'_>,
    ) -> Result<Builder, MissingCredential> {
        match authentication {
            local::Authentication::None => Ok(request_builder),
            local::Authentication::ApiKey => match &self.api_key {
                Some(api_key) => {
                    Ok(request_builder.header(API_KEY_HEADER, api_key.as_header_value().clone()))
                }
                None => Err(MissingCredential::ApiKey),
            },
            local::Authentication::EnablementToken(token) => Ok(
                request_builder.header(ENABLEMENT_TOKEN_HEADER, token.as_header_value().clone())
            ),
        }
    }
}

#[derive(Debug, thiserror::Error)]
#[error(
    "The Local API is only served over plain HTTP, but '{}' uses the '{}' scheme",
    url,
    url.scheme()
)]
pub struct UnsupportedScheme {
    pub url: Url,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[yare::parameterized(
        host = { "http://vestaboard.local:7000", "http://vestaboard.local:7000/local-api/message" },
        host_with_slash = { "http://vestaboard.local:7000/", "http://vestaboard.local:7000/local-api/message" },
        ip = { "http://192.168.1.101:7000", "http://192.168.1.101:7000/local-api/message" },
        path = { "http://proxy.local/board-2", "http://proxy.local/board-2/local-api/message" },
        path_with_slash = { "http://proxy.local/board-2/", "http://proxy.local/board-2/local-api/message" },
    )]
    fn base_url(url: &str, expected: &str) {
        let connection = LocalConnection::new(Url::parse(url).unwrap()).unwrap();

        assert_eq!(
            connection
                .base_url()
                .join("local-api/message")
                .unwrap()
                .as_str(),
            expected
        );
    }

    #[yare::parameterized(
        https = { "https://vestaboard.local:7000" },
        file = { "file:///tmp/board" },
    )]
    fn rejects_unsupported_schemes(url: &str) {
        assert!(LocalConnection::new(Url::parse(url).unwrap()).is_err());
    }

    #[test]
    fn default_url_is_valid() {
        assert!(LocalConnection::new(Url::parse(DEFAULT_LOCAL_URL).unwrap()).is_ok());
    }

    #[test]
    fn missing_api_key() {
        let connection = LocalConnection::new(Url::parse(DEFAULT_LOCAL_URL).unwrap()).unwrap();

        assert_eq!(
            connection
                .authorize(Builder::new(), local::Authentication::ApiKey)
                .unwrap_err(),
            MissingCredential::ApiKey
        );
    }
}
