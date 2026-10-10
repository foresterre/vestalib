use anyhow::Context;
use clap::{ArgGroup, Args, Parser};
use std::borrow::Cow;
use std::num::NonZeroU32;
use std::time::Duration;
use url::Url;
use vestalib_core::api::local::{Enablement, ReadMessage, SendMessage, Strategy, Transition};
use vestalib_core::api::{Json, Query, Void};
use vestalib_core::board::{Layout, Model};
use vestalib_core::client::{ApiKey, DEFAULT_URL, EnablementToken, ReqwestClient};

pub fn run() -> anyhow::Result<()> {
    if let Some(value) = execute(Cli::parse())? {
        println!("{}", value);
    }

    Ok(())
}

fn execute(args: Cli) -> anyhow::Result<Option<serde_json::Value>> {
    let mut client = ReqwestClient::new(args.url)?;

    if let Some(api_key) = args.api_key.as_deref() {
        client = client.with_api_key(ApiKey::new(api_key).context("Invalid Local API key")?);
    }

    let value = match args.subcommand {
        Subcommand::Enable(opts) => Some(Json::new(opts.endpoint()?).query(&client)?),
        Subcommand::Read => Some(Json::new(ReadMessage::new()).query(&client)?),
        Subcommand::Send(opts) => {
            let layout = opts.layout()?;
            Void::new(opts.endpoint(&layout)?).query(&client)?;
            None
        }
    };

    Ok(value)
}

#[derive(Parser)]
#[command(name = "vestalib", version, about, long_about = None)]
struct Cli {
    /// Address of the Vestaboard.
    ///
    /// Per the docs, a second Vestaboard on the network is reachable at
    /// http://vestaboard-2.local:7000, a third at http://vestaboard-3.local:7000, etc.
    /// This is untested, I only have one vestaboard.
    #[arg(long, env = "VESTABOARD_URL", default_value = DEFAULT_URL)]
    url: Url,

    /// Local API key, as printed by the `enable` command.
    #[arg(long, env = "VESTABOARD_LOCAL_API_KEY", hide_env_values = true)]
    api_key: Option<String>,

    #[command(subcommand)]
    subcommand: Subcommand,
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
            (Some(text), _) => {
                let model = self.model.unwrap_or(Model::Flagship);

                Layout::from_text(model, text)
                    .with_context(|| format!("Unable to show the text on a '{}'", model))
            }
            (None, Some(characters)) => {
                serde_json::from_str(characters).context("Unable to parse the character codes")
            }
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

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;
    use mockito::{Matcher, Server};
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
}
