pub mod local;

use crate::client::{ENABLEMENT_TOKEN_HEADER, EnablementToken};
use bytes::Bytes;
use http::StatusCode;
use std::borrow::Cow;
use std::fmt;
use url::Url;

#[derive(fmt::Debug, thiserror::Error)]
#[error(transparent)]
#[non_exhaustive]
pub struct BodyError {
    pub error: serde_json::Error,
}

/// The body of a request, together with its content type.
#[derive(Clone, fmt::Debug, PartialEq, Eq)]
pub struct Body {
    content_type: &'static str,
    bytes: Vec<u8>,
}

impl Body {
    pub fn new(content_type: &'static str, bytes: Vec<u8>) -> Self {
        Self {
            content_type,
            bytes,
        }
    }

    pub fn json<T: serde::Serialize + ?Sized>(value: &T) -> Result<Self, BodyError> {
        serde_json::to_vec(value)
            .map(|bytes| Self::new("application/json", bytes))
            .map_err(|error| BodyError { error })
    }

    pub fn content_type(&self) -> &'static str {
        self.content_type
    }

    pub fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }
}

/// How an endpoint must be authenticated.
#[derive(Clone, Copy, fmt::Debug, Default, PartialEq, Eq)]
pub enum Authentication<'a> {
    #[default]
    None,
    ApiKey,
    EnablementToken(&'a EnablementToken),
}

#[derive(fmt::Debug)]
#[non_exhaustive]
pub enum ResponseBody {
    Empty,
    Json(serde_json::Value),
    Text(String),
}

impl From<&[u8]> for ResponseBody {
    fn from(bytes: &[u8]) -> Self {
        if bytes.iter().all(u8::is_ascii_whitespace) {
            return ResponseBody::Empty;
        }

        match serde_json::from_slice(bytes) {
            Ok(value) => ResponseBody::Json(value),
            Err(_) => ResponseBody::Text(String::from_utf8_lossy(bytes).into_owned()),
        }
    }
}

impl fmt::Display for ResponseBody {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ResponseBody::Empty => f.write_str("<empty body>"),
            ResponseBody::Json(value) => write!(f, "{}", value),
            ResponseBody::Text(text) => f.write_str(text),
        }
    }
}

#[derive(fmt::Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ApiError<C: std::error::Error + Send + Sync + 'static> {
    #[error("Body error: '{}'", error)]
    Body {
        #[from]
        error: BodyError,
    },
    #[error("Client error: '{}'", error)]
    Client { error: C },
    #[error("No Local API key found")]
    NoApiKey,
    #[error("Unable to build HTTP request: '{}'", error)]
    HttpRequest { error: http::Error },
    #[error("HTTP request failed with status code '{}': '{}'", status_code, body)]
    HttpResponse {
        status_code: StatusCode,
        body: ResponseBody,
    },
    #[error("Unable to parse JSON response into type '{}': '{}'", r#type, error)]
    ParseType {
        error: serde_json::Error,
        r#type: &'static str,
    },
    #[error("Unable to parse url '{}' (path '{}'): '{}'", url, path, error)]
    Url {
        error: url::ParseError,
        url: String,
        path: Cow<'static, str>,
    },
}

impl<C: std::error::Error + Send + Sync + 'static> ApiError<C> {
    pub fn parse_type_error<T>(error: serde_json::Error) -> Self {
        ApiError::ParseType {
            error,
            r#type: std::any::type_name::<T>(),
        }
    }
}

/// Http endpoint trait for vestalib.
///
/// # Credits
///
/// Inspired by Ben Boeckel's blog [post](https://plume.benboeckel.net/~/JustAnotherBlog/designing-rust-bindings-for-rest-ap-is)
/// titled "Designing Rust bindings for REST APIs".
pub trait Endpoint {
    fn method(&self) -> http::Method;

    fn endpoint(&self) -> Cow<'static, str>;

    fn body(&self) -> Result<Option<Body>, BodyError> {
        Ok(None)
    }

    fn authentication(&self) -> Authentication<'_> {
        Authentication::None
    }
}

/// HTTP api trait for vestalib.
pub trait Client {
    type Error: std::error::Error + Send + Sync + 'static;

    fn base_endpoint(&self, path: &str) -> Result<Url, ApiError<Self::Error>>;

    /// Adds the Local API key to a request of an endpoint which requires it.
    ///
    /// Returns [`ApiError::NoApiKey`] when the client has no Local API key.
    fn authorize(
        &self,
        request_builder: http::request::Builder,
    ) -> Result<http::request::Builder, ApiError<Self::Error>>;

    // By separating the request builder and the body, additional items may be added
    // to the request, such as authentication.
    fn send(
        &self,
        request_builder: http::request::Builder,
        body: Vec<u8>,
    ) -> Result<http::Response<Bytes>, ApiError<Self::Error>>;
}

/// Query trait for vestalib.
///
/// # Credits
///
/// Inspired by Ben Boeckel's blog [post](https://plume.benboeckel.net/~/JustAnotherBlog/designing-rust-bindings-for-rest-ap-is)
/// titled "Designing Rust bindings for REST APIs".
pub trait Query<T, C: Client> {
    fn query(&self, client: &C) -> Result<T, ApiError<C::Error>>;
}

impl<E> Endpoint for &E
where
    E: Endpoint,
{
    fn method(&self) -> http::Method {
        (*self).method()
    }

    fn endpoint(&self) -> Cow<'static, str> {
        (*self).endpoint()
    }

    fn body(&self) -> Result<Option<Body>, BodyError> {
        (*self).body()
    }

    fn authentication(&self) -> Authentication<'_> {
        (*self).authentication()
    }
}

fn send<E, C>(endpoint: &E, client: &C) -> Result<http::Response<Bytes>, ApiError<C::Error>>
where
    E: Endpoint,
    C: Client,
{
    let url = client.base_endpoint(endpoint.endpoint().as_ref())?;

    // build the request
    let body = endpoint.body()?;
    let mut request = http::Request::builder()
        .method(endpoint.method())
        .uri(url.as_str())
        .header(http::header::ACCEPT, "application/json");

    if let Some(body) = &body {
        request = request.header(http::header::CONTENT_TYPE, body.content_type());
    }

    let request = match endpoint.authentication() {
        Authentication::None => request,
        Authentication::ApiKey => client.authorize(request)?,
        Authentication::EnablementToken(token) => {
            request.header(ENABLEMENT_TOKEN_HEADER, token.as_header_value().clone())
        }
    };

    // send
    let body = body.map(Body::into_bytes).unwrap_or_default();
    let response = client.send(request, body)?;

    // handle response errors
    if !response.status().is_success() {
        return Err(ApiError::HttpResponse {
            status_code: response.status(),
            body: ResponseBody::from(response.body().as_ref()),
        });
    }

    Ok(response)
}

impl<E, T, C> Query<T, C> for E
where
    E: Endpoint,
    T: serde::de::DeserializeOwned,
    C: Client,
{
    fn query(&self, client: &C) -> Result<T, ApiError<C::Error>> {
        let response = send(self, client)?;

        serde_json::from_slice::<T>(response.body()).map_err(ApiError::parse_type_error::<T>)
    }
}

/// Run the endpoint, return response as JSON
pub struct Json<E> {
    endpoint: E,
}

impl<E> Json<E> {
    pub fn new(endpoint: E) -> Self {
        Self { endpoint }
    }
}

impl<E, C> Query<serde_json::Value, C> for Json<E>
where
    E: Endpoint,
    C: Client,
{
    fn query(&self, client: &C) -> Result<serde_json::Value, ApiError<C::Error>> {
        self.endpoint.query(client)
    }
}

/// Run the endpoint, but discard the response
pub struct Void<E> {
    endpoint: E,
}

impl<E> Void<E> {
    pub fn new(endpoint: E) -> Self {
        Self { endpoint }
    }
}

impl<E, C> Query<(), C> for Void<E>
where
    E: Endpoint,
    C: Client,
{
    fn query(&self, client: &C) -> Result<(), ApiError<C::Error>> {
        send(&self.endpoint, client).map(|_| ())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[yare::parameterized(
        empty = { b"", "<empty body>" },
        whitespace = { b" \n", "<empty body>" },
        json = { br#"{"error":"nope"}"#, r#"{"error":"nope"}"# },
        text = { b"Unauthorized", "Unauthorized" },
    )]
    fn response_body(bytes: &[u8], expected: &str) {
        assert_eq!(ResponseBody::from(bytes).to_string(), expected);
    }
}
