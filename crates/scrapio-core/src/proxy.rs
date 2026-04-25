//! Proxy configuration and utilities for Scrapio
//!
//! This module provides proxy configuration types and utilities
//! for large-scale web crawling with proxy rotation support.

use std::fmt;
use std::str::FromStr;
use std::time::Duration;

use rand::Rng;

/// Proxy configuration for HTTP/HTTPS connections
#[derive(Debug, Clone)]
pub struct ProxyConfig {
    /// Proxy URL (e.g., "http://proxy.example.com:8080")
    pub url: String,
    /// Optional username for authenticated proxies
    pub username: Option<String>,
    /// Optional password for authenticated proxies
    pub password: Option<String>,
}

impl ProxyConfig {
    /// Create a new proxy config from a URL string
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            username: None,
            password: None,
        }
    }

    /// Set username for proxy authentication
    pub fn with_username(mut self, username: impl Into<String>) -> Self {
        self.username = Some(username.into());
        self
    }

    /// Set password for proxy authentication
    pub fn with_password(mut self, password: impl Into<String>) -> Self {
        self.password = Some(password.into());
        self
    }

    /// Parse proxy config from a URL string
    /// Supports formats:
    /// - "http://host:port"
    /// - "http://user:pass@host:port"
    /// - "host:port" (defaults to HTTP)
    pub fn parse(s: &str) -> Result<Self, ProxyParseError> {
        // Add scheme if missing
        let url_with_scheme = if s.starts_with("http://") || s.starts_with("https://") {
            s.to_string()
        } else {
            format!("http://{}", s)
        };

        // Parse URL
        let parsed = url::Url::parse(&url_with_scheme).map_err(|e| ProxyParseError::InvalidUrl {
            url: s.to_string(),
            source: e,
        })?;

        let username = parsed.username();
        let password = parsed.password().map(|s| s.to_string());

        // Reconstruct URL without credentials for the base URL
        let mut base_url = parsed.clone();
        let _ = base_url.set_username("");
        let _ = base_url.set_password(None);

        Ok(Self {
            url: base_url.to_string(),
            username: if username.is_empty() {
                None
            } else {
                Some(username.to_string())
            },
            password,
        })
    }

    /// Get the full proxy URL with credentials embedded (for browser args)
    pub fn with_auth(&self) -> String {
        if let (Some(username), Some(password)) = (&self.username, &self.password) {
            // Parse URL and inject credentials
            if let Ok(mut parsed) = url::Url::parse(&self.url) {
                let _ = parsed.set_username(username);
                let _ = parsed.set_password(Some(password));
                return parsed.to_string();
            }
        }
        self.url.clone()
    }

    /// Get proxy host and port
    pub fn host_port(&self) -> Option<(String, u16)> {
        if let Ok(parsed) = url::Url::parse(&self.url)
            && let Some(host) = parsed.host_str()
            && let Some(port) = parsed.port()
        {
            return Some((host.to_string(), port));
        }
        None
    }
}

impl FromStr for ProxyConfig {
    type Err = ProxyParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

impl fmt::Display for ProxyConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let (Some(username), Some(password)) = (&self.username, &self.password) {
            // Show masked password
            write!(f, "{}:{}***@{}", username, &password[..password.len().min(2)], self.url)
        } else {
            write!(f, "{}", self.url)
        }
    }
}

/// Error type for proxy parsing
#[derive(Debug, thiserror::Error)]
pub enum ProxyParseError {
    #[error("Invalid proxy URL '{url}': {source}")]
    InvalidUrl {
        url: String,
        #[source]
        source: url::ParseError,
    },
}

/// Proxy rotation strategy
#[derive(Debug, Clone, Copy, Default)]
pub enum RotationStrategy {
    /// Use a single proxy for all requests
    None,
    /// Rotate proxies round-robin style
    #[default]
    RoundRobin,
    /// Select a random proxy for each request
    Random,
    /// Use a different proxy per domain
    PerDomain,
    /// Use a different proxy per request
    PerRequest,
}

impl RotationStrategy {
    pub fn parse_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "roundrobin" | "round-robin" => RotationStrategy::RoundRobin,
            "random" => RotationStrategy::Random,
            "perdomain" | "per-domain" => RotationStrategy::PerDomain,
            "perrequest" | "per-request" => RotationStrategy::PerRequest,
            _ => RotationStrategy::None,
        }
    }
}

impl fmt::Display for RotationStrategy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RotationStrategy::None => write!(f, "none"),
            RotationStrategy::RoundRobin => write!(f, "round-robin"),
            RotationStrategy::Random => write!(f, "random"),
            RotationStrategy::PerDomain => write!(f, "per-domain"),
            RotationStrategy::PerRequest => write!(f, "per-request"),
        }
    }
}

/// Configuration for proxy rotation
#[derive(Debug, Clone)]
pub struct ProxyRotationConfig {
    /// List of proxies to rotate through
    pub proxies: Vec<ProxyConfig>,
    /// Rotation strategy to use
    pub strategy: RotationStrategy,
    /// Current index for round-robin
    current_index: usize,
    /// Domain to proxy mapping for per-domain strategy
    domain_map: std::collections::HashMap<String, usize>,
}

impl ProxyRotationConfig {
    /// Create a new proxy rotation config with the given proxies and strategy
    pub fn new(proxies: Vec<ProxyConfig>, strategy: RotationStrategy) -> Self {
        Self {
            proxies,
            strategy,
            current_index: 0,
            domain_map: std::collections::HashMap::new(),
        }
    }

    /// Get the next proxy based on the rotation strategy
    pub fn get_proxy(&mut self, domain: Option<&str>) -> Option<&ProxyConfig> {
        if self.proxies.is_empty() {
            return None;
        }

        match self.strategy {
            RotationStrategy::None | RotationStrategy::RoundRobin => {
                let proxy = &self.proxies[self.current_index];
                self.current_index = (self.current_index + 1) % self.proxies.len();
                Some(proxy)
            }
            RotationStrategy::Random => {
                let mut rng = rand::rng();
                let idx = rng.random_range(0..self.proxies.len());
                Some(&self.proxies[idx])
            }
            RotationStrategy::PerDomain => {
                let domain = domain.unwrap_or("default");
                // Compute index before borrowing to avoid borrow checker issues
                let proxies_len = self.proxies.len();
                let map_len = self.domain_map.len();
                let idx = *self.domain_map.entry(domain.to_string()).or_insert(map_len % proxies_len);
                Some(&self.proxies[idx])
            }
            RotationStrategy::PerRequest => {
                let mut rng = rand::rng();
                let idx = rng.random_range(0..self.proxies.len());
                Some(&self.proxies[idx])
            }
        }
    }

    /// Add a proxy to the rotation
    pub fn add_proxy(&mut self, proxy: ProxyConfig) {
        self.proxies.push(proxy);
    }

    /// Remove a proxy from the rotation by index
    pub fn remove_proxy(&mut self, index: usize) -> Option<ProxyConfig> {
        if index < self.proxies.len() {
            Some(self.proxies.remove(index))
        } else {
            None
        }
    }

    /// Get the number of proxies in the rotation
    pub fn len(&self) -> usize {
        self.proxies.len()
    }

    /// Check if there are no proxies configured
    pub fn is_empty(&self) -> bool {
        self.proxies.is_empty()
    }
}

impl Default for ProxyRotationConfig {
    fn default() -> Self {
        Self {
            proxies: Vec::new(),
            strategy: RotationStrategy::RoundRobin,
            current_index: 0,
            domain_map: std::collections::HashMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_proxy_parse_no_auth() {
        let proxy = ProxyConfig::parse("http://proxy.example.com:8080").unwrap();
        assert_eq!(proxy.url, "http://proxy.example.com:8080/");
        assert!(proxy.username.is_none());
        assert!(proxy.password.is_none());
    }

    #[test]
    fn test_proxy_parse_with_auth() {
        let proxy = ProxyConfig::parse("http://user:pass@proxy.example.com:8080").unwrap();
        assert_eq!(proxy.username, Some("user".to_string()));
        assert_eq!(proxy.password, Some("pass".to_string()));
    }

    #[test]
    fn test_proxy_parse_no_scheme() {
        let proxy = ProxyConfig::parse("proxy.example.com:8080").unwrap();
        assert_eq!(proxy.url, "http://proxy.example.com:8080/");
    }

    #[test]
    fn test_proxy_with_auth() {
        let mut proxy = ProxyConfig::new("http://proxy.example.com:8080");
        proxy = proxy.with_username("user").with_password("pass");
        assert!(proxy.with_auth().contains("user:pass@"));
    }

    #[test]
    fn test_rotation_strategy_round_robin() {
        let proxies = vec![
            ProxyConfig::new("http://proxy1.com:8080"),
            ProxyConfig::new("http://proxy2.com:8080"),
            ProxyConfig::new("http://proxy3.com:8080"),
        ];
        let mut config = ProxyRotationConfig::new(proxies, RotationStrategy::RoundRobin);

        // Should cycle through proxies
        assert_eq!(config.get_proxy(None).unwrap().url, "http://proxy1.com:8080");
        assert_eq!(config.get_proxy(None).unwrap().url, "http://proxy2.com:8080");
        assert_eq!(config.get_proxy(None).unwrap().url, "http://proxy3.com:8080");
        assert_eq!(config.get_proxy(None).unwrap().url, "http://proxy1.com:8080");
    }

    #[test]
    fn test_rotation_strategy_per_domain() {
        let proxies = vec![
            ProxyConfig::new("http://proxy1.com:8080"),
            ProxyConfig::new("http://proxy2.com:8080"),
        ];
        let mut config = ProxyRotationConfig::new(proxies, RotationStrategy::PerDomain);

        // Same domain should get same proxy
        let proxy1_url = config.get_proxy(Some("example.com")).unwrap().url.clone();
        let proxy2_url = config.get_proxy(Some("example.com")).unwrap().url.clone();
        assert_eq!(proxy1_url, proxy2_url);

        // Different domain should get different proxy (or same, depending on assignment)
        let proxy3 = config.get_proxy(Some("other.com"));
        // Just verify it returns a proxy
        assert!(proxy3.is_some());
    }
}

/// Proxy health status
#[derive(Debug, Clone)]
pub struct ProxyHealth {
    /// Whether the proxy is reachable and functional
    pub is_alive: bool,
    /// Response latency in milliseconds
    pub latency_ms: u64,
    /// Detected country (if available)
    pub country: Option<String>,
    /// When the health check was performed
    pub last_checked: chrono::DateTime<chrono::Utc>,
    /// Anonymity level of the proxy
    pub anonymity_level: AnonymityLevel,
}

impl Default for ProxyHealth {
    fn default() -> Self {
        Self {
            is_alive: false,
            latency_ms: 0,
            country: None,
            last_checked: chrono::Utc::now(),
            anonymity_level: AnonymityLevel::Unknown,
        }
    }
}

/// Proxy anonymity level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AnonymityLevel {
    /// Transparent proxy - reveals your IP address
    Transparent,
    /// Anonymous proxy - hides your IP but identifies itself as proxy
    Anonymous,
    /// Elite/High anonymity - hides your IP and doesn't identify as proxy
    Elite,
    /// Unknown anonymity level
    #[default]
    Unknown,
}

impl fmt::Display for AnonymityLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AnonymityLevel::Transparent => write!(f, "transparent"),
            AnonymityLevel::Anonymous => write!(f, "anonymous"),
            AnonymityLevel::Elite => write!(f, "elite"),
            AnonymityLevel::Unknown => write!(f, "unknown"),
        }
    }
}

/// Manager for proxy operations including health checking
pub struct ProxyManager {
    http_client: crate::http::HttpClient,
}

impl ProxyManager {
    /// Create a new proxy manager
    pub fn new() -> Self {
        Self {
            http_client: crate::http::HttpClient::new(),
        }
    }

    /// Create a proxy manager with a custom timeout
    pub fn with_timeout(timeout: Duration) -> Self {
        Self {
            http_client: crate::http::HttpClient::builder()
                .timeout(timeout)
                .build()
                .unwrap_or_else(|_| crate::http::HttpClient::new()),
        }
    }

    /// Validate a proxy by attempting to connect through it
    ///
    /// Returns a ProxyHealth struct with the proxy's status
    pub async fn validate_proxy(&self, proxy: &ProxyConfig) -> Result<ProxyHealth, ProxyParseError> {
        let start = std::time::Instant::now();

        // Create HTTP client with the proxy
        let client = match crate::http::HttpClient::builder()
            .proxy(proxy.clone())
            .timeout(Duration::from_secs(10))
            .build()
        {
            Ok(client) => client,
            Err(_) => {
                return Ok(ProxyHealth {
                    is_alive: false,
                    latency_ms: 0,
                    country: None,
                    last_checked: chrono::Utc::now(),
                    anonymity_level: AnonymityLevel::Unknown,
                });
            }
        };

        // Try to fetch a known URL (example.com is reliable)
        let result = client.client().get("http://example.com").send().await;
        let elapsed = start.elapsed();

        match result {
            Ok(_) => Ok(ProxyHealth {
                is_alive: true,
                latency_ms: elapsed.as_millis() as u64,
                country: None,
                last_checked: chrono::Utc::now(),
                anonymity_level: AnonymityLevel::Unknown,
            }),
            Err(_) => Ok(ProxyHealth {
                is_alive: false,
                latency_ms: 0,
                country: None,
                last_checked: chrono::Utc::now(),
                anonymity_level: AnonymityLevel::Unknown,
            }),
        }
    }

    /// Check the anonymity level of a proxy
    ///
    /// This works by checking if certain headers are present in responses
    /// that would indicate the proxy is identifying itself
    pub async fn check_anonymity_level(&self, proxy: &ProxyConfig) -> Result<AnonymityLevel, ProxyParseError> {
        // Create HTTP client with the proxy
        let client = match crate::http::HttpClient::builder()
            .proxy(proxy.clone())
            .timeout(Duration::from_secs(10))
            .build()
        {
            Ok(client) => client,
            Err(_) => return Ok(AnonymityLevel::Unknown),
        };

        // Try to fetch a URL that echoes request info
        // Using httpbin.org which returns request headers
        match client.client().get("https://httpbin.org/headers").send().await {
            Ok(response) => {
                // Check response for proxy-identifying headers
                let body = response.text().await.unwrap_or_default();
                let body_lower = body.to_lowercase();

                // If response contains Via, X-Forwarded-For, or Proxy-Connection headers,
                // the proxy is revealing information
                if body_lower.contains("via:") || body_lower.contains("x-forwarded-for:") {
                    // Proxy is forwarding your IP - transparent
                    Ok(AnonymityLevel::Transparent)
                } else if body_lower.contains("proxy") {
                    // Proxy identifies itself but may not forward IP
                    Ok(AnonymityLevel::Anonymous)
                } else {
                    // No proxy headers detected - elite
                    Ok(AnonymityLevel::Elite)
                }
            }
            Err(_) => Ok(AnonymityLevel::Unknown),
        }
    }

    /// Get the country of a proxy using an IP geolocation API
    pub async fn get_country(&self, proxy: &ProxyConfig) -> Option<String> {
        // Extract host from proxy URL
        let (host, _port) = proxy.host_port()?;

        // Use ipapi.co for geolocation (free, no API key required)
        let url = format!("https://ipapi.co/{}/json", host);

        match self.http_client.client().get(&url).send().await {
            Ok(response) => {
                if let Ok(body) = response.text().await {
                    // Try to parse country from JSON response
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&body) {
                        return json
                            .get("country_name")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string());
                    }
                }
                None
            }
            Err(_) => None,
        }
    }

    /// Validate multiple proxies and return their health status
    pub async fn validate_all(&self, proxies: &[ProxyConfig]) -> Vec<(ProxyConfig, ProxyHealth)> {
        let mut results = Vec::new();

        for proxy in proxies {
            let health = self.validate_proxy(proxy).await.unwrap_or_default();
            results.push((proxy.clone(), health));
        }

        results
    }

    /// Filter proxies by health status
    pub fn filter_healthy(proxies_with_health: &[(ProxyConfig, ProxyHealth)]) -> Vec<ProxyConfig> {
        proxies_with_health
            .iter()
            .filter(|(_, health)| health.is_alive)
            .map(|(proxy, _)| proxy.clone())
            .collect()
    }

    /// Sort proxies by latency (fastest first)
    pub fn sort_by_latency(proxies_with_health: &mut [(ProxyConfig, ProxyHealth)]) {
        proxies_with_health.sort_by_key(|a| a.1.latency_ms);
    }
}

impl Default for ProxyManager {
    fn default() -> Self {
        Self::new()
    }
}
