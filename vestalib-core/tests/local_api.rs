use mockito::{Matcher, Server, ServerGuard};
use serde_json::json;
use std::borrow::Cow;
use std::num::NonZeroU32;
use std::time::Duration;
use url::Url;
use vestalib_core::api::local::{
    Enabled, Enablement, Message, ReadMessage, SendMessage, Strategy, Transition,
};
use vestalib_core::api::{ApiError, Json, Query, ResponseBody, Void};
use vestalib_core::board::{Character, Layout, Model};
use vestalib_core::client::ReqwestClient;
use vestalib_core::client::local::{ApiKey, EnablementToken, LocalConnection};

const API_KEY: &str = "local-api-key";
const API_KEY_HEADER: &str = "X-Vestaboard-Local-Api-Key";
const ENABLEMENT_TOKEN_HEADER: &str = "X-Vestaboard-Local-Api-Enablement-Token";

fn client(server: &ServerGuard) -> ReqwestClient<LocalConnection> {
    let connection = LocalConnection::new(Url::parse(&server.url()).unwrap())
        .unwrap()
        .with_api_key(ApiKey::new(API_KEY).unwrap());

    ReqwestClient::new(connection).unwrap()
}

fn hello_world() -> Layout {
    Layout::from_text(Model::Flagship, "\n\n     HELLO WORLD").unwrap()
}

#[test]
fn enable() {
    let mut server = Server::new();
    let mock = server
        .mock("POST", "/local-api/enablement")
        .match_header(ENABLEMENT_TOKEN_HEADER, "enablement-token")
        .match_header(API_KEY_HEADER, Matcher::Missing)
        .match_body("")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{"message":"Local API enabled","apiKey":"XXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX"}"#,
        )
        .create();
    let client =
        ReqwestClient::new(LocalConnection::new(Url::parse(&server.url()).unwrap()).unwrap())
            .unwrap();
    let endpoint = Enablement::new(EnablementToken::new("enablement-token").unwrap());

    let enabled: Enabled = endpoint.query(&client).unwrap();

    mock.assert();
    assert_eq!(enabled.message, "Local API enabled");
    assert_eq!(
        enabled.api_key.expose_secret(),
        "XXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX"
    );
}

#[test]
fn enable_with_used_token() {
    let mut server = Server::new();
    let mock = server
        .mock("POST", "/local-api/enablement")
        .with_status(401)
        .create();
    let client =
        ReqwestClient::new(LocalConnection::new(Url::parse(&server.url()).unwrap()).unwrap())
            .unwrap();
    let endpoint = Enablement::new(EnablementToken::new("used").unwrap());

    let result: Result<Enabled, _> = endpoint.query(&client);

    mock.assert();
    assert!(matches!(
        result,
        Err(ApiError::HttpResponse {
            status_code: http::StatusCode::UNAUTHORIZED,
            body: ResponseBody::Empty,
        })
    ));
}

#[test]
fn read_message() {
    let mut server = Server::new();
    let mock = server
        .mock("GET", "/local-api/message")
        .match_header(API_KEY_HEADER, API_KEY)
        .match_header("accept", "application/json")
        .with_body(serde_json::to_string(&hello_world()).unwrap())
        .create();

    let message: Message = ReadMessage::new().query(&client(&server)).unwrap();

    mock.assert();
    assert_eq!(message.layout(), &hello_world());
    assert_eq!(message.layout().model(), Model::Flagship);
}

#[test]
fn read_wrapped_message_of_note() {
    let note = Layout::from_text(Model::Note, "\n       °").unwrap();
    let mut server = Server::new();
    let mock = server
        .mock("GET", "/local-api/message")
        .with_body(json!({ "message": note }).to_string())
        .create();

    let message: Message = ReadMessage::new().query(&client(&server)).unwrap();

    mock.assert();
    assert_eq!(message.layout().model(), Model::Note);
    assert_eq!(message.layout().rows()[1][7], Character::HEART);
}

#[test]
fn read_message_as_json() {
    let mut server = Server::new();
    let mock = server
        .mock("GET", "/local-api/message")
        .with_body(r#"{"message":[[1,2,3]]}"#)
        .create();

    let value = Json::new(ReadMessage::new())
        .query(&client(&server))
        .unwrap();

    mock.assert();
    assert_eq!(value, json!({ "message": [[1, 2, 3]] }));
}

#[test]
fn read_message_of_unexpected_size() {
    let mut server = Server::new();
    let mock = server
        .mock("GET", "/local-api/message")
        .with_body("[[1,2,3]]")
        .create();

    let result: Result<Message, _> = ReadMessage::new().query(&client(&server));

    mock.assert();
    assert!(matches!(result, Err(ApiError::ParseType { .. })));
}

#[test]
fn send_message() {
    let mut server = Server::new();
    let mock = server
        .mock("POST", "/local-api/message")
        .match_header(API_KEY_HEADER, API_KEY)
        .match_header("content-type", "application/json")
        .match_body(Matcher::Json(serde_json::to_value(hello_world()).unwrap()))
        .with_status(201)
        .create();
    let endpoint = SendMessage::new(Cow::Owned(hello_world())).unwrap();

    Void::new(endpoint).query(&client(&server)).unwrap();

    mock.assert();
}

#[test]
fn send_message_with_transition() {
    let mut server = Server::new();
    let mock = server
        .mock("POST", "/local-api/message")
        .match_header(API_KEY_HEADER, API_KEY)
        .match_body(Matcher::Json(json!({
            "characters": hello_world(),
            "strategy": "random",
            "step_interval_ms": 250,
            "step_size": 4,
        })))
        .with_status(201)
        .create();
    let endpoint = SendMessage::new(Cow::Owned(hello_world()))
        .unwrap()
        .with_transition(
            Transition::new(Strategy::Random)
                .with_step_interval(Duration::from_millis(250))
                .with_step_size(NonZeroU32::new(4).unwrap()),
        );

    Void::new(endpoint).query(&client(&server)).unwrap();

    mock.assert();
}

#[test]
fn missing_api_key_is_an_error_before_sending() {
    let mut server = Server::new();
    let mock = server.mock("GET", "/local-api/message").expect(0).create();
    let client =
        ReqwestClient::new(LocalConnection::new(Url::parse(&server.url()).unwrap()).unwrap())
            .unwrap();

    let result: Result<Message, _> = ReadMessage::new().query(&client);

    mock.assert();
    assert!(matches!(result, Err(ApiError::NoApiKey)));
}

#[yare::parameterized(
    unauthorized = { 401, "Unauthorized", "Unauthorized" },
    json = { 400, r#"{"error":"bad message"}"#, r#"{"error":"bad message"}"# },
    empty = { 500, "", "<empty body>" },
)]
fn unsuccessful_status_is_an_error(status: usize, body: &str, expected: &str) {
    let mut server = Server::new();
    let mock = server
        .mock("POST", "/local-api/message")
        .with_status(status)
        .with_body(body)
        .create();
    let endpoint = SendMessage::new(Cow::Owned(hello_world())).unwrap();

    let result = Void::new(endpoint).query(&client(&server));

    mock.assert();
    match result {
        Err(ApiError::HttpResponse { status_code, body }) => {
            assert_eq!(status_code.as_u16() as usize, status);
            assert_eq!(body.to_string(), expected);
        }
        other => panic!("expected an HTTP response error, got {:?}", other),
    }
}

#[test]
fn base_url_with_path() {
    let mut server = Server::new();
    let mock = server
        .mock("GET", "/board-2/local-api/message")
        .with_body(serde_json::to_string(&hello_world()).unwrap())
        .create();
    let url = Url::parse(&format!("{}/board-2", server.url())).unwrap();
    let connection = LocalConnection::new(url)
        .unwrap()
        .with_api_key(ApiKey::new(API_KEY).unwrap());
    let client = ReqwestClient::new(connection).unwrap();

    let _: Message = ReadMessage::new().query(&client).unwrap();

    mock.assert();
}

#[test]
fn unreachable_board_is_a_client_error() {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let url = Url::parse(&format!("http://{}", listener.local_addr().unwrap())).unwrap();
    drop(listener);
    let connection = LocalConnection::new(url)
        .unwrap()
        .with_api_key(ApiKey::new(API_KEY).unwrap());
    let client = ReqwestClient::new(connection).unwrap();

    let result: Result<Message, _> = ReadMessage::new().query(&client);

    assert!(matches!(result, Err(ApiError::Client { .. })));
}
