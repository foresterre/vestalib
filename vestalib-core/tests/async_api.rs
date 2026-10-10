use mockito::{Matcher, Server, ServerGuard};
use std::borrow::Cow;
use url::Url;
use vestalib_core::api::cloud::{ReadTransition, Transition, TransitionSpeed, TransitionType};
use vestalib_core::api::local::{Message, ReadMessage, SendMessage};
use vestalib_core::api::{ApiError, AsyncQuery, Json, ResponseBody, Void};
use vestalib_core::board::{Layout, Model};
use vestalib_core::client::ReqwestAsyncClient;
use vestalib_core::client::cloud::{CloudConnection, CloudToken};
use vestalib_core::client::local::{ApiKey, LocalConnection};

const API_KEY: &str = "local-api-key";
const API_KEY_HEADER: &str = "X-Vestaboard-Local-Api-Key";
const TOKEN: &str = "cloud-api-token";
const TOKEN_HEADER: &str = "X-Vestaboard-Token";

fn local_client(server: &ServerGuard) -> ReqwestAsyncClient<LocalConnection> {
    let connection = LocalConnection::new(Url::parse(&server.url()).unwrap())
        .unwrap()
        .with_api_key(ApiKey::new(API_KEY).unwrap());

    ReqwestAsyncClient::new(connection).unwrap()
}

fn cloud_client(server: &ServerGuard) -> ReqwestAsyncClient<CloudConnection> {
    let connection = CloudConnection::new(Url::parse(&server.url()).unwrap())
        .unwrap()
        .with_token(CloudToken::new(TOKEN).unwrap());

    ReqwestAsyncClient::new(connection).unwrap()
}

fn hello_world() -> Layout {
    Layout::from_text(Model::Flagship, "\n\n     HELLO WORLD").unwrap()
}

#[tokio::test]
async fn read_message() {
    let mut server = Server::new_async().await;
    let mock = server
        .mock("GET", "/local-api/message")
        .match_header(API_KEY_HEADER, API_KEY)
        .match_header("accept", "application/json")
        .with_body(serde_json::to_string(&hello_world()).unwrap())
        .create_async()
        .await;

    let message: Message = ReadMessage::new()
        .query_async(&local_client(&server))
        .await
        .unwrap();

    mock.assert_async().await;
    assert_eq!(message.layout(), &hello_world());
}

#[tokio::test]
async fn read_message_as_json() {
    let mut server = Server::new_async().await;
    let mock = server
        .mock("GET", "/local-api/message")
        .with_body(r#"{"message":[[1,2,3]]}"#)
        .create_async()
        .await;

    let value = Json::new(ReadMessage::new())
        .query_async(&local_client(&server))
        .await
        .unwrap();

    mock.assert_async().await;
    assert_eq!(value, serde_json::json!({ "message": [[1, 2, 3]] }));
}

#[tokio::test]
async fn send_message() {
    let mut server = Server::new_async().await;
    let mock = server
        .mock("POST", "/local-api/message")
        .match_header(API_KEY_HEADER, API_KEY)
        .match_header("content-type", "application/json")
        .match_body(Matcher::Json(serde_json::to_value(hello_world()).unwrap()))
        .with_status(201)
        .create_async()
        .await;
    let endpoint = SendMessage::new(Cow::Owned(hello_world())).unwrap();

    Void::new(endpoint)
        .query_async(&local_client(&server))
        .await
        .unwrap();

    mock.assert_async().await;
}

#[tokio::test]
async fn read_transition() {
    let mut server = Server::new_async().await;
    let mock = server
        .mock("GET", "/transition")
        .match_header(TOKEN_HEADER, TOKEN)
        .with_body(r#"{"transition":"wave","transitionSpeed":"gentle"}"#)
        .create_async()
        .await;

    let transition: Transition = ReadTransition::new()
        .query_async(&cloud_client(&server))
        .await
        .unwrap();

    mock.assert_async().await;
    assert_eq!(
        transition,
        Transition::new(TransitionType::Wave, TransitionSpeed::Gentle)
    );
}

#[tokio::test]
async fn missing_api_key_is_an_error_before_sending() {
    let mut server = Server::new_async().await;
    let mock = server
        .mock("GET", Matcher::Any)
        .expect(0)
        .create_async()
        .await;
    let connection = LocalConnection::new(Url::parse(&server.url()).unwrap()).unwrap();
    let client = ReqwestAsyncClient::new(connection).unwrap();

    let result: Result<Message, _> = ReadMessage::new().query_async(&client).await;

    mock.assert_async().await;
    assert!(matches!(result, Err(ApiError::NoApiKey)));
}

#[tokio::test]
async fn unsuccessful_status_is_an_error() {
    let mut server = Server::new_async().await;
    let mock = server
        .mock("GET", "/transition")
        .with_status(401)
        .with_body("Unauthorized")
        .create_async()
        .await;

    let result: Result<Transition, _> = ReadTransition::new()
        .query_async(&cloud_client(&server))
        .await;

    mock.assert_async().await;
    match result {
        Err(ApiError::HttpResponse {
            status_code,
            body: ResponseBody::Text(body),
        }) => {
            assert_eq!(status_code, http::StatusCode::UNAUTHORIZED);
            assert_eq!(body, "Unauthorized");
        }
        other => panic!("expected an HTTP response error, got {:?}", other),
    }
}

#[tokio::test]
async fn query_runs_on_a_spawned_task() {
    let mut server = Server::new_async().await;
    let mock = server
        .mock("GET", "/local-api/message")
        .with_body(serde_json::to_string(&hello_world()).unwrap())
        .create_async()
        .await;
    let client = local_client(&server);

    let message: Message =
        tokio::spawn(async move { ReadMessage::new().query_async(&client).await })
            .await
            .unwrap()
            .unwrap();

    mock.assert_async().await;
    assert_eq!(message.layout(), &hello_world());
}
