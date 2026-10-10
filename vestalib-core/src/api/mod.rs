pub mod cloud;
pub mod local;

use bytes::Bytes;
use http::StatusCode;
use std::borrow::Cow;
use std::fmt;
use std::future::Future;
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

pub trait Api {
    type Authentication<'a>: Default;
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
    #[error("No Cloud API token found")]
    NoCloudToken,
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

#[derive(Clone, Copy, fmt::Debug, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum MissingCredential {
    #[error("No Local API key found")]
    ApiKey,
    #[error("No Cloud API token found")]
    CloudToken,
}

impl<C: std::error::Error + Send + Sync + 'static> From<MissingCredential> for ApiError<C> {
    fn from(missing: MissingCredential) -> Self {
        match missing {
            MissingCredential::ApiKey => ApiError::NoApiKey,
            MissingCredential::CloudToken => ApiError::NoCloudToken,
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
    type Api: Api;

    fn method(&self) -> http::Method;

    fn endpoint(&self) -> Cow<'static, str>;

    fn body(&self) -> Result<Option<Body>, BodyError> {
        Ok(None)
    }

    fn authentication(&self) -> <Self::Api as Api>::Authentication<'_> {
        Default::default()
    }
}

pub trait Connection {
    type Api: Api;

    fn base_url(&self) -> &Url;

    fn authorize(
        &self,
        request_builder: http::request::Builder,
        authentication: <Self::Api as Api>::Authentication<'_>,
    ) -> Result<http::request::Builder, MissingCredential>;
}

/// HTTP api trait for vestalib.
pub trait Client: Connection {
    type Error: std::error::Error + Send + Sync + 'static;

    // By separating the request builder and the body, additional items may be added
    // to the request, such as authentication.
    fn send(
        &self,
        request_builder: http::request::Builder,
        body: Vec<u8>,
    ) -> Result<http::Response<Bytes>, ApiError<Self::Error>>;
}

pub trait AsyncClient: Connection {
    type Error: std::error::Error + Send + Sync + 'static;

    fn send_async(
        &self,
        request_builder: http::request::Builder,
        body: Vec<u8>,
    ) -> impl Future<Output = Result<http::Response<Bytes>, ApiError<Self::Error>>> + Send;
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

pub trait AsyncQuery<T, C: AsyncClient> {
    fn query_async(&self, client: &C)
    -> impl Future<Output = Result<T, ApiError<C::Error>>> + Send;
}

impl<E> Endpoint for &E
where
    E: Endpoint,
{
    type Api = E::Api;

    fn method(&self) -> http::Method {
        (*self).method()
    }

    fn endpoint(&self) -> Cow<'static, str> {
        (*self).endpoint()
    }

    fn body(&self) -> Result<Option<Body>, BodyError> {
        (*self).body()
    }

    fn authentication(&self) -> <Self::Api as Api>::Authentication<'_> {
        (*self).authentication()
    }
}

fn request<Err, E, C>(
    endpoint: &E,
    connection: &C,
) -> Result<(http::request::Builder, Vec<u8>), ApiError<Err>>
where
    Err: std::error::Error + Send + Sync + 'static,
    E: Endpoint,
    C: Connection<Api = E::Api>,
{
    let base_url = connection.base_url();
    let path = endpoint.endpoint();
    let url = base_url.join(&path).map_err(|error| ApiError::Url {
        error,
        url: base_url.to_string(),
        path,
    })?;

    // build the request
    let body = endpoint.body()?;
    let mut request = http::Request::builder()
        .method(endpoint.method())
        .uri(url.as_str())
        .header(http::header::ACCEPT, "application/json");

    if let Some(body) = &body {
        request = request.header(http::header::CONTENT_TYPE, body.content_type());
    }

    let request = connection.authorize(request, endpoint.authentication())?;

    Ok((request, body.map(Body::into_bytes).unwrap_or_default()))
}

fn check_status<Err>(
    response: http::Response<Bytes>,
) -> Result<http::Response<Bytes>, ApiError<Err>>
where
    Err: std::error::Error + Send + Sync + 'static,
{
    if !response.status().is_success() {
        return Err(ApiError::HttpResponse {
            status_code: response.status(),
            body: ResponseBody::from(response.body().as_ref()),
        });
    }

    Ok(response)
}

fn send<E, C>(endpoint: &E, client: &C) -> Result<http::Response<Bytes>, ApiError<C::Error>>
where
    E: Endpoint,
    C: Client<Api = E::Api>,
{
    let (request, body) = request::<C::Error, _, _>(endpoint, client)?;

    check_status(client.send(request, body)?)
}

async fn send_async<E, C>(
    endpoint: &E,
    client: &C,
) -> Result<http::Response<Bytes>, ApiError<C::Error>>
where
    E: Endpoint,
    C: AsyncClient<Api = E::Api>,
{
    let (request, body) = request::<C::Error, _, _>(endpoint, client)?;

    check_status(client.send_async(request, body).await?)
}

impl<E, T, C> Query<T, C> for E
where
    E: Endpoint,
    T: serde::de::DeserializeOwned,
    C: Client<Api = E::Api>,
{
    fn query(&self, client: &C) -> Result<T, ApiError<C::Error>> {
        let response = send(self, client)?;

        serde_json::from_slice::<T>(response.body()).map_err(ApiError::parse_type_error::<T>)
    }
}

impl<E, T, C> AsyncQuery<T, C> for E
where
    E: Endpoint + Sync,
    T: serde::de::DeserializeOwned,
    C: AsyncClient<Api = E::Api> + Sync,
{
    async fn query_async(&self, client: &C) -> Result<T, ApiError<C::Error>> {
        let response = send_async(self, client).await?;

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
    C: Client<Api = E::Api>,
{
    fn query(&self, client: &C) -> Result<serde_json::Value, ApiError<C::Error>> {
        self.endpoint.query(client)
    }
}

impl<E, C> AsyncQuery<serde_json::Value, C> for Json<E>
where
    E: Endpoint + Sync,
    C: AsyncClient<Api = E::Api> + Sync,
{
    async fn query_async(&self, client: &C) -> Result<serde_json::Value, ApiError<C::Error>> {
        self.endpoint.query_async(client).await
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
    C: Client<Api = E::Api>,
{
    fn query(&self, client: &C) -> Result<(), ApiError<C::Error>> {
        send(&self.endpoint, client).map(|_| ())
    }
}

impl<E, C> AsyncQuery<(), C> for Void<E>
where
    E: Endpoint + Sync,
    C: AsyncClient<Api = E::Api> + Sync,
{
    async fn query_async(&self, client: &C) -> Result<(), ApiError<C::Error>> {
        send_async(&self.endpoint, client).await.map(|_| ())
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
