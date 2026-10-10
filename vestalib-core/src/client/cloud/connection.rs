use crate::api::cloud::{self, Cloud};
use crate::api::{Connection, MissingCredential};
use crate::client::cloud::{CLOUD_TOKEN_HEADER, CloudToken};
use crate::client::with_trailing_slash;
use http::request::Builder;
use url::{Host, Url};

pub const DEFAULT_CLOUD_URL: &str = "https://cloud.vestaboard.com/";

#[derive(Clone, Debug)]
pub struct CloudConnection {
    url: Url,
    token: Option<CloudToken>,
}

impl CloudConnection {
    /// The url must use HTTPS, so the Cloud API token is never sent in plain text.
    /// Plain HTTP is only accepted for loopback addresses, e.g. for a local test server.
    pub fn new(url: Url) -> Result<Self, InsecureScheme> {
        if !is_secure(&url) {
            return Err(InsecureScheme { url });
        }

        Ok(Self {
            url: with_trailing_slash(url),
            token: None,
        })
    }

    /// Cloud API token for endpoints which require authentication.
    pub fn with_token(mut self, token: CloudToken) -> Self {
        self.token = Some(token);
        self
    }
}

impl Connection for CloudConnection {
    type Api = Cloud;

    fn base_url(&self) -> &Url {
        &self.url
    }

    fn authorize(
        &self,
        request_builder: Builder,
        authentication: cloud::Authentication,
    ) -> Result<Builder, MissingCredential> {
        match authentication {
            cloud::Authentication::None => Ok(request_builder),
            cloud::Authentication::CloudToken => match &self.token {
                Some(token) => {
                    Ok(request_builder.header(CLOUD_TOKEN_HEADER, token.as_header_value().clone()))
                }
                None => Err(MissingCredential::CloudToken),
            },
        }
    }
}

#[derive(Debug, thiserror::Error)]
#[error(
    "The Cloud API token may only be sent over HTTPS, but '{}' uses the '{}' scheme",
    url,
    url.scheme()
)]
pub struct InsecureScheme {
    pub url: Url,
}

fn is_secure(url: &Url) -> bool {
    let is_loopback = match url.host() {
        Some(Host::Domain(domain)) => domain == "localhost",
        Some(Host::Ipv4(ip)) => ip.is_loopback(),
        Some(Host::Ipv6(ip)) => ip.is_loopback(),
        None => false,
    };

    match url.scheme() {
        "https" => true,
        "http" => is_loopback,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[yare::parameterized(
        root = { "https://cloud.vestaboard.com", "", "https://cloud.vestaboard.com/" },
        transition = { "https://cloud.vestaboard.com/", "transition", "https://cloud.vestaboard.com/transition" },
        path = { "https://proxy.example/cloud", "vbml/format", "https://proxy.example/cloud/vbml/format" },
    )]
    fn base_url(url: &str, path: &str, expected: &str) {
        let connection = CloudConnection::new(Url::parse(url).unwrap()).unwrap();

        assert_eq!(connection.base_url().join(path).unwrap().as_str(), expected);
    }

    #[yare::parameterized(
        https = { "https://cloud.vestaboard.com" },
        localhost = { "http://localhost:8080" },
        ipv4_loopback = { "http://127.0.0.1:8080" },
        ipv6_loopback = { "http://[::1]:8080" },
    )]
    fn accepts_secure_urls(url: &str) {
        assert!(CloudConnection::new(Url::parse(url).unwrap()).is_ok());
    }

    #[yare::parameterized(
        http = { "http://cloud.vestaboard.com" },
        local_board = { "http://vestaboard.local:7000" },
        private_ip = { "http://192.168.1.101:7000" },
        file = { "file:///tmp/board" },
    )]
    fn rejects_insecure_urls(url: &str) {
        assert!(CloudConnection::new(Url::parse(url).unwrap()).is_err());
    }

    #[test]
    fn default_url_is_valid() {
        assert!(CloudConnection::new(Url::parse(DEFAULT_CLOUD_URL).unwrap()).is_ok());
    }

    #[test]
    fn missing_cloud_token() {
        let connection = CloudConnection::new(Url::parse(DEFAULT_CLOUD_URL).unwrap()).unwrap();

        assert_eq!(
            connection
                .authorize(Builder::new(), cloud::Authentication::CloudToken)
                .unwrap_err(),
            MissingCredential::CloudToken
        );
    }
}
