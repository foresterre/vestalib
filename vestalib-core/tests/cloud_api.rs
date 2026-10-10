use mockito::{Matcher, Server, ServerGuard};
use serde_json::json;
use std::borrow::Cow;
use std::time::{Duration, UNIX_EPOCH};
use url::Url;
use vestalib_core::api::cloud::{
    FormatMessage, Message, ReadMessage, ReadTransition, SendMessage, Sent, SetTransition,
    Transition, TransitionSpeed, TransitionType,
};
use vestalib_core::api::local;
use vestalib_core::api::{ApiError, Json, Query, ResponseBody};
use vestalib_core::board::{Layout, Model, NoteArray};
use vestalib_core::client::{CloudToken, ReqwestCloudClient};

const TOKEN: &str = "cloud-api-token";
const TOKEN_HEADER: &str = "X-Vestaboard-Token";
const SENT: &str =
    r#"{"status":"ok","id":"9b4f13f0-4623-4fb9-8fbf-09ae13350da1","created":1681154452865}"#;

fn client(server: &ServerGuard) -> ReqwestCloudClient {
    ReqwestCloudClient::new(Url::parse(&server.url()).unwrap())
        .unwrap()
        .with_token(CloudToken::new(TOKEN).unwrap())
}

fn hello_world() -> Layout {
    Layout::from_text(Model::Flagship, "\n\n     HELLO WORLD").unwrap()
}

#[test]
fn read_message() {
    let mut server = Server::new();
    let mock = server
        .mock("GET", "/")
        .match_header(TOKEN_HEADER, TOKEN)
        .match_header("accept", "application/json")
        .match_header("X-Vestaboard-Local-Api-Key", Matcher::Missing)
        .with_body(
            json!({
                "currentMessage": {
                    "layout": serde_json::to_string(&hello_world()).unwrap(),
                    "id": "172c7808-52db-4eb9-9ecf-505bd00436bc",
                }
            })
            .to_string(),
        )
        .create();

    let message: Message = ReadMessage::new().query(&client(&server)).unwrap();

    mock.assert();
    assert_eq!(message.id(), "172c7808-52db-4eb9-9ecf-505bd00436bc");
    assert_eq!(message.layout(), &hello_world());
}

#[test]
fn read_message_of_note_array() {
    let layout = Layout::blank(Model::NoteArray(NoteArray::Grid2x2));
    let mut server = Server::new();
    let mock = server
        .mock("GET", "/")
        .with_body(
            json!({
                "currentMessage": { "layout": serde_json::to_string(&layout).unwrap(), "id": "id" }
            })
            .to_string(),
        )
        .create();

    let message: Message = ReadMessage::new().query(&client(&server)).unwrap();

    mock.assert();
    assert_eq!(
        message.layout().model(),
        Model::NoteArray(NoteArray::Grid2x2)
    );
}

#[test]
fn send_text() {
    let mut server = Server::new();
    let mock = server
        .mock("POST", "/")
        .match_header(TOKEN_HEADER, TOKEN)
        .match_header("content-type", "application/json")
        .match_body(Matcher::Json(
            json!({ "text": "Hello World", "forced": true }),
        ))
        .with_body(SENT)
        .create();
    let endpoint = SendMessage::text(Cow::Borrowed("Hello World"))
        .unwrap()
        .with_forced(true);

    let sent: Sent = endpoint.query(&client(&server)).unwrap();

    mock.assert();
    assert_eq!(sent.id, "9b4f13f0-4623-4fb9-8fbf-09ae13350da1");
    assert_eq!(
        sent.created,
        UNIX_EPOCH + Duration::from_millis(1681154452865)
    );
}

#[test]
fn send_characters() {
    let mut server = Server::new();
    let mock = server
        .mock("POST", "/")
        .match_header(TOKEN_HEADER, TOKEN)
        .match_body(Matcher::Json(json!({ "characters": hello_world() })))
        .with_body(SENT)
        .create();
    let endpoint = SendMessage::characters(Cow::Owned(hello_world())).unwrap();

    let _: Sent = endpoint.query(&client(&server)).unwrap();

    mock.assert();
}

#[test]
fn read_transition() {
    let mut server = Server::new();
    let mock = server
        .mock("GET", "/transition")
        .match_header(TOKEN_HEADER, TOKEN)
        .with_body(r#"{"transition":"wave","transitionSpeed":"gentle"}"#)
        .create();

    let transition: Transition = ReadTransition::new().query(&client(&server)).unwrap();

    mock.assert();
    assert_eq!(
        transition,
        Transition::new(TransitionType::Wave, TransitionSpeed::Gentle)
    );
}

#[test]
fn set_transition() {
    let mut server = Server::new();
    let mock = server
        .mock("PUT", "/transition")
        .match_header(TOKEN_HEADER, TOKEN)
        .match_body(Matcher::Json(
            json!({ "transition": "drift", "transitionSpeed": "fast" }),
        ))
        .with_body(r#"{"transition":"drift","transitionSpeed":"fast"}"#)
        .create();
    let transition = Transition::new(TransitionType::Drift, TransitionSpeed::Fast);

    let updated: Transition = SetTransition::new(transition)
        .query(&client(&server))
        .unwrap();

    mock.assert();
    assert_eq!(updated, transition);
}

#[test]
fn format_message_without_token() {
    let mut server = Server::new();
    let mock = server
        .mock("POST", "/vbml/format")
        .match_header(TOKEN_HEADER, Matcher::Missing)
        .match_body(Matcher::Json(
            json!({ "message": "Hello World", "rows": 6, "columns": 22 }),
        ))
        .with_body(serde_json::to_string(&hello_world()).unwrap())
        .create();
    let client = ReqwestCloudClient::new(Url::parse(&server.url()).unwrap()).unwrap();

    let layout: Layout = FormatMessage::new(Cow::Borrowed("Hello World"), Model::Flagship)
        .query(&client)
        .unwrap();

    mock.assert();
    assert_eq!(layout, hello_world());
}

#[test]
fn read_message_as_json() {
    let mut server = Server::new();
    let mock = server
        .mock("GET", "/")
        .with_body(r#"{"currentMessage":{"layout":"[[1]]","id":"id"}}"#)
        .create();

    let value = Json::new(ReadMessage::new())
        .query(&client(&server))
        .unwrap();

    mock.assert();
    assert_eq!(value["currentMessage"]["layout"], "[[1]]");
}

#[test]
fn missing_token_is_an_error_before_sending() {
    let mut server = Server::new();
    let mock = server.mock("GET", Matcher::Any).expect(0).create();
    let client = ReqwestCloudClient::new(Url::parse(&server.url()).unwrap()).unwrap();

    let result: Result<Message, _> = ReadMessage::new().query(&client);

    mock.assert();
    assert!(matches!(result, Err(ApiError::NoCloudToken)));
}

#[test]
fn local_endpoint_is_an_error_before_sending() {
    let mut server = Server::new();
    let mock = server.mock("GET", Matcher::Any).expect(0).create();

    let result: Result<local::Message, _> = local::ReadMessage::new().query(&client(&server));

    mock.assert();
    assert!(matches!(result, Err(ApiError::NoApiKey)));
}

#[test]
fn unauthorized_is_an_error() {
    let mut server = Server::new();
    let mock = server
        .mock("GET", "/transition")
        .with_status(401)
        .with_body("Unauthorized")
        .create();

    let result: Result<Transition, _> = ReadTransition::new().query(&client(&server));

    mock.assert();
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
