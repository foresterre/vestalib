use crate::api::{Authentication, Endpoint};
use crate::client::{ApiKey, EnablementToken};
use std::borrow::Cow;

/// API to enable the Local API, which responds with the Local API key.
///
/// The enablement token can only be used once.
#[derive(Clone, Debug)]
pub struct Enablement {
    token: EnablementToken,
}

impl Enablement {
    pub fn new(token: EnablementToken) -> Self {
        Self { token }
    }
}

impl Endpoint for Enablement {
    fn method(&self) -> http::Method {
        http::Method::POST
    }

    fn endpoint(&self) -> Cow<'static, str> {
        Cow::Borrowed("local-api/enablement")
    }

    fn authentication(&self) -> Authentication<'_> {
        Authentication::EnablementToken(&self.token)
    }
}

#[derive(Clone, Debug, serde::Deserialize)]
#[non_exhaustive]
pub struct Enabled {
    pub message: String,
    #[serde(rename = "apiKey")]
    pub api_key: ApiKey,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserialize_enabled() {
        let enabled: Enabled =
            serde_json::from_str(r#"{"message":"Local API enabled","apiKey":"XXXX"}"#).unwrap();

        assert_eq!(enabled.message, "Local API enabled");
        assert_eq!(enabled.api_key.expose_secret(), "XXXX");
    }
}
