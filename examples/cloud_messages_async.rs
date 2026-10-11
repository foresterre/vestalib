use futures_util::{StreamExt, TryStreamExt, stream};
use std::borrow::Cow;
use std::env;
use std::error::Error;
use std::time::Duration;
use tokio::time::{Instant, sleep_until};
use vestalib::api::cloud::SendMessage;
use vestalib::api::{AsyncQuery, Void};
use vestalib::client::ReqwestAsyncClient;
use vestalib::client::cloud::{CloudConnection, CloudToken, DEFAULT_CLOUD_URL};

const INTERVAL: Duration = Duration::from_secs(15);

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn Error>> {
    let url = env::var("VESTABOARD_CLOUD_URL").unwrap_or_else(|_| DEFAULT_CLOUD_URL.to_string());
    let token = env::var("VESTABOARD_CLOUD_API_TOKEN")
        .map_err(|_| "VESTABOARD_CLOUD_API_TOKEN is not set")?;
    let messages: Vec<String> = env::args().skip(1).collect();

    if messages.is_empty() {
        return Err("Usage: cargo run --example cloud_messages_async -- <message>...".into());
    }

    let connection = CloudConnection::new(url.parse()?)?.with_token(CloudToken::new(&token)?);
    let client = &ReqwestAsyncClient::new(connection)?;

    stream::iter(&messages)
        .map(Ok)
        .try_fold(Instant::now(), |next, text| async move {
            sleep_until(next).await;

            Void::new(SendMessage::text(Cow::Borrowed(text))?)
                .query_async(client)
                .await?;

            Ok::<_, Box<dyn Error>>(Instant::now() + INTERVAL)
        })
        .await?;

    Ok(())
}
