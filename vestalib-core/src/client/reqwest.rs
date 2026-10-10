use crate::api::ApiError::HttpRequest;
use crate::api::{ApiError, Client};
use crate::client::{API_KEY_HEADER, ApiKey, CLOUD_TOKEN_HEADER, CloudToken};
use bytes::Bytes;
use http::request::Builder;
use reqwest::ResponseBuilderExt;
use std::borrow::Cow;
use url::{Host, Url};

pub const DEFAULT_URL: &str = "http://vestaboard.local:7000/";
pub const DEFAULT_CLOUD_URL: &str = "https://cloud.vestaboard.com/";

pub struct ReqwestClient {
    client: reqwest::blocking::Client,
    // Doesn't include the path of an endpoint.
    // A full path would be: http://vestaboard.local:7000/local-api/message,
    // just the url: http://vestaboard.local:7000
    url: Url,
    api_key: Option<ApiKey>,
}

impl ReqwestClient {
    pub fn new(url: Url) -> Result<Self, NewClientError> {
        if url.scheme() != "http" {
            return Err(NewClientError::UnsupportedScheme { url });
        }

        Ok(Self {
            client: reqwest::blocking::ClientBuilder::default().build()?,
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

impl Client for ReqwestClient {
    type Error = Error;

    fn base_endpoint(&self, path: &str) -> Result<Url, ApiError<Self::Error>> {
        join(&self.url, path)
    }

    fn authorize(&self, request_builder: Builder) -> Result<Builder, ApiError<Self::Error>> {
        match &self.api_key {
            Some(api_key) => {
                Ok(request_builder.header(API_KEY_HEADER, api_key.as_header_value().clone()))
            }
            None => Err(ApiError::NoApiKey),
        }
    }

    fn send(
        &self,
        request_builder: Builder,
        body: Vec<u8>,
    ) -> Result<http::Response<Bytes>, ApiError<Self::Error>> {
        send(&self.client, request_builder, body)
    }
}

pub struct ReqwestCloudClient {
    client: reqwest::blocking::Client,
    url: Url,
    token: Option<CloudToken>,
}

impl ReqwestCloudClient {
    /// The url must use HTTPS, so the Cloud API token is never sent in plain text.
    /// Plain HTTP is only accepted for loopback addresses, e.g. for a local test server.
    pub fn new(url: Url) -> Result<Self, NewClientError> {
        if !is_secure(&url) {
            return Err(NewClientError::InsecureScheme { url });
        }

        Ok(Self {
            client: reqwest::blocking::ClientBuilder::default().build()?,
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

impl Client for ReqwestCloudClient {
    type Error = Error;

    fn base_endpoint(&self, path: &str) -> Result<Url, ApiError<Self::Error>> {
        join(&self.url, path)
    }

    fn authorize(&self, _request_builder: Builder) -> Result<Builder, ApiError<Self::Error>> {
        Err(ApiError::NoApiKey)
    }

    fn authorize_cloud(&self, request_builder: Builder) -> Result<Builder, ApiError<Self::Error>> {
        match &self.token {
            Some(token) => {
                Ok(request_builder.header(CLOUD_TOKEN_HEADER, token.as_header_value().clone()))
            }
            None => Err(ApiError::NoCloudToken),
        }
    }

    fn send(
        &self,
        request_builder: Builder,
        body: Vec<u8>,
    ) -> Result<http::Response<Bytes>, ApiError<Self::Error>> {
        send(&self.client, request_builder, body)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Reqwest {
        #[from]
        error: reqwest::Error,
    },
}

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum NewClientError {
    #[error("Unable to create the HTTP client: '{}'", error)]
    Reqwest {
        #[from]
        error: reqwest::Error,
    },
    #[error(
        "The Local API is only served over plain HTTP, but '{}' uses the '{}' scheme",
        url,
        url.scheme()
    )]
    UnsupportedScheme { url: Url },
    #[error(
        "The Cloud API token may only be sent over HTTPS, but '{}' uses the '{}' scheme",
        url,
        url.scheme()
    )]
    InsecureScheme { url: Url },
}

fn with_trailing_slash(mut url: Url) -> Url {
    if !url.path().ends_with('/') {
        url.set_path(&format!("{}/", url.path()));
    }

    url
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

fn join<E: std::error::Error + Send + Sync + 'static>(
    url: &Url,
    path: &str,
) -> Result<Url, ApiError<E>> {
    url.join(path).map_err(|e| ApiError::Url {
        error: e,
        url: url.to_string(),
        path: Cow::Owned(path.to_string()),
    })
}

fn send(
    client: &reqwest::blocking::Client,
    request_builder: Builder,
    body: Vec<u8>,
) -> Result<http::Response<Bytes>, ApiError<Error>> {
    let request = request_builder
        .body(body)
        .map_err(|error| HttpRequest { error })?;

    send_request(client, request).map_err(|error| ApiError::Client { error })
}

fn send_request(
    client: &reqwest::blocking::Client,
    request: http::Request<Vec<u8>>,
) -> Result<http::Response<Bytes>, Error> {
    // https://docs.rs/reqwest/latest/reqwest/blocking/struct.Request.html#method.try_from
    let req = request.try_into()?;

    // execute the query
    let res = client.execute(req)?;

    // no try_into from reqwest::blocking::Response for http::Response
    let http_res = into_http_response(res)?;

    Ok(http_res)
}

fn into_http_response(res: reqwest::blocking::Response) -> Result<http::Response<Bytes>, Error> {
    let mut builder = http::response::Builder::new()
        .status(res.status())
        .version(res.version())
        .url(res.url().clone());

    for (name, value) in res.headers() {
        builder = builder.header(name, value);
    }

    let body = res.bytes()?;
    let response = builder.body(body).unwrap();

    Ok(response)
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
    fn base_endpoint(url: &str, expected: &str) {
        let client = ReqwestClient::new(Url::parse(url).unwrap()).unwrap();

        assert_eq!(
            client.base_endpoint("local-api/message").unwrap().as_str(),
            expected
        );
    }

    #[yare::parameterized(
        https = { "https://vestaboard.local:7000" },
        file = { "file:///tmp/board" },
    )]
    fn rejects_unsupported_schemes(url: &str) {
        assert!(matches!(
            ReqwestClient::new(Url::parse(url).unwrap()),
            Err(NewClientError::UnsupportedScheme { .. })
        ));
    }

    #[test]
    fn default_url_is_valid() {
        assert!(ReqwestClient::new(Url::parse(DEFAULT_URL).unwrap()).is_ok());
    }

    #[yare::parameterized(
        root = { "https://cloud.vestaboard.com", "", "https://cloud.vestaboard.com/" },
        transition = { "https://cloud.vestaboard.com/", "transition", "https://cloud.vestaboard.com/transition" },
        path = { "https://proxy.example/cloud", "vbml/format", "https://proxy.example/cloud/vbml/format" },
    )]
    fn cloud_base_endpoint(url: &str, path: &str, expected: &str) {
        let client = ReqwestCloudClient::new(Url::parse(url).unwrap()).unwrap();

        assert_eq!(client.base_endpoint(path).unwrap().as_str(), expected);
    }

    #[yare::parameterized(
        https = { "https://cloud.vestaboard.com" },
        localhost = { "http://localhost:8080" },
        ipv4_loopback = { "http://127.0.0.1:8080" },
        ipv6_loopback = { "http://[::1]:8080" },
    )]
    fn cloud_accepts_secure_urls(url: &str) {
        assert!(ReqwestCloudClient::new(Url::parse(url).unwrap()).is_ok());
    }

    #[yare::parameterized(
        http = { "http://cloud.vestaboard.com" },
        local_board = { "http://vestaboard.local:7000" },
        private_ip = { "http://192.168.1.101:7000" },
        file = { "file:///tmp/board" },
    )]
    fn cloud_rejects_insecure_urls(url: &str) {
        assert!(matches!(
            ReqwestCloudClient::new(Url::parse(url).unwrap()),
            Err(NewClientError::InsecureScheme { .. })
        ));
    }

    #[test]
    fn default_cloud_url_is_valid() {
        assert!(ReqwestCloudClient::new(Url::parse(DEFAULT_CLOUD_URL).unwrap()).is_ok());
    }
}
