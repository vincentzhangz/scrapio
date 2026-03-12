//! HTTP utilities

use std::time::Duration;

use reqwest::Client;

pub const DEFAULT_USER_AGENT: &str = "Mozilla/5.0 (compatible; Scrapio/0.1)";
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);

pub struct HttpClient {
    client: Client,
}

impl HttpClient {
    pub fn new() -> Self {
        Self {
            client: Client::builder()
                .user_agent(DEFAULT_USER_AGENT)
                .timeout(DEFAULT_TIMEOUT)
                .build()
                .expect("Failed to create HTTP client"),
        }
    }

    pub fn client(&self) -> &Client {
        &self.client
    }
}

impl Default for HttpClient {
    fn default() -> Self {
        Self::new()
    }
}
