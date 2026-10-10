use crate::api::ApiError::HttpRequest;
use crate::api::{ApiError, Client};
use crate::client::{API_KEY_HEADER, ApiKey};
use bytes::Bytes;
use http::request::Builder;
use reqwest::ResponseBuilderExt;
use std::borrow::Cow;
use url::Url;

pub const DEFAULT_URL: &str = "http://vestaboard.local:7000/";

pub struct ReqwestClient {
    client: reqwest::blocking::Client,
    // Doesn't include the path of an endpoint.
    // A full path would be: http://vestaboard.local:7000/local-api/message,
    // just the url: http://vestaboard.local:7000
    url: Url,
    api_key: Option<ApiKey>,
}

impl ReqwestClient {
    pub fn new(mut url: Url) -> Result<Self, NewClientError> {
        if url.scheme() != "http" {
            return Err(NewClientError::UnsupportedScheme { url });
        }

        if !url.path().ends_with('/') {
            url.set_path(&format!("{}/", url.path()));
        }

        Ok(Self {
            client: reqwest::blocking::ClientBuilder::default().build()?,
            url,
            api_key: None,
        })
    }

    /// Local API key for endpoints which require authentication.
    pub fn with_api_key(mut self, api_key: ApiKey) -> Self {
        self.api_key = Some(api_key);
        self
    }

    fn send_request(
        &self,
        request: http::Request<Vec<u8>>,
    ) -> Result<http::Response<Bytes>, Error> {
        // https://docs.rs/reqwest/latest/reqwest/blocking/struct.Request.html#method.try_from
        let req = request.try_into()?;

        // execute the query
        let res = self.client.execute(req)?;

        // no try_into from reqwest::blocking::Response for http::Response
        let http_res = into_http_response(res)?;

        Ok(http_res)
    }
}

impl Client for ReqwestClient {
    type Error = Error;

    fn base_endpoint(&self, path: &str) -> Result<Url, ApiError<Self::Error>> {
        self.url.join(path).map_err(|e| ApiError::Url {
            error: e,
            url: self.url.to_string(),
            path: Cow::Owned(path.to_string()),
        })
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
        let request = request_builder
            .body(body)
            .map_err(|error| HttpRequest { error })?;

        self.send_request(request)
            .map_err(|error| ApiError::Client { error })
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
}
