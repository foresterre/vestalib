use crate::api::{Authentication, Body, BodyError, Endpoint};
use crate::board::{Character, InvalidSize, Layout};
use std::borrow::Cow;
use std::fmt;
use std::num::NonZeroU32;
use std::str::FromStr;
use std::time::Duration;

/// API to read the message which is currently shown.
#[derive(Clone, Debug, Default)]
pub struct ReadMessage;

impl ReadMessage {
    pub fn new() -> Self {
        Self
    }
}

impl Endpoint for ReadMessage {
    fn method(&self) -> http::Method {
        http::Method::GET
    }

    fn endpoint(&self) -> Cow<'static, str> {
        Cow::Borrowed("local-api/message")
    }

    fn authentication(&self) -> Authentication<'_> {
        Authentication::ApiKey
    }
}

/// The message which is currently shown.
#[derive(Clone, Debug, PartialEq, Eq, serde::Deserialize)]
#[serde(try_from = "MessageResponse")]
pub struct Message {
    layout: Layout,
}

impl Message {
    pub fn layout(&self) -> &Layout {
        &self.layout
    }

    pub fn into_layout(self) -> Layout {
        self.layout
    }
}

#[derive(serde::Deserialize)]
#[serde(untagged)]
enum MessageResponse {
    Wrapped { message: Vec<Vec<Character>> },
    Bare(Vec<Vec<Character>>),
}

impl TryFrom<MessageResponse> for Message {
    type Error = InvalidSize;

    fn try_from(response: MessageResponse) -> Result<Self, Self::Error> {
        let rows = match response {
            MessageResponse::Wrapped { message } => message,
            MessageResponse::Bare(rows) => rows,
        };

        Layout::new(rows).map(|layout| Self { layout })
    }
}

/// API to show a message.
#[derive(Clone, Debug)]
pub struct SendMessage<'a> {
    layout: Cow<'a, Layout>,
    transition: Option<Transition>,
}

impl<'a> SendMessage<'a> {
    pub fn new(layout: Cow<'a, Layout>) -> Result<Self, UnavailableCharacter> {
        if layout.contains(Character::Filled) {
            return Err(UnavailableCharacter {
                character: Character::Filled,
            });
        }

        Ok(Self {
            layout,
            transition: None,
        })
    }

    /// Transitions set in the Vestaboard app don't apply to messages sent with the Local API.
    pub fn with_transition(mut self, transition: Transition) -> Self {
        self.transition = Some(transition);
        self
    }
}

impl<'a> Endpoint for SendMessage<'a> {
    fn method(&self) -> http::Method {
        http::Method::POST
    }

    fn endpoint(&self) -> Cow<'static, str> {
        Cow::Borrowed("local-api/message")
    }

    fn body(&self) -> Result<Option<Body>, BodyError> {
        #[derive(serde::Serialize)]
        struct TransitionBody<'b> {
            characters: &'b Layout,
            strategy: Strategy,
            #[serde(skip_serializing_if = "Option::is_none")]
            step_interval_ms: Option<u64>,
            #[serde(skip_serializing_if = "Option::is_none")]
            step_size: Option<NonZeroU32>,
        }

        let body = match &self.transition {
            None => Body::json(self.layout.as_ref())?,
            Some(transition) => Body::json(&TransitionBody {
                characters: self.layout.as_ref(),
                strategy: transition.strategy,
                step_interval_ms: transition
                    .step_interval
                    .map(|interval| u64::try_from(interval.as_millis()).unwrap_or(u64::MAX)),
                step_size: transition.step_size,
            })?,
        };

        Ok(Some(body))
    }

    fn authentication(&self) -> Authentication<'_> {
        Authentication::ApiKey
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
#[error(
    "The character '{:?}' (code '{}') is not available for the Local API",
    character,
    character.code()
)]
pub struct UnavailableCharacter {
    pub character: Character,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Transition {
    strategy: Strategy,
    step_interval: Option<Duration>,
    step_size: Option<NonZeroU32>,
}

impl Transition {
    pub fn new(strategy: Strategy) -> Self {
        Self {
            strategy,
            step_interval: None,
            step_size: None,
        }
    }

    /// The delay between the start of each step. Defaults to starting steps as quickly as possible.
    pub fn with_step_interval(mut self, step_interval: Duration) -> Self {
        self.step_interval = Some(step_interval);
        self
    }

    /// The amount of rows, columns or characters which are updated at once. Defaults to one.
    pub fn with_step_size(mut self, step_size: NonZeroU32) -> Self {
        self.step_size = Some(step_size);
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Strategy {
    /// "Wave" in the Vestaboard app
    Column,
    /// "Drift" in the Vestaboard app
    ReverseColumn,
    /// "Curtain" in the Vestaboard app
    EdgesToCenter,
    Row,
    Diagonal,
    Random,
}

impl Strategy {
    const ALL: [Strategy; 6] = [
        Strategy::Column,
        Strategy::ReverseColumn,
        Strategy::EdgesToCenter,
        Strategy::Row,
        Strategy::Diagonal,
        Strategy::Random,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Strategy::Column => "column",
            Strategy::ReverseColumn => "reverse-column",
            Strategy::EdgesToCenter => "edges-to-center",
            Strategy::Row => "row",
            Strategy::Diagonal => "diagonal",
            Strategy::Random => "random",
        }
    }
}

impl fmt::Display for Strategy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for Strategy {
    type Err = UnknownStrategy;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Strategy::ALL
            .into_iter()
            .find(|strategy| strategy.as_str() == s)
            .ok_or_else(|| UnknownStrategy {
                value: s.to_string(),
            })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[error(
    "Unknown transition strategy '{}', expected one of: column, reverse-column, edges-to-center, row, diagonal, or random",
    value
)]
pub struct UnknownStrategy {
    pub value: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::Model;
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
    fn body_without_transition() {
        let endpoint = SendMessage::new(Cow::Owned(note())).unwrap();

        assert_eq!(body(&endpoint), serde_json::to_value(note()).unwrap());
    }

    #[test]
    fn body_with_transition() {
        let endpoint = SendMessage::new(Cow::Owned(note()))
            .unwrap()
            .with_transition(Transition::new(Strategy::EdgesToCenter));

        assert_eq!(
            body(&endpoint),
            json!({ "characters": note(), "strategy": "edges-to-center" })
        );
    }

    #[test]
    fn body_with_transition_steps() {
        let endpoint = SendMessage::new(Cow::Owned(note()))
            .unwrap()
            .with_transition(
                Transition::new(Strategy::Column)
                    .with_step_interval(Duration::from_secs(3))
                    .with_step_size(NonZeroU32::new(2).unwrap()),
            );

        assert_eq!(
            body(&endpoint),
            json!({
                "characters": note(),
                "strategy": "column",
                "step_interval_ms": 3000,
                "step_size": 2,
            })
        );
    }

    #[test]
    fn filled_is_unavailable() {
        let mut rows = note().into_rows();
        rows[1][1] = Character::Filled;
        let layout = Layout::new(rows).unwrap();

        assert_eq!(
            SendMessage::new(Cow::Owned(layout)).unwrap_err(),
            UnavailableCharacter {
                character: Character::Filled
            }
        );
    }

    #[test]
    fn deserialize_bare_message() {
        let json = serde_json::to_string(&note()).unwrap();

        let message: Message = serde_json::from_str(&json).unwrap();

        assert_eq!(message.into_layout(), note());
    }

    #[test]
    fn deserialize_wrapped_message() {
        let json = json!({ "message": note() });

        let message: Message = serde_json::from_value(json).unwrap();

        assert_eq!(message.into_layout(), note());
    }

    #[yare::parameterized(
        invalid_size = { "[[0, 0], [0, 0]]" },
        unknown_code = { "[[43]]" },
        object = { r#"{"characters": []}"# },
    )]
    fn deserialize_invalid_message(json: &str) {
        assert!(serde_json::from_str::<Message>(json).is_err());
    }

    #[test]
    fn strategy_matches_serde() {
        for strategy in Strategy::ALL {
            assert_eq!(serde_json::to_value(strategy).unwrap(), strategy.as_str());
            assert_eq!(strategy.as_str().parse::<Strategy>(), Ok(strategy));
        }
    }

    #[test]
    fn unknown_strategy() {
        assert_eq!(
            "wave".parse::<Strategy>(),
            Err(UnknownStrategy {
                value: "wave".to_string()
            })
        );
    }
}
