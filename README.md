# Vestalib

... is an HTTP client library and CLI for the [Vestaboard](https://docs.vestaboard.com/docs). Both the [Local API](https://docs.vestaboard.com/docs/local-api/introduction)
and the [Cloud API](https://docs.vestaboard.com/docs/read-write-api/introduction) are supported. Both API's have been tested against a Vestaboard Note.

## Install

**CLI**

```sh
cargo install vestalib --features cli
```

**HTTP Client**

```sh
cargo add vestalib
```

## Usage

### Client

**CLI**

```sh
export VESTABOARD_URL=http://vestaboard.local:7000
export VESTABOARD_LOCAL_API_KEY=<api-key>
```

**HTTP Client**

```rust
use vestalib::client::ReqwestClient;
use vestalib::client::local::{ApiKey, LocalConnection};

let connection = LocalConnection::new("http://192.168.1.50:7000".parse()?)?
    .with_api_key(ApiKey::new("<api-key>")?);
let client = ReqwestClient::new(connection)?;
```

### Enable the Local API: `vestalib enable`

**CLI**

```sh
vestalib enable --token <enablement-token>
```

**HTTP Client**

```rust
use vestalib::api::Query;
use vestalib::api::local::{Enabled, Enablement};
use vestalib::client::local::EnablementToken;

let enabled: Enabled = Enablement::new(EnablementToken::new("<enablement-token>")?).query(&client)?;
```

### Read the current message: `vestalib read`

**CLI**

```sh
vestalib read
```

**HTTP Client**

```rust
use vestalib::api::Query;
use vestalib::api::local::{Message, ReadMessage};

let message: Message = ReadMessage::new().query(&client)?;
```

### Send a message: `vestalib send`

**CLI**

```sh
vestalib send --text "HELLO WORLD"
vestalib send --model note --text $'HELLO\nWORLD'
```

**HTTP Client**

```rust
use std::borrow::Cow;
use vestalib::api::{Void, Query};
use vestalib::api::local::SendMessage;
use vestalib::board::{Layout, Model};

let layout = Layout::from_text(Model::Note, "HELLO\nWORLD")?;
Void::new(SendMessage::new(Cow::Borrowed(&layout))?).query(&client)?;
```

### Send a message with a transition: `vestalib send --strategy`

**CLI**

```sh
vestalib send --text "HELLO WORLD" --strategy column --step-interval-ms 100 --step-size 2
```

**HTTP Client**

```rust
use std::borrow::Cow;
use std::num::NonZeroU32;
use std::time::Duration;
use vestalib::api::{Void, Query};
use vestalib::api::local::{SendMessage, Strategy, Transition};
use vestalib::board::{Layout, Model};

let layout = Layout::from_text(Model::Flagship, "HELLO WORLD")?;
let transition = Transition::new(Strategy::Column)
    .with_step_interval(Duration::from_millis(100))
    .with_step_size(NonZeroU32::new(2).unwrap());
Void::new(SendMessage::new(Cow::Borrowed(&layout))?.with_transition(transition)).query(&client)?;
```

## Examples

### Simple text

**CLI**

```sh
vestalib send --model note --text $'HELLO\nWORLD'
```

**HTTP Client**

```rust
use std::borrow::Cow;
use vestalib::api::{Void, Query};
use vestalib::api::local::SendMessage;
use vestalib::board::{Layout, Model};

let layout = Layout::from_text(Model::Note, "HELLO\nWORLD")?;
Void::new(SendMessage::new(Cow::Borrowed(&layout))?).query(&client)?;
```

### Clock

**CLI**

```sh
while true; do
  vestalib send --model note --text $'\n     '"$(date +%H:%M)"
  sleep $((60 - $(date +%s) % 60))
done
```

**HTTP Client**

```rust
use std::borrow::Cow;
use std::thread::sleep;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use vestalib::api::{Void, Query};
use vestalib::api::local::SendMessage;
use vestalib::board::{Layout, Model};

loop {
    let secs = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    let text = format!("\n   {:02}:{:02} UTC", secs / 3600 % 24, secs / 60 % 60);
    let layout = Layout::from_text(Model::Note, &text)?;
    Void::new(SendMessage::new(Cow::Borrowed(&layout))?).query(&client)?;
    sleep(Duration::from_secs(60 - secs % 60));
}
```
