pub mod enablement;
pub mod message;

pub use enablement::{Enabled, Enablement};
pub use message::{
    InvalidMessage, Message, ReadMessage, SendMessage, Strategy, Transition, UnknownStrategy,
};

use crate::api::Api;
use crate::client::local::EnablementToken;

pub enum Local {}

impl Api for Local {
    type Authentication<'a> = Authentication<'a>;
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Authentication<'a> {
    #[default]
    None,
    ApiKey,
    EnablementToken(&'a EnablementToken),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::Endpoint;
    use crate::board::{Layout, Model};
    use http::Method;
    use std::borrow::Cow;

    #[test]
    fn enablement() {
        let token = EnablementToken::new("token").unwrap();
        let endpoint = Enablement::new(token.clone());

        assert_eq!(endpoint.method(), Method::POST);
        assert_eq!(endpoint.endpoint(), "local-api/enablement");
        assert_eq!(
            endpoint.authentication(),
            Authentication::EnablementToken(&token)
        );
        assert_eq!(endpoint.body().unwrap(), None);
    }

    #[yare::parameterized(
        read_message = { &ReadMessage::new(), Method::GET, "local-api/message" },
        send_message = { &SendMessage::new(Cow::Owned(Layout::blank(Model::Note))).unwrap(), Method::POST, "local-api/message" },
    )]
    fn endpoint(endpoint: &dyn Endpoint<Api = Local>, method: Method, path: &str) {
        assert_eq!(endpoint.method(), method);
        assert_eq!(endpoint.endpoint(), path);
        assert_eq!(endpoint.authentication(), Authentication::ApiKey);
    }
}
