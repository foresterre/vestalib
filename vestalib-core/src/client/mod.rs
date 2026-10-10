pub mod cloud;
pub mod local;
pub mod reqwest;
pub mod token;

pub use reqwest::{Error, NewClientError, ReqwestAsyncClient, ReqwestClient};
pub use token::InvalidToken;

use url::Url;

fn with_trailing_slash(mut url: Url) -> Url {
    if !url.path().ends_with('/') {
        url.set_path(&format!("{}/", url.path()));
    }

    url
}
