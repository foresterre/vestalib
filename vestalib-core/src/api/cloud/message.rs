use crate::api::cloud::{Authentication, Cloud};
use crate::api::{Body, BodyError, Endpoint};
use crate::board::{Character, Layout};
use serde::de::Error as _;
use std::borrow::Cow;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// API to read the message which is currently shown.
#[derive(Clone, Debug, Default)]
pub struct ReadMessage;

impl ReadMessage {
    pub fn new() -> Self {
        Self
    }
}

impl Endpoint for ReadMessage {
    type Api = Cloud;

    fn method(&self) -> http::Method {
        http::Method::GET
    }

    fn endpoint(&self) -> Cow<'static, str> {
        Cow::Borrowed("")
    }

    fn authentication(&self) -> Authentication {
        Authentication::CloudToken
    }
}

/// The message which is currently shown.
#[derive(Clone, Debug, PartialEq, Eq, serde::Deserialize)]
#[serde(try_from = "MessageResponse")]
pub struct Message {
    id: String,
    layout: Layout,
}

impl Message {
    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn layout(&self) -> &Layout {
        &self.layout
    }

    pub fn into_layout(self) -> Layout {
        self.layout
    }
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct MessageResponse {
    current_message: CurrentMessage,
}

#[derive(serde::Deserialize)]
struct CurrentMessage {
    id: String,
    layout: String,
}

impl TryFrom<MessageResponse> for Message {
    type Error = serde_json::Error;

    fn try_from(response: MessageResponse) -> Result<Self, Self::Error> {
        let CurrentMessage { id, layout } = response.current_message;

        Ok(Self {
            id,
            layout: serde_json::from_str(&layout)?,
        })
    }
}

/// API to show a message.
///
/// Vestaboard drops messages when more than one message is sent every 15 seconds.
#[derive(Clone, Debug)]
pub struct SendMessage<'a> {
    content: Content<'a>,
    forced: bool,
}

#[derive(Clone, Debug)]
enum Content<'a> {
    Text(Cow<'a, str>),
    Characters(Cow<'a, Layout>),
}

impl<'a> SendMessage<'a> {
    /// Vestaboard formats the text to fit the board.
    pub fn text(text: Cow<'a, str>) -> Result<Self, BlankMessage> {
        if text.trim().is_empty() {
            return Err(BlankMessage);
        }

        Ok(Self {
            content: Content::Text(text),
            forced: false,
        })
    }

    pub fn characters(layout: Cow<'a, Layout>) -> Result<Self, BlankMessage> {
        if layout
            .rows()
            .iter()
            .flatten()
            .all(|cell| *cell == Character::Blank)
        {
            return Err(BlankMessage);
        }

        Ok(Self {
            content: Content::Characters(layout),
            forced: false,
        })
    }

    /// Show the message, even during the quiet hours set in the Vestaboard app.
    pub fn with_forced(mut self, forced: bool) -> Self {
        self.forced = forced;
        self
    }
}

impl<'a> Endpoint for SendMessage<'a> {
    type Api = Cloud;

    fn method(&self) -> http::Method {
        http::Method::POST
    }

    fn endpoint(&self) -> Cow<'static, str> {
        Cow::Borrowed("")
    }

    fn body(&self) -> Result<Option<Body>, BodyError> {
        #[derive(serde::Serialize)]
        struct MessageBody<'b> {
            #[serde(skip_serializing_if = "Option::is_none")]
            text: Option<&'b str>,
            #[serde(skip_serializing_if = "Option::is_none")]
            characters: Option<&'b Layout>,
            #[serde(skip_serializing_if = "std::ops::Not::not")]
            forced: bool,
        }

        let (text, characters) = match &self.content {
            Content::Text(text) => (Some(text.as_ref()), None),
            Content::Characters(layout) => (None, Some(layout.as_ref())),
        };

        Body::json(&MessageBody {
            text,
            characters,
            forced: self.forced,
        })
        .map(Some)
    }

    fn authentication(&self) -> Authentication {
        Authentication::CloudToken
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
#[error("The Cloud API doesn't accept blank messages")]
pub struct BlankMessage;

/// The response to a message which was sent.
#[derive(Clone, Debug, serde::Deserialize)]
#[non_exhaustive]
pub struct Sent {
    pub status: String,
    pub id: String,
    #[serde(deserialize_with = "from_unix_millis")]
    pub created: SystemTime,
}

fn from_unix_millis<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<SystemTime, D::Error> {
    let millis = <u64 as serde::Deserialize>::deserialize(deserializer)?;

    UNIX_EPOCH
        .checked_add(Duration::from_millis(millis))
        .ok_or_else(|| D::Error::custom(format!("timestamp '{}' is out of range", millis)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::{Model, NoteArray};
    use serde_json::json;

    fn note() -> Layout {
        Layout::from_text(Model::Note, "HI").unwrap()
    }

    fn body(endpoint: &SendMessage) -> serde_json::Value {
        let body = endpoint.body().unwrap().unwrap();
        assert_eq!(body.content_type(), "application/json");

        serde_json::from_slice(&body.into_bytes()).unwrap()
    }

    #[test]
    fn text_body() {
        let endpoint = SendMessage::text(Cow::Borrowed("Hello World")).unwrap();

        assert_eq!(body(&endpoint), json!({ "text": "Hello World" }));
    }

    #[test]
    fn characters_body() {
        let endpoint = SendMessage::characters(Cow::Owned(note())).unwrap();

        assert_eq!(body(&endpoint), json!({ "characters": note() }));
    }

    #[test]
    fn forced_body() {
        let endpoint = SendMessage::text(Cow::Borrowed("Hello World"))
            .unwrap()
            .with_forced(true);

        assert_eq!(
            body(&endpoint),
            json!({ "text": "Hello World", "forced": true })
        );
    }

    #[test]
    fn unforced_body() {
        let endpoint = SendMessage::characters(Cow::Owned(note()))
            .unwrap()
            .with_forced(true)
            .with_forced(false);

        assert_eq!(body(&endpoint), json!({ "characters": note() }));
    }

    #[yare::parameterized(
        empty = { "" },
        whitespace = { " \n\t" },
    )]
    fn blank_text_is_rejected(text: &str) {
        assert_eq!(
            SendMessage::text(Cow::Borrowed(text)).unwrap_err(),
            BlankMessage
        );
    }

    #[yare::parameterized(
        flagship = { Model::Flagship },
        note = { Model::Note },
        note_array = { Model::NoteArray(NoteArray::Grid2x2) },
    )]
    fn blank_characters_are_rejected(model: Model) {
        assert_eq!(
            SendMessage::characters(Cow::Owned(Layout::blank(model))).unwrap_err(),
            BlankMessage
        );
    }

    #[test]
    fn deserialize_message() {
        let json = json!({
            "currentMessage": {
                "layout": serde_json::to_string(&note()).unwrap(),
                "id": "172c7808-52db-4eb9-9ecf-505bd00436bc",
            }
        });

        let message: Message = serde_json::from_value(json).unwrap();

        assert_eq!(message.id(), "172c7808-52db-4eb9-9ecf-505bd00436bc");
        assert_eq!(message.into_layout(), note());
    }

    #[test]
    fn deserialize_message_of_note_array() {
        let layout = Layout::blank(Model::NoteArray(NoteArray::SideBySide2));
        let json = json!({
            "currentMessage": { "layout": serde_json::to_string(&layout).unwrap(), "id": "id" }
        });

        let message: Message = serde_json::from_value(json).unwrap();

        assert_eq!(message.into_layout(), layout);
    }

    #[yare::parameterized(
        layout_is_not_a_string = { r#"{"currentMessage":{"layout":[[0]],"id":"id"}}"# },
        layout_is_not_json = { r#"{"currentMessage":{"layout":"nope","id":"id"}}"# },
        invalid_size = { r#"{"currentMessage":{"layout":"[[0,0]]","id":"id"}}"# },
        missing_id = { r#"{"currentMessage":{"layout":"[[0,0]]"}}"# },
        bare = { "[[0]]" },
    )]
    fn deserialize_invalid_message(json: &str) {
        assert!(serde_json::from_str::<Message>(json).is_err());
    }

    #[test]
    fn deserialize_sent() {
        let sent: Sent = serde_json::from_str(
            r#"{"status":"ok","id":"9b4f13f0-4623-4fb9-8fbf-09ae13350da1","created":1681154452865}"#,
        )
        .unwrap();

        assert_eq!(sent.status, "ok");
        assert_eq!(sent.id, "9b4f13f0-4623-4fb9-8fbf-09ae13350da1");
        assert_eq!(
            sent.created,
            UNIX_EPOCH + Duration::from_millis(1681154452865)
        );
    }
}
