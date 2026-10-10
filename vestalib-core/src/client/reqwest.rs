use crate::api::ApiError::HttpRequest;
use crate::api::{Api, ApiError, AsyncClient, Client, Connection, MissingCredential};
use bytes::Bytes;
use http::request::Builder;
use http::{HeaderMap, StatusCode, Version};
use reqwest::ResponseBuilderExt;
use url::Url;

pub struct ReqwestClient<C> {
    client: reqwest::blocking::Client,
    connection: C,
}

impl<C: Connection> ReqwestClient<C> {
    pub fn new(connection: C) -> Result<Self, NewClientError> {
        Ok(Self {
            client: reqwest::blocking::ClientBuilder::default().build()?,
            connection,
        })
    }
}

impl<C: Connection> Connection for ReqwestClient<C> {
    type Api = C::Api;

    fn base_url(&self) -> &Url {
        self.connection.base_url()
    }

    fn authorize(
        &self,
        request_builder: Builder,
        authentication: <Self::Api as Api>::Authentication<'_>,
    ) -> Result<Builder, MissingCredential> {
        self.connection.authorize(request_builder, authentication)
    }
}

impl<C: Connection> Client for ReqwestClient<C> {
    type Error = Error;

    fn send(
        &self,
        request_builder: Builder,
        body: Vec<u8>,
    ) -> Result<http::Response<Bytes>, ApiError<Self::Error>> {
        let request = request_builder
            .body(body)
            .map_err(|error| HttpRequest { error })?;

        send_request(&self.client, request).map_err(|error| ApiError::Client { error })
    }
}

pub struct ReqwestAsyncClient<C> {
    client: reqwest::Client,
    connection: C,
}

impl<C: Connection> ReqwestAsyncClient<C> {
    pub fn new(connection: C) -> Result<Self, NewClientError> {
        Ok(Self {
            client: reqwest::ClientBuilder::default().build()?,
            connection,
        })
    }
}

impl<C: Connection> Connection for ReqwestAsyncClient<C> {
    type Api = C::Api;

    fn base_url(&self) -> &Url {
        self.connection.base_url()
    }

    fn authorize(
        &self,
        request_builder: Builder,
        authentication: <Self::Api as Api>::Authentication<'_>,
    ) -> Result<Builder, MissingCredential> {
        self.connection.authorize(request_builder, authentication)
    }
}

impl<C: Connection + Sync> AsyncClient for ReqwestAsyncClient<C> {
    type Error = Error;

    async fn send_async(
        &self,
        request_builder: Builder,
        body: Vec<u8>,
    ) -> Result<http::Response<Bytes>, ApiError<Self::Error>> {
        let request = request_builder
            .body(body)
            .map_err(|error| HttpRequest { error })?;

        send_request_async(&self.client, request)
            .await
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
    let builder = response_builder(res.status(), res.version(), res.url(), res.headers());
    let body = res.bytes()?;
    let response = builder.body(body).unwrap();

    Ok(response)
}

async fn send_request_async(
    client: &reqwest::Client,
    request: http::Request<Vec<u8>>,
) -> Result<http::Response<Bytes>, Error> {
    let req = request.try_into()?;

    let res = client.execute(req).await?;

    let builder = response_builder(res.status(), res.version(), res.url(), res.headers());
    let body = res.bytes().await?;
    let response = builder.body(body).unwrap();

    Ok(response)
}

fn response_builder(
    status: StatusCode,
    version: Version,
    url: &Url,
    headers: &HeaderMap,
) -> http::response::Builder {
    let mut builder = http::response::Builder::new()
        .status(status)
        .version(version)
        .url(url.clone());

    for (name, value) in headers {
        builder = builder.header(name, value);
    }

    builder
}
