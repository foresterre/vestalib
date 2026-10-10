use crate::api::cloud::{Authentication, Cloud};
use crate::api::{Body, BodyError, Endpoint};
use std::borrow::Cow;
use std::fmt;
use std::str::FromStr;

/// API to read the transition which is used to show a new message.
///
/// Transitions are supported by the Flagship and Note, but not by Note Arrays.
#[derive(Clone, Debug, Default)]
pub struct ReadTransition;

impl ReadTransition {
    pub fn new() -> Self {
        Self
    }
}

impl Endpoint for ReadTransition {
    type Api = Cloud;

    fn method(&self) -> http::Method {
        http::Method::GET
    }

    fn endpoint(&self) -> Cow<'static, str> {
        Cow::Borrowed("transition")
    }

    fn authentication(&self) -> Authentication {
        Authentication::CloudToken
    }
}

/// API to change the transition which is used to show a new message.
///
/// Transitions are supported by the Flagship and Note, but not by Note Arrays.
#[derive(Clone, Debug)]
pub struct SetTransition {
    transition: Transition,
}

impl SetTransition {
    pub fn new(transition: Transition) -> Self {
        Self { transition }
    }
}

impl Endpoint for SetTransition {
    type Api = Cloud;

    fn method(&self) -> http::Method {
        http::Method::PUT
    }

    fn endpoint(&self) -> Cow<'static, str> {
        Cow::Borrowed("transition")
    }

    fn body(&self) -> Result<Option<Body>, BodyError> {
        Body::json(&self.transition).map(Some)
    }

    fn authentication(&self) -> Authentication {
        Authentication::CloudToken
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Transition {
    #[serde(rename = "transition")]
    transition_type: TransitionType,
    #[serde(rename = "transitionSpeed")]
    speed: TransitionSpeed,
}

impl Transition {
    pub fn new(transition_type: TransitionType, speed: TransitionSpeed) -> Self {
        Self {
            transition_type,
            speed,
        }
    }

    pub fn transition_type(&self) -> TransitionType {
        self.transition_type
    }

    pub fn speed(&self) -> TransitionSpeed {
        self.speed
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TransitionType {
    Classic,
    Wave,
    Drift,
    Curtain,
}

impl TransitionType {
    const ALL: [TransitionType; 4] = [
        TransitionType::Classic,
        TransitionType::Wave,
        TransitionType::Drift,
        TransitionType::Curtain,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            TransitionType::Classic => "classic",
            TransitionType::Wave => "wave",
            TransitionType::Drift => "drift",
            TransitionType::Curtain => "curtain",
        }
    }
}

impl fmt::Display for TransitionType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for TransitionType {
    type Err = UnknownTransitionType;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        TransitionType::ALL
            .into_iter()
            .find(|transition_type| transition_type.as_str() == s)
            .ok_or_else(|| UnknownTransitionType {
                value: s.to_string(),
            })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[error(
    "Unknown transition type '{}', expected one of: classic, wave, drift, or curtain",
    value
)]
pub struct UnknownTransitionType {
    pub value: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TransitionSpeed {
    Gentle,
    Fast,
}

impl TransitionSpeed {
    const ALL: [TransitionSpeed; 2] = [TransitionSpeed::Gentle, TransitionSpeed::Fast];

    pub fn as_str(self) -> &'static str {
        match self {
            TransitionSpeed::Gentle => "gentle",
            TransitionSpeed::Fast => "fast",
        }
    }
}

impl fmt::Display for TransitionSpeed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for TransitionSpeed {
    type Err = UnknownTransitionSpeed;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        TransitionSpeed::ALL
            .into_iter()
            .find(|speed| speed.as_str() == s)
            .ok_or_else(|| UnknownTransitionSpeed {
                value: s.to_string(),
            })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[error(
    "Unknown transition speed '{}', expected one of: gentle, or fast",
    value
)]
pub struct UnknownTransitionSpeed {
    pub value: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn body() {
        let endpoint = SetTransition::new(Transition::new(
            TransitionType::Wave,
            TransitionSpeed::Gentle,
        ));
        let body = endpoint.body().unwrap().unwrap();

        assert_eq!(body.content_type(), "application/json");
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&body.into_bytes()).unwrap(),
            json!({ "transition": "wave", "transitionSpeed": "gentle" })
        );
    }

    #[test]
    fn deserialize_transition() {
        let transition: Transition =
            serde_json::from_str(r#"{"transition":"curtain","transitionSpeed":"fast"}"#).unwrap();

        assert_eq!(transition.transition_type(), TransitionType::Curtain);
        assert_eq!(transition.speed(), TransitionSpeed::Fast);
    }

    #[yare::parameterized(
        unknown_type = { r#"{"transition":"flip","transitionSpeed":"fast"}"# },
        unknown_speed = { r#"{"transition":"wave","transitionSpeed":"slow"}"# },
        missing_speed = { r#"{"transition":"wave"}"# },
    )]
    fn deserialize_invalid_transition(json: &str) {
        assert!(serde_json::from_str::<Transition>(json).is_err());
    }

    #[test]
    fn transition_type_matches_serde() {
        for transition_type in TransitionType::ALL {
            assert_eq!(
                serde_json::to_value(transition_type).unwrap(),
                transition_type.as_str()
            );
            assert_eq!(
                transition_type.as_str().parse::<TransitionType>(),
                Ok(transition_type)
            );
        }
    }

    #[test]
    fn transition_speed_matches_serde() {
        for speed in TransitionSpeed::ALL {
            assert_eq!(serde_json::to_value(speed).unwrap(), speed.as_str());
            assert_eq!(speed.as_str().parse::<TransitionSpeed>(), Ok(speed));
        }
    }

    #[test]
    fn unknown_transition_type() {
        assert_eq!(
            "flip".parse::<TransitionType>(),
            Err(UnknownTransitionType {
                value: "flip".to_string()
            })
        );
    }

    #[test]
    fn unknown_transition_speed() {
        assert_eq!(
            "slow".parse::<TransitionSpeed>(),
            Err(UnknownTransitionSpeed {
                value: "slow".to_string()
            })
        );
    }
}
