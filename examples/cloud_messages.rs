use std::borrow::Cow;
use std::env;
use std::error::Error;
use std::thread::sleep;
use std::time::{Duration, Instant};
use vestalib::api::cloud::SendMessage;
use vestalib::api::{Query, Void};
use vestalib::client::ReqwestClient;
use vestalib::client::cloud::{CloudConnection, CloudToken, DEFAULT_CLOUD_URL};

const INTERVAL: Duration = Duration::from_secs(15);

fn main() -> Result<(), Box<dyn Error>> {
    let url = env::var("VESTABOARD_CLOUD_URL").unwrap_or_else(|_| DEFAULT_CLOUD_URL.to_string());
    let token = env::var("VESTABOARD_CLOUD_API_TOKEN")
        .map_err(|_| "VESTABOARD_CLOUD_API_TOKEN is not set")?;
    let messages: Vec<String> = env::args().skip(1).collect();

    if messages.is_empty() {
        return Err("Usage: cargo run --example cloud_messages -- <message>...".into());
    }

    let connection = CloudConnection::new(url.parse()?)?.with_token(CloudToken::new(&token)?);
    let client = ReqwestClient::new(connection)?;

    messages.iter().try_fold(Instant::now(), |next, text| {
        sleep(next.saturating_duration_since(Instant::now()));

        Void::new(SendMessage::text(Cow::Borrowed(text))?).query(&client)?;

        Ok::<_, Box<dyn Error>>(Instant::now() + INTERVAL)
    })?;

    Ok(())
}
