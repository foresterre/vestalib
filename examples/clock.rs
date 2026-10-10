use std::borrow::Cow;
use std::env;
use std::error::Error;
use std::thread::sleep;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use vestalib::api::local::SendMessage;
use vestalib::api::{Query, Void};
use vestalib::board::{Layout, Model};
use vestalib::client::ReqwestClient;
use vestalib::client::local::{ApiKey, DEFAULT_LOCAL_URL, LocalConnection};

fn main() -> Result<(), Box<dyn Error>> {
    let url = env::var("VESTABOARD_URL").unwrap_or_else(|_| DEFAULT_LOCAL_URL.to_string());
    let api_key =
        env::var("VESTABOARD_LOCAL_API_KEY").map_err(|_| "VESTABOARD_LOCAL_API_KEY is not set")?;

    let connection = LocalConnection::new(url.parse()?)?.with_api_key(ApiKey::new(&api_key)?);
    let client = ReqwestClient::new(connection)?;

    loop {
        let secs = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
        let text = format!("\n   {:02}:{:02} UTC", secs / 3600 % 24, secs / 60 % 60);
        let layout = Layout::from_text(Model::Note, &text)?;

        Void::new(SendMessage::new(Cow::Borrowed(&layout))?).query(&client)?;

        sleep(Duration::from_secs(60 - secs % 60));
    }
}
