//! HTTP utilities

use std::time::Duration;

use reqwest::Client;

pub use crate::proxy::ProxyConfig;

pub const DEFAULT_USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/146.0.7680.153 Safari/537.36";
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);

/// Configuration for building an HTTP client
#[derive(Debug, Clone)]
pub struct HttpClientConfig {
    pub proxy: Option<ProxyConfig>,
    pub user_agent: Option<String>,
    pub timeout: Duration,
}

impl Default for HttpClientConfig {
    fn default() -> Self {
        Self {
            proxy: None,
            user_agent: Some(DEFAULT_USER_AGENT.to_string()),
            timeout: DEFAULT_TIMEOUT,
        }
    }
}

impl HttpClientConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_proxy(mut self, proxy: ProxyConfig) -> Self {
        self.proxy = Some(proxy);
        self
    }

    pub fn with_user_agent(mut self, user_agent: impl Into<String>) -> Self {
        self.user_agent = Some(user_agent.into());
        self
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }
}

/// Builder for creating HTTP clients with custom configuration
pub struct HttpClientBuilder {
    config: HttpClientConfig,
}

impl HttpClientBuilder {
    pub fn new() -> Self {
        Self {
            config: HttpClientConfig::default(),
        }
    }

    pub fn with_config(mut self, config: HttpClientConfig) -> Self {
        self.config = config;
        self
    }

    pub fn proxy(mut self, proxy: ProxyConfig) -> Self {
        self.config.proxy = Some(proxy);
        self
    }

    pub fn user_agent(mut self, user_agent: impl Into<String>) -> Self {
        self.config.user_agent = Some(user_agent.into());
        self
    }

    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.config.timeout = timeout;
        self
    }

    pub fn build(self) -> Result<HttpClient, crate::error::ScrapioError> {
        HttpClient::with_config(self.config)
    }
}

impl Default for HttpClientBuilder {
    fn default() -> Self {
        Self::new()
    }
}

pub struct HttpClient {
    client: Client,
}

impl Clone for HttpClient {
    fn clone(&self) -> Self {
        Self {
            client: self.client.clone(),
        }
    }
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

    pub fn with_config(config: HttpClientConfig) -> Result<Self, crate::error::ScrapioError> {
        let mut builder = Client::builder()
            .timeout(config.timeout);

        // Add user agent
        if let Some(ua) = config.user_agent {
            builder = builder.user_agent(ua);
        }

        // Add proxy if configured
        if let Some(proxy) = config.proxy {
            let proxy_url = if let (Some(username), Some(password)) = (&proxy.username, &proxy.password) {
                // Inject credentials into URL for reqwest
                format!("{}{}:{}@", proxy.url, username, password)
            } else {
                proxy.url.clone()
            };

            // reqwest::Proxy::http/https returns Result<Proxy, Error>
            // Try http first, fall back to https if it fails
            let reqwest_proxy = reqwest::Proxy::http(&proxy_url)
                .or_else(|_| reqwest::Proxy::https(&proxy_url))?;

            builder = builder.proxy(reqwest_proxy);
        }

        let client = builder
            .build()
            .map_err(|e| crate::error::ScrapioError::Http(format!("Failed to create HTTP client: {}", e)))?;

        Ok(Self { client })
    }

    pub fn builder() -> HttpClientBuilder {
        HttpClientBuilder::new()
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
