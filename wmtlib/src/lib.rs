use std::env;

use reqwest::{Client, Response};
use reqwest::header::{AUTHORIZATION, HeaderMap};
use serde::de::DeserializeOwned;

pub use self::check::CrateCheck;
pub use self::questions::{DeserializableQuestions, Question, Questions, read_questions_from_file};
pub use self::result::CommandResult;

mod cargo_crate;
mod check;
mod constants;
mod doc;
mod github;
mod questions;
mod result;
mod version;

pub fn log_if_verbose(verbose: bool, message: &str) {
    match verbose {
        true => tracing::info!(message),
        false => {}
    }
}

#[derive(Debug)]
pub struct HTTPClient {
    request_client: Client,
}

pub fn get_github_token() -> String {
    match env::var("GITHUB_TOKEN") {
        Ok(v) => v,
        Err(_) => "NO_TOKEN_FOUND".to_string(),
    }
}

impl Default for HTTPClient {
    fn default() -> Self {
        Self::new()
    }
}

impl HTTPClient {
    pub fn new() -> HTTPClient {
        static APP_USER_AGENT: &str =
            concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"));

        let request_client = reqwest::Client::builder().user_agent(APP_USER_AGENT);

        let request_client = match env::var("GITHUB_TOKEN") {
            Ok(_) => {
                let token = get_github_token();
                let mut hmap = HeaderMap::new();

                hmap.append(AUTHORIZATION, format!("Bearer {}", token).parse().unwrap());
                request_client
                    .default_headers(hmap)
                    .build()
                    .unwrap_or_default()
            }
            Err(_) => request_client.build().unwrap_or_default(),
        };
        HTTPClient { request_client }
    }

    #[tokio::main(flavor = "current_thread")]
    pub async fn get(&self, url: &str) -> Option<Response> {
        self.request_client.get(url).send().await.ok()
    }

    #[tokio::main(flavor = "current_thread")]
    pub async fn get_json<T: DeserializeOwned>(&self, url: String) -> reqwest::Result<T> {
        self.request_client.get(url).send().await?.json::<T>().await
    }

    pub fn build_github_api_url(&self, owner: &str, repo: &str) -> String {
        format!("https://api.github.com/repos/{}/{}", owner, repo)
    }
}
