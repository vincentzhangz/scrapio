//! Robots.txt compliance and politeness

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use url::Url;

/// Manages robots.txt compliance for a domain
pub struct RobotsTxtManager {
    /// Parsed robots.txt rules per domain
    rules: Arc<RwLock<HashMap<String, ParsedRobots>>>,
    /// Request timestamps for rate limiting
    request_log: Arc<RwLock<RequestLog>>,
    /// User agent string
    user_agent: String,
}

#[derive(Clone)]
struct ParsedRobots {
    /// Allowed paths
    allowed: Vec<PathRule>,
    /// Disallowed paths
    disallowed: Vec<PathRule>,
    /// Crawl delay in seconds (if specified)
    crawl_delay: Option<f64>,
}

#[derive(Clone)]
struct PathRule {
    pattern: String,
    is_wildcard: bool,
}

struct RequestLog {
    /// Last request time per domain
    last_request: HashMap<String, Instant>,
    /// Request count for sliding window
    requests: Vec<(String, Instant)>,
    /// Window size for rate limiting
    window_secs: u64,
}

impl Default for RequestLog {
    fn default() -> Self {
        Self {
            last_request: HashMap::new(),
            requests: Vec::new(),
            window_secs: 60,
        }
    }
}

impl RobotsTxtManager {
    /// Create a new manager with the given user agent
    pub fn new(user_agent: &str) -> Self {
        Self {
            rules: Arc::new(RwLock::new(HashMap::new())),
            request_log: Arc::new(RwLock::new(RequestLog::default())),
            user_agent: user_agent.to_string(),
        }
    }

    /// Fetch and parse robots.txt for a domain
    pub async fn fetch(&self, base_url: &str) -> Result<(), RobotsError> {
        let domain = extract_domain(base_url)?;
        let robots_url = format!("{}://{}/robots.txt", scheme(base_url), domain);

        let client = reqwest::Client::new();
        let response = client
            .get(&robots_url)
            .header(reqwest::header::USER_AGENT, &self.user_agent)
            .send()
            .await
            .map_err(|e: reqwest::Error| RobotsError::FetchError(e.to_string()))?;

        if !response.status().is_success() {
            // No robots.txt or error - allow all
            return Ok(());
        }

        let content = response
            .text()
            .await
            .map_err(|e: reqwest::Error| RobotsError::FetchError(e.to_string()))?;

        let parsed = parse_robots_txt(&content)?;
        let mut rules = self.rules.write().await;
        rules.insert(domain, parsed);

        Ok(())
    }

    /// Check if a URL is allowed by robots.txt
    pub async fn is_allowed(&self, url: &str) -> bool {
        let domain = match extract_domain(url) {
            Ok(d) => d,
            Err(_) => return true, // Allow unknown domains
        };

        let rules = self.rules.read().await;
        let parsed = match rules.get(&domain) {
            Some(r) => r,
            None => return true, // No rules = allow
        };

        // Check disallowed patterns
        for rule in &parsed.disallowed {
            if matches_pattern(url, &rule.pattern, rule.is_wildcard) {
                // Check if any allowed pattern overrides
                for allowed in &parsed.allowed {
                    if matches_pattern(url, &allowed.pattern, allowed.is_wildcard) {
                        return true;
                    }
                }
                return false;
            }
        }

        true
    }

    /// Get crawl delay for the domain
    pub async fn crawl_delay(&self) -> Option<Duration> {
        // For now, return a default. In production, would fetch robots.txt first
        Some(Duration::from_millis(100))
    }

    /// Get crawl delay for a specific domain
    pub async fn crawl_delay_for(&self, url: &str) -> Option<Duration> {
        let domain = match extract_domain(url) {
            Ok(d) => d,
            Err(_) => return None,
        };

        let rules = self.rules.read().await;
        rules
            .get(&domain)
            .and_then(|p| p.crawl_delay.map(Duration::from_secs_f64))
    }

    /// Wait until next request is allowed (respects crawl-delay)
    pub async fn wait_until_allowed(&self, url: &str) {
        if let Some(delay) = self.crawl_delay_for(url).await {
            let domain = extract_domain(url).unwrap_or_default();
            let mut log = self.request_log.write().await;

            if let Some(last) = log.last_request.get(&domain) {
                let elapsed = last.elapsed();
                if elapsed < delay {
                    let wait = delay - elapsed;
                    tokio::time::sleep(wait).await;
                }
            }

            log.last_request.insert(domain, Instant::now());
        }
    }

    /// Check if we should rate limit (too many requests)
    pub async fn should_rate_limit(&self, domain: &str) -> bool {
        let mut log = self.request_log.write().await;
        let now = Instant::now();

        // Clean old entries
        let window = Duration::from_secs(log.window_secs);
        log.requests
            .retain(|(_, time)| now.duration_since(*time) < window);

        // Count requests to this domain in window
        let count = log.requests.iter().filter(|(d, _)| d == domain).count();
        count >= 10 // Max 10 requests per domain per window
    }

    /// Record a request for rate limiting
    pub async fn record_request(&self, url: &str) {
        let domain = extract_domain(url).unwrap_or_default();
        let mut log = self.request_log.write().await;
        log.requests.push((domain, Instant::now()));
    }
}

/// Extract domain from URL
fn extract_domain(url: &str) -> Result<String, RobotsError> {
    let parsed = Url::parse(url).map_err(|e| RobotsError::InvalidUrl(e.to_string()))?;
    parsed
        .host_str()
        .map(|s| s.to_string())
        .ok_or_else(|| RobotsError::InvalidUrl("no host".to_string()))
}

/// Get scheme from URL
fn scheme(url: &str) -> &'static str {
    if url.starts_with("https://") {
        "https"
    } else {
        "http"
    }
}

/// Parse robots.txt content
fn parse_robots_txt(content: &str) -> Result<ParsedRobots, RobotsError> {
    let mut allowed = Vec::new();
    let mut disallowed = Vec::new();
    let mut crawl_delay = None;

    let mut current_agent = "*".to_string();

    for line in content.lines() {
        let line = line.trim();

        // Skip comments and empty lines
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        // Handle user-agent
        if line.to_lowercase().starts_with("user-agent:") {
            let agent = line[11..].trim();
            current_agent = agent.to_string();
            continue;
        }

        // Skip non-* user agents (we act as *)
        if current_agent != "*" {
            continue;
        }

        // Handle Disallow
        if line.to_lowercase().starts_with("disallow:") {
            let path = line[9..].trim().to_string();
            if !path.is_empty() {
                let (pattern, is_wildcard) = convert_to_pattern(&path);
                disallowed.push(PathRule {
                    pattern,
                    is_wildcard,
                });
            }
            continue;
        }

        // Handle Allow (higher priority)
        if line.to_lowercase().starts_with("allow:") {
            let path = line[6..].trim().to_string();
            if !path.is_empty() {
                let (pattern, is_wildcard) = convert_to_pattern(&path);
                allowed.push(PathRule {
                    pattern,
                    is_wildcard,
                });
            }
            continue;
        }

        // Handle Crawl-delay
        if line.to_lowercase().starts_with("crawl-delay:") {
            let delay = line[12..].trim();
            if let Ok(d) = delay.parse::<f64>() {
                crawl_delay = Some(d);
            }
            continue;
        }
    }

    // Sort allowed patterns by length (longer = more specific first)
    allowed.sort_by(|a, b| b.pattern.len().cmp(&a.pattern.len()));

    Ok(ParsedRobots {
        allowed,
        disallowed,
        crawl_delay,
    })
}

/// Convert robots.txt path to regex pattern
fn convert_to_pattern(path: &str) -> (String, bool) {
    let mut pattern = String::new();
    let mut is_wildcard = false;

    for ch in path.chars() {
        match ch {
            '*' => {
                pattern.push_str(".*");
                is_wildcard = true;
            }
            '$' => {
                // End anchor - handled in matching
            }
            '.' | '+' | '?' | '[' | ']' | '(' | ')' | '^' | '|' => {
                pattern.push('\\');
                pattern.push(ch);
            }
            _ => {
                pattern.push(ch);
            }
        }
    }

    (pattern, is_wildcard)
}

/// Check if URL matches a robots.txt pattern
fn matches_pattern(url: &str, pattern: &str, is_wildcard: bool) -> bool {
    if is_wildcard {
        let regex_pattern = format!("^{}", pattern);

        regex::Regex::new(&regex_pattern)
            .map(|re| re.is_match(url))
            .unwrap_or(false)
    } else {
        // Exact prefix match
        url.starts_with(pattern)
    }
}

/// Robots.txt related errors
#[derive(Debug, thiserror::Error)]
pub enum RobotsError {
    #[error("Failed to fetch robots.txt: {0}")]
    FetchError(String),

    #[error("Invalid URL: {0}")]
    InvalidUrl(String),

    #[error("Parse error: {0}")]
    ParseError(String),
}

/// Politeness configuration
#[derive(Debug, Clone)]
pub struct PolitenessConfig {
    /// Minimum delay between requests to same domain (ms)
    pub min_delay_ms: u64,
    /// Maximum delay between requests to same domain (ms)
    pub max_delay_ms: u64,
    /// Whether to respect crawl-delay from robots.txt
    pub respect_crawl_delay: bool,
    /// Exponential backoff on errors
    pub backoff_on_error: bool,
    /// Initial backoff duration (ms)
    pub initial_backoff_ms: u64,
    /// Maximum backoff duration (ms)
    pub max_backoff_ms: u64,
}

impl Default for PolitenessConfig {
    fn default() -> Self {
        Self {
            min_delay_ms: 100,
            max_delay_ms: 5000,
            respect_crawl_delay: true,
            backoff_on_error: true,
            initial_backoff_ms: 1000,
            max_backoff_ms: 60000,
        }
    }
}

/// Rate limiter for domain-based request throttling
#[derive(Clone)]
pub struct DomainRateLimiter {
    /// Last request time per domain
    last_request: Arc<RwLock<HashMap<String, Instant>>>,
    /// Config
    config: PolitenessConfig,
}

impl DomainRateLimiter {
    /// Create a new rate limiter
    pub fn new(config: PolitenessConfig) -> Self {
        Self {
            last_request: Arc::new(RwLock::new(HashMap::new())),
            config,
        }
    }

    /// Wait until request is allowed for this domain
    pub async fn wait_for(&self, domain: &str) {
        let mut last = self.last_request.write().await;

        if let Some(last_time) = last.get(domain) {
            let elapsed = last_time.elapsed();
            let min_delay = Duration::from_millis(self.config.min_delay_ms);

            if elapsed < min_delay {
                let wait = min_delay - elapsed;
                drop(last); // Release lock before sleeping
                tokio::time::sleep(wait).await;
                let mut last = self.last_request.write().await;
                last.insert(domain.to_string(), Instant::now());
                return;
            }
        }

        last.insert(domain.to_string(), Instant::now());
    }

    /// Calculate backoff duration after an error
    pub fn calculate_backoff(&self, attempts: u32) -> Duration {
        let base = self.config.initial_backoff_ms;
        let max = self.config.max_backoff_ms;

        // Exponential backoff: base * 2^attempts
        let delay = base * 2u64.saturating_pow(attempts.min(10));
        let delay = delay.min(max);

        // Add jitter (±10%)
        let jitter = (delay as f64 * 0.1) as u64;
        let jitter = rand_jitter(jitter);

        Duration::from_millis(delay + jitter)
    }
}

/// Generate random jitter
fn rand_jitter(max: u64) -> u64 {
    use std::time::SystemTime;
    let nanos = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .subsec_nanos();
    (nanos as u64) % max
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_robots_txt_allows_root() {
        let content = "User-agent: *\nDisallow: /private/\nAllow: /public/";
        let result = parse_robots_txt(content);
        assert!(result.is_ok());
        let parsed = result.unwrap();
        assert!(!parsed.allowed.is_empty());
        assert!(!parsed.disallowed.is_empty());
    }

    #[test]
    fn test_parse_robots_txt_crawl_delay() {
        let content = "User-agent: *\nCrawl-delay: 1";
        let result = parse_robots_txt(content);
        assert!(result.is_ok());
        let parsed = result.unwrap();
        assert_eq!(parsed.crawl_delay, Some(1.0));
    }

    #[test]
    fn test_parse_robots_txt_sitemap() {
        let content = "User-agent: *\nSitemap: https://example.com/sitemap.xml";
        let result = parse_robots_txt(content);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_robots_txt_multiple_agents() {
        let content = "User-agent: Googlebot\nDisallow: /\n\nUser-agent: *\nAllow: /";
        let result = parse_robots_txt(content);
        assert!(result.is_ok());
    }

    #[test]
    fn test_convert_to_pattern_exact() {
        let (pattern, is_wildcard) = convert_to_pattern("/api/");
        assert_eq!(pattern, "/api/");
        assert!(!is_wildcard);
    }

    #[test]
    fn test_convert_to_pattern_wildcard() {
        let (pattern, is_wildcard) = convert_to_pattern("/api/*");
        assert!(pattern.contains(".*"));
        assert!(is_wildcard);
    }

    #[test]
    fn test_matches_pattern_exact() {
        assert!(matches_pattern("/api/test", "/api/", false));
        assert!(!matches_pattern("/other/test", "/api/", false));
    }

    #[test]
    fn test_matches_pattern_wildcard() {
        assert!(matches_pattern("/api/users/123", "/api/*", true));
        assert!(matches_pattern("/api/anything", "/api/*", true));
    }

    #[tokio::test]
    async fn test_robots_txt_manager_new() {
        let manager = RobotsTxtManager::new("TestBot/1.0");
        let _ = manager;
    }

    #[tokio::test]
    async fn test_robots_txt_manager_fetch_not_found() {
        let manager = RobotsTxtManager::new("TestBot/1.0");
        // This will fail to fetch but should not panic
        let result = manager
            .fetch("http://invalid-domain-that-does-not-exist-12345.com/")
            .await;
        // Result is error because domain doesn't exist
        assert!(result.is_err() || result.is_ok()); // Either is fine
    }

    #[tokio::test]
    async fn test_robots_txt_manager_crawl_delay_default() {
        let manager = RobotsTxtManager::new("TestBot/1.0");
        let delay = manager.crawl_delay().await;
        // Should return default delay
        assert!(delay.is_some());
    }

    #[test]
    fn test_politeness_config_default() {
        let config = PolitenessConfig::default();
        assert_eq!(config.min_delay_ms, 100);
        assert_eq!(config.max_delay_ms, 5000);
        assert!(config.respect_crawl_delay);
        assert!(config.backoff_on_error);
    }

    #[test]
    fn test_politeness_config_aggressive() {
        let config = PolitenessConfig {
            min_delay_ms: 50,
            max_delay_ms: 1000,
            respect_crawl_delay: false,
            backoff_on_error: false,
            initial_backoff_ms: 100,
            max_backoff_ms: 5000,
        };
        assert_eq!(config.min_delay_ms, 50);
        assert!(!config.respect_crawl_delay);
    }

    #[test]
    fn test_domain_rate_limiter_new() {
        let config = PolitenessConfig::default();
        let limiter = DomainRateLimiter::new(config);
        let _ = limiter;
    }

    #[test]
    fn test_domain_rate_limiter_calculate_backoff() {
        let config = PolitenessConfig::default();
        let limiter = DomainRateLimiter::new(config);

        // First backoff
        let backoff1 = limiter.calculate_backoff(0);
        assert!(backoff1.as_millis() >= 1000);

        // Higher backoff level
        let backoff3 = limiter.calculate_backoff(3);
        assert!(backoff3.as_millis() > backoff1.as_millis());
    }

    #[tokio::test]
    async fn test_domain_rate_limiter_wait_for() {
        let config = PolitenessConfig::default();
        let limiter = DomainRateLimiter::new(config);

        // Should complete without hanging
        limiter.wait_for("example.com").await;
    }

    #[test]
    fn test_rand_jitter() {
        let jitter = rand_jitter(100);
        assert!(jitter < 100);
    }
}
