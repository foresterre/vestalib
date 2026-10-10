pub mod format;
pub mod message;
pub mod transition;

pub use format::FormatMessage;
pub use message::{BlankMessage, Message, ReadMessage, SendMessage, Sent};
pub use transition::{
    ReadTransition, SetTransition, Transition, TransitionSpeed, TransitionType,
    UnknownTransitionSpeed, UnknownTransitionType,
};

use crate::api::Api;

pub enum Cloud {}

impl Api for Cloud {
    type Authentication<'a> = Authentication;
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Authentication {
    #[default]
    None,
    CloudToken,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::Endpoint;
    use crate::board::Model;
    use http::Method;
    use std::borrow::Cow;

    #[yare::parameterized(
        read_message = { &ReadMessage::new(), Method::GET, "", Authentication::CloudToken },
        send_message = { &SendMessage::text(Cow::Borrowed("HI")).unwrap(), Method::POST, "", Authentication::CloudToken },
        read_transition = { &ReadTransition::new(), Method::GET, "transition", Authentication::CloudToken },
        set_transition = { &SetTransition::new(Transition::new(TransitionType::Wave, TransitionSpeed::Fast)), Method::PUT, "transition", Authentication::CloudToken },
        format_message = { &FormatMessage::new(Cow::Borrowed("HI"), Model::Note), Method::POST, "vbml/format", Authentication::None },
    )]
    fn endpoint(
        endpoint: &dyn Endpoint<Api = Cloud>,
        method: Method,
        path: &str,
        authentication: Authentication,
    ) {
        assert_eq!(endpoint.method(), method);
        assert_eq!(endpoint.endpoint(), path);
        assert_eq!(endpoint.authentication(), authentication);
    }
}
