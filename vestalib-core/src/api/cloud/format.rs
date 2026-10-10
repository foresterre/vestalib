use crate::api::{Body, BodyError, Endpoint};
use crate::board::Model;
use std::borrow::Cow;

/// API to format text into the characters of a board, the same way Vestaboard
/// formats text which is sent with [`SendMessage::text`](crate::api::cloud::SendMessage::text).
///
/// Responds with a [`Layout`](crate::board::Layout). Doesn't require a Cloud API token.
#[derive(Clone, Debug)]
pub struct FormatMessage<'a> {
    message: Cow<'a, str>,
    model: Model,
}

impl<'a> FormatMessage<'a> {
    pub fn new(message: Cow<'a, str>, model: Model) -> Self {
        Self { message, model }
    }
}

impl<'a> Endpoint for FormatMessage<'a> {
    fn method(&self) -> http::Method {
        http::Method::POST
    }

    fn endpoint(&self) -> Cow<'static, str> {
        Cow::Borrowed("vbml/format")
    }

    fn body(&self) -> Result<Option<Body>, BodyError> {
        #[derive(serde::Serialize)]
        struct FormatBody<'b> {
            message: &'b str,
            rows: usize,
            columns: usize,
        }

        Body::json(&FormatBody {
            message: self.message.as_ref(),
            rows: self.model.rows(),
            columns: self.model.columns(),
        })
        .map(Some)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::NoteArray;
    use serde_json::json;

    #[yare::parameterized(
        flagship = { Model::Flagship, 6, 22 },
        note = { Model::Note, 3, 15 },
        note_array = { Model::NoteArray(NoteArray::Stacked4), 12, 15 },
    )]
    fn body(model: Model, rows: usize, columns: usize) {
        let endpoint = FormatMessage::new(Cow::Borrowed("Hello World"), model);
        let body = endpoint.body().unwrap().unwrap();

        assert_eq!(body.content_type(), "application/json");
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&body.into_bytes()).unwrap(),
            json!({ "message": "Hello World", "rows": rows, "columns": columns })
        );
    }
}
