use anyhow::Context;
use clap::{ArgGroup, Args, Parser};
use std::borrow::Cow;
use std::num::NonZeroU32;
use std::time::Duration;
use url::Url;
use vestalib_core::api::cloud::{self, FormatMessage, ReadTransition, SetTransition};
use vestalib_core::api::cloud::{TransitionSpeed, TransitionType};
use vestalib_core::api::local::{Enablement, ReadMessage, SendMessage, Strategy, Transition};
use vestalib_core::api::{Json, Query, Void};
use vestalib_core::board::{Layout, Model};
use vestalib_core::client::ReqwestClient;
use vestalib_core::client::cloud::{CloudConnection, CloudToken, DEFAULT_CLOUD_URL};
use vestalib_core::client::local::{ApiKey, DEFAULT_LOCAL_URL, EnablementToken, LocalConnection};

pub fn run() -> anyhow::Result<()> {
    if let Some(value) = execute(Cli::parse())? {
        println!("{}", value);
    }

    Ok(())
}

fn execute(args: Cli) -> anyhow::Result<Option<serde_json::Value>> {
    let value = match &args.subcommand {
        Subcommand::Enable(opts) => Some(Json::new(opts.endpoint()?).query(&args.client()?)?),
        Subcommand::Read => Some(Json::new(ReadMessage::new()).query(&args.client()?)?),
        Subcommand::Send(opts) => {
            let layout = opts.layout()?;
            Void::new(opts.endpoint(&layout)?).query(&args.client()?)?;
            None
        }
        Subcommand::Cloud(opts) => Some(opts.execute()?),
    };

    Ok(value)
}

fn text_layout(model: Model, text: &str) -> anyhow::Result<Layout> {
    Layout::from_text(model, text)
        .with_context(|| format!("Unable to show the text on a '{}'", model))
}

fn characters_layout(characters: &str) -> anyhow::Result<Layout> {
    serde_json::from_str(characters).context("Unable to parse the character codes")
}

#[derive(Parser)]
#[command(name = "vestalib", version, about, long_about = None)]
struct Cli {
    /// Address of the Vestaboard.
    ///
    /// Per the docs, a second Vestaboard on the network is reachable at
    /// http://vestaboard-2.local:7000, a third at http://vestaboard-3.local:7000, etc.
    /// This is untested, I only have one vestaboard.
    #[arg(long, env = "VESTABOARD_URL", default_value = DEFAULT_LOCAL_URL)]
    url: Url,

    /// Local API key, as printed by the `enable` command.
    #[arg(long, env = "VESTABOARD_LOCAL_API_KEY", hide_env_values = true)]
    api_key: Option<String>,

    #[command(subcommand)]
    subcommand: Subcommand,
}

impl Cli {
    fn client(&self) -> anyhow::Result<ReqwestClient<LocalConnection>> {
        let mut connection = LocalConnection::new(self.url.clone())?;

        if let Some(api_key) = self.api_key.as_deref() {
            connection =
                connection.with_api_key(ApiKey::new(api_key).context("Invalid Local API key")?);
        }

        Ok(ReqwestClient::new(connection)?)
    }
}

#[derive(clap::Subcommand)]
#[command(propagate_version = true)]
pub enum Subcommand {
    /// Enable the Local API, and print the Local API key. Requires an enablement token.
    ///
    /// Endpoint: /local-api/enablement
    Enable(EnableOpts),
    /// Print the message which is currently shown. Requires a Local API key.
    ///
    /// Endpoint: /local-api/message
    Read,
    /// Show a message. Requires a Local API key.
    ///
    /// Endpoint: /local-api/message
    Send(SendOpts),
    /// Use the Cloud API, instead of the Local API.
    Cloud(CloudOpts),
}

#[derive(Debug, Args)]
#[command(next_help_heading = "Enable options")]
pub struct EnableOpts {
    /// The one-time enablement token
    ///
    /// Must be requested separately (fill in the rquest form on their website).
    #[arg(
        long,
        env = "VESTABOARD_LOCAL_API_ENABLEMENT_TOKEN",
        hide_env_values = true
    )]
    token: String,
}

impl EnableOpts {
    fn endpoint(&self) -> anyhow::Result<Enablement> {
        let token = EnablementToken::new(&self.token).context("Invalid enablement token")?;

        Ok(Enablement::new(token))
    }
}

#[derive(Debug, Args)]
#[command(next_help_heading = "Send options")]
#[command(group(ArgGroup::new("message").required(true).args(["text", "characters"])))]
pub struct SendOpts {
    /// Text to show, starting at the top left. Each line is shown on its own row.
    ///
    /// Supports letters, digits, spaces, and the symbols ! @ # $ ( ) - + & = ; : ' " % , . / ? °
    ///
    /// NB: ° is a heart.
    #[arg(long)]
    text: Option<String>,

    /// The Vestaboard model to lay out the text for. One of: flagship, or note. Defaults to flagship.
    #[arg(long, conflicts_with = "characters")]
    model: Option<Model>,

    /// Character codes to show, as a JSON array of rows.
    ///
    /// A Flagship has 6 rows of 22 characters, and a Note has 3 rows of 15 characters.
    #[arg(long, value_name = "JSON")]
    characters: Option<String>,

    /// One of: column, reverse-column, edges-to-center, row, diagonal, or random.
    #[arg(long)]
    strategy: Option<Strategy>,

    /// Delay between the start of each transition step, in milliseconds.
    #[arg(long, value_name = "MS", requires = "strategy")]
    step_interval_ms: Option<u64>,

    /// Amount of rows, columns or characters which are updated per transition step.
    #[arg(long, requires = "strategy")]
    step_size: Option<NonZeroU32>,
}

impl SendOpts {
    fn layout(&self) -> anyhow::Result<Layout> {
        match (&self.text, &self.characters) {
            (Some(text), _) => text_layout(self.model.unwrap_or(Model::Flagship), text),
            (None, Some(characters)) => characters_layout(characters),
            (None, None) => unreachable!("clap requires either --text or --characters"),
        }
    }

    fn endpoint<'a>(&self, layout: &'a Layout) -> anyhow::Result<SendMessage<'a>> {
        let mut endpoint = SendMessage::new(Cow::Borrowed(layout))?;

        if let Some(strategy) = self.strategy {
            let mut transition = Transition::new(strategy);

            if let Some(step_interval_ms) = self.step_interval_ms {
                transition = transition.with_step_interval(Duration::from_millis(step_interval_ms));
            }
            if let Some(step_size) = self.step_size {
                transition = transition.with_step_size(step_size);
            }

            endpoint = endpoint.with_transition(transition);
        }

        Ok(endpoint)
    }
}

#[derive(Debug, Args)]
#[command(next_help_heading = "Cloud options")]
pub struct CloudOpts {
    /// Address of the Vestaboard Cloud API. Must use HTTPS.
    #[arg(long, env = "VESTABOARD_CLOUD_URL", default_value = DEFAULT_CLOUD_URL)]
    url: Url,

    /// Cloud API token, as created in the Vestaboard app, or on web.vestaboard.com.
    #[arg(long, env = "VESTABOARD_CLOUD_API_TOKEN", hide_env_values = true)]
    token: Option<String>,

    #[command(subcommand)]
    subcommand: CloudSubcommand,
}

impl CloudOpts {
    fn client(&self) -> anyhow::Result<ReqwestClient<CloudConnection>> {
        let mut connection = CloudConnection::new(self.url.clone())?;

        if let Some(token) = self.token.as_deref() {
            connection =
                connection.with_token(CloudToken::new(token).context("Invalid Cloud API token")?);
        }

        Ok(ReqwestClient::new(connection)?)
    }

    fn execute(&self) -> anyhow::Result<serde_json::Value> {
        let client = self.client()?;

        let value = match &self.subcommand {
            CloudSubcommand::Read => Json::new(cloud::ReadMessage::new()).query(&client)?,
            CloudSubcommand::Send(opts) => Json::new(opts.endpoint()?).query(&client)?,
            CloudSubcommand::Transition(opts) => match (opts.transition_type, opts.speed) {
                (Some(transition_type), Some(speed)) => {
                    let transition = cloud::Transition::new(transition_type, speed);
                    Json::new(SetTransition::new(transition)).query(&client)?
                }
                (None, None) => Json::new(ReadTransition::new()).query(&client)?,
                _ => unreachable!("clap requires both --type and --speed, or neither"),
            },
            CloudSubcommand::Format(opts) => Json::new(opts.endpoint()).query(&client)?,
        };

        Ok(value)
    }
}

#[derive(Debug, clap::Subcommand)]
pub enum CloudSubcommand {
    /// Print the message which is currently shown. Requires a Cloud API token.
    ///
    /// Endpoint: /
    Read,
    /// Show a message. Requires a Cloud API token.
    ///
    /// Sending more than one message every 15 seconds may cause messages to be dropped.
    ///
    /// Endpoint: /
    Send(CloudSendOpts),
    /// Print the transition, or change it with --type and --speed. Requires a Cloud API token.
    ///
    /// Not supported by Note Arrays.
    ///
    /// Endpoint: /transition
    Transition(TransitionOpts),
    /// Print the character codes of a text, formatted the way Vestaboard formats a text message.
    ///
    /// Endpoint: /vbml/format
    Format(FormatOpts),
}

#[derive(Debug, Args)]
#[command(next_help_heading = "Send options")]
#[command(group(ArgGroup::new("message").required(true).args(["text", "characters"])))]
pub struct CloudSendOpts {
    /// Text to show. Vestaboard formats the text to fit the board, unless --model is given.
    #[arg(long)]
    text: Option<String>,

    /// Lay out the text for this model, starting at the top left, instead of letting
    /// Vestaboard format it. Each line is shown on its own row.
    ///
    /// One of: flagship, note, note-array-1x2, note-array-1x4, note-array-2x1, note-array-4x1,
    /// or note-array-2x2. A Note Array is named after its Notes, as rows x columns.
    #[arg(long, conflicts_with = "characters")]
    model: Option<Model>,

    /// Character codes to show, as a JSON array of rows.
    ///
    /// A Flagship has 6 rows of 22 characters, and a Note has 3 rows of 15 characters.
    /// A Note Array combines the rows and columns of its Notes.
    #[arg(long, value_name = "JSON")]
    characters: Option<String>,

    /// Show the message, even during quiet hours.
    #[arg(long)]
    forced: bool,
}

impl CloudSendOpts {
    fn endpoint(&self) -> anyhow::Result<cloud::SendMessage<'_>> {
        let endpoint = match (&self.text, &self.characters) {
            (Some(text), _) => match self.model {
                Some(model) => {
                    cloud::SendMessage::characters(Cow::Owned(text_layout(model, text)?))
                }
                None => cloud::SendMessage::text(Cow::Borrowed(text)),
            },
            (None, Some(characters)) => {
                cloud::SendMessage::characters(Cow::Owned(characters_layout(characters)?))
            }
            (None, None) => unreachable!("clap requires either --text or --characters"),
        }?;

        Ok(endpoint.with_forced(self.forced))
    }
}

#[derive(Debug, Args)]
#[command(next_help_heading = "Transition options")]
pub struct TransitionOpts {
    /// Change the transition type. One of: classic, wave, drift, or curtain.
    #[arg(long = "type", requires = "speed")]
    transition_type: Option<TransitionType>,

    /// Change the transition speed. One of: gentle, or fast.
    #[arg(long, requires = "transition_type")]
    speed: Option<TransitionSpeed>,
}

#[derive(Debug, Args)]
#[command(next_help_heading = "Format options")]
pub struct FormatOpts {
    /// Text to format.
    #[arg(long)]
    text: String,

    /// The Vestaboard model to format the text for. Defaults to flagship.
    ///
    /// One of: flagship, note, note-array-1x2, note-array-1x4, note-array-2x1, note-array-4x1,
    /// or note-array-2x2. A Note Array is named after its Notes, as rows x columns.
    #[arg(long)]
    model: Option<Model>,
}

impl FormatOpts {
    fn endpoint(&self) -> FormatMessage<'_> {
        FormatMessage::new(
            Cow::Borrowed(&self.text),
            self.model.unwrap_or(Model::Flagship),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;
    use mockito::{Matcher, Server, ServerGuard};
    use serde_json::json;

    #[test]
    fn cli_definition_is_valid() {
        Cli::command().debug_assert();
    }

    fn parse(args: &[&str]) -> Result<Cli, clap::Error> {
        Cli::try_parse_from(std::iter::once("vestalib").chain(args.iter().copied()))
    }

    #[yare::parameterized(
        no_message = { &["send"] },
        both_messages = { &["send", "--text", "HI", "--characters", "[]"] },
        model_without_text = { &["send", "--characters", "[]", "--model", "note"] },
        step_size_without_strategy = { &["send", "--text", "HI", "--step-size", "2"] },
        step_interval_without_strategy = { &["send", "--text", "HI", "--step-interval-ms", "2"] },
        zero_step_size = { &["send", "--text", "HI", "--strategy", "row", "--step-size", "0"] },
        unknown_strategy = { &["send", "--text", "HI", "--strategy", "wave"] },
        unknown_model = { &["send", "--text", "HI", "--model", "mini"] },
        invalid_url = { &["--url", "vestaboard", "read"] },
        cloud_no_message = { &["cloud", "send"] },
        cloud_both_messages = { &["cloud", "send", "--text", "HI", "--characters", "[]"] },
        cloud_model_without_text = { &["cloud", "send", "--characters", "[]", "--model", "note"] },
        cloud_strategy = { &["cloud", "send", "--text", "HI", "--strategy", "row"] },
        cloud_type_without_speed = { &["cloud", "transition", "--type", "wave"] },
        cloud_speed_without_type = { &["cloud", "transition", "--speed", "fast"] },
        cloud_unknown_type = { &["cloud", "transition", "--type", "flip", "--speed", "fast"] },
        cloud_format_without_text = { &["cloud", "format"] },
        cloud_enable = { &["cloud", "enable", "--token", "token"] },
    )]
    fn rejects_invalid_arguments(args: &[&str]) {
        assert!(parse(args).is_err());
    }

    #[test]
    fn send_text_with_transition() {
        let blank = vec![0; 15];
        let mut server = Server::new();
        let mock = server
            .mock("POST", "/local-api/message")
            .match_header("X-Vestaboard-Local-Api-Key", "key")
            .match_body(Matcher::PartialJson(json!({
                "characters": [[8, 9, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0], blank, blank],
                "strategy": "edges-to-center",
                "step_size": 2,
            })))
            .with_status(201)
            .create();
        let args = parse(&[
            "--url",
            &server.url(),
            "--api-key",
            "key",
            "send",
            "--text",
            "hi",
            "--model",
            "note",
            "--strategy",
            "edges-to-center",
            "--step-size",
            "2",
        ])
        .unwrap();

        let value = execute(args).unwrap();

        mock.assert();
        assert_eq!(value, None);
    }

    #[test]
    fn send_characters() {
        let blank = vec![0; 15];
        let characters = json!([
            [63, 64, 65, 66, 67, 68, 69, 70, 0, 0, 0, 0, 0, 0, 0],
            blank,
            blank
        ]);
        let mut server = Server::new();
        let mock = server
            .mock("POST", "/local-api/message")
            .match_body(Matcher::Json(characters.clone()))
            .with_status(201)
            .create();
        let args = parse(&[
            "--url",
            &server.url(),
            "--api-key",
            "key",
            "send",
            "--characters",
            &characters.to_string(),
        ])
        .unwrap();

        execute(args).unwrap();

        mock.assert();
    }

    #[yare::parameterized(
        too_long = { &["send", "--text", "THIS LINE IS FAR TOO LONG FOR A BOARD"] },
        unsupported = { &["send", "--text", "snake_case"] },
        invalid_size = { &["send", "--characters", "[[1, 2, 3]]"] },
        filled = { &["send", "--characters", "[[71,0,0,0,0,0,0,0,0,0,0,0,0,0,0],[0,0,0,0,0,0,0,0,0,0,0,0,0,0,0],[0,0,0,0,0,0,0,0,0,0,0,0,0,0,0]]"] },
    )]
    fn invalid_message_is_an_error_before_sending(args: &[&str]) {
        let mut server = Server::new();
        let mock = server.mock("POST", Matcher::Any).expect(0).create();
        let url = server.url();
        let args = [&["--url", url.as_str(), "--api-key", "key"][..], args].concat();

        assert!(execute(parse(&args).unwrap()).is_err());
        mock.assert();
    }

    #[test]
    fn read() {
        let mut server = Server::new();
        let mock = server
            .mock("GET", "/local-api/message")
            .match_header("X-Vestaboard-Local-Api-Key", "key")
            .with_body(r#"{"message":[[0]]}"#)
            .create();
        let args = parse(&["--url", &server.url(), "--api-key", "key", "read"]).unwrap();

        let value = execute(args).unwrap();

        mock.assert();
        assert_eq!(value, Some(json!({ "message": [[0]] })));
    }

    #[test]
    fn enable() {
        let mut server = Server::new();
        let mock = server
            .mock("POST", "/local-api/enablement")
            .match_header("X-Vestaboard-Local-Api-Enablement-Token", "token")
            .with_body(r#"{"message":"Local API enabled","apiKey":"XXXX"}"#)
            .create();
        let args = parse(&["--url", &server.url(), "enable", "--token", "token"]).unwrap();

        let value = execute(args).unwrap();

        mock.assert();
        assert_eq!(value.unwrap()["apiKey"], "XXXX");
    }

    fn cloud(server: &ServerGuard, args: &[&str]) -> Cli {
        let url = server.url();
        let args = [
            &["cloud", "--url", url.as_str(), "--token", "token"][..],
            args,
        ]
        .concat();

        parse(&args).unwrap()
    }

    #[test]
    fn cloud_read() {
        let mut server = Server::new();
        let mock = server
            .mock("GET", "/")
            .match_header("X-Vestaboard-Token", "token")
            .with_body(r#"{"currentMessage":{"layout":"[[0]]","id":"id"}}"#)
            .create();

        let value = execute(cloud(&server, &["read"])).unwrap();

        mock.assert();
        assert_eq!(value.unwrap()["currentMessage"]["id"], "id");
    }

    #[test]
    fn cloud_send_text() {
        let mut server = Server::new();
        let mock = server
            .mock("POST", "/")
            .match_header("X-Vestaboard-Token", "token")
            .match_body(Matcher::Json(
                json!({ "text": "Hello World", "forced": true }),
            ))
            .with_body(r#"{"status":"ok","id":"id","created":1681154452865}"#)
            .create();

        let value = execute(cloud(
            &server,
            &["send", "--text", "Hello World", "--forced"],
        ))
        .unwrap();

        mock.assert();
        assert_eq!(value.unwrap()["status"], "ok");
    }

    #[test]
    fn cloud_send_text_for_model() {
        let blank = vec![0; 15];
        let mut server = Server::new();
        let mock = server
            .mock("POST", "/")
            .match_body(Matcher::Json(json!({
                "characters": [[8, 9, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0], blank, blank],
            })))
            .with_body(r#"{"status":"ok","id":"id","created":1681154452865}"#)
            .create();

        execute(cloud(&server, &["send", "--text", "hi", "--model", "note"])).unwrap();

        mock.assert();
    }

    #[test]
    fn cloud_send_characters_of_note_array() {
        let mut row = vec![0; 30];
        row[29] = 63;
        let characters = json!([row, vec![0; 30], vec![0; 30]]);
        let mut server = Server::new();
        let mock = server
            .mock("POST", "/")
            .match_body(Matcher::Json(json!({ "characters": characters })))
            .with_body(r#"{"status":"ok","id":"id","created":1681154452865}"#)
            .create();

        execute(cloud(
            &server,
            &["send", "--characters", &characters.to_string()],
        ))
        .unwrap();

        mock.assert();
    }

    #[yare::parameterized(
        blank_text = { &["send", "--text", "  "] },
        blank_characters = { &["send", "--characters", "[[0,0,0,0,0,0,0,0,0,0,0,0,0,0,0],[0,0,0,0,0,0,0,0,0,0,0,0,0,0,0],[0,0,0,0,0,0,0,0,0,0,0,0,0,0,0]]"] },
        too_long_for_model = { &["send", "--text", "THIS LINE IS FAR TOO LONG", "--model", "note"] },
        invalid_size = { &["send", "--characters", "[[1, 2, 3]]"] },
    )]
    fn invalid_cloud_message_is_an_error_before_sending(args: &[&str]) {
        let mut server = Server::new();
        let mock = server.mock("POST", Matcher::Any).expect(0).create();

        assert!(execute(cloud(&server, args)).is_err());
        mock.assert();
    }

    #[test]
    fn cloud_read_transition() {
        let mut server = Server::new();
        let mock = server
            .mock("GET", "/transition")
            .match_header("X-Vestaboard-Token", "token")
            .with_body(r#"{"transition":"wave","transitionSpeed":"gentle"}"#)
            .create();

        let value = execute(cloud(&server, &["transition"])).unwrap();

        mock.assert();
        assert_eq!(
            value,
            Some(json!({ "transition": "wave", "transitionSpeed": "gentle" }))
        );
    }

    #[test]
    fn cloud_set_transition() {
        let mut server = Server::new();
        let mock = server
            .mock("PUT", "/transition")
            .match_header("X-Vestaboard-Token", "token")
            .match_body(Matcher::Json(
                json!({ "transition": "curtain", "transitionSpeed": "fast" }),
            ))
            .with_body(r#"{"transition":"curtain","transitionSpeed":"fast"}"#)
            .create();

        execute(cloud(
            &server,
            &["transition", "--type", "curtain", "--speed", "fast"],
        ))
        .unwrap();

        mock.assert();
    }

    #[test]
    fn cloud_format() {
        let mut server = Server::new();
        let mock = server
            .mock("POST", "/vbml/format")
            .match_header("X-Vestaboard-Token", Matcher::Missing)
            .match_body(Matcher::Json(
                json!({ "message": "Hello", "rows": 3, "columns": 15 }),
            ))
            .with_body("[[0,0,0,0,0,0,0,0,0,0,0,0,0,0,0],[0,0,0,0,0,8,5,12,12,15,0,0,0,0,0],[0,0,0,0,0,0,0,0,0,0,0,0,0,0,0]]")
            .create();
        let url = server.url();
        let args = parse(&[
            "cloud", "--url", &url, "format", "--text", "Hello", "--model", "note",
        ])
        .unwrap();

        let value = execute(args).unwrap();

        mock.assert();
        assert_eq!(value.unwrap()[1][5], 8);
    }

    #[test]
    fn cloud_rejects_insecure_url() {
        let args = parse(&["cloud", "--url", "http://vestaboard.local:7000", "read"]).unwrap();

        assert!(execute(args).is_err());
    }
}
