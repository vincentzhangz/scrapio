//! Crawler types and configuration

use chrono::{DateTime, Utc};
use regex::Regex;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

/// Configuration for crawler operation
#[derive(Debug, Clone)]
pub struct CrawlOptions {
    /// Maximum crawl depth
    pub max_depth: usize,
    /// Maximum number of pages to crawl
    pub max_pages: usize,
    /// Number of concurrent requests
    pub concurrency: usize,
    /// Rate limit (requests per second)
    pub rate_limit: Option<u64>,
    /// Delay between requests in milliseconds
    pub delay_ms: u64,
    /// Scope configuration
    pub scope: Scope,
    /// Discovery options
    pub discover: DiscoverOptions,
    /// Output options
    pub output: OutputOptions,
    /// Browser escalation mode
    pub browser_escalation: BrowserEscalation,
    /// AI provider for intelligent decisions
    pub ai_provider: Option<String>,
    /// AI model to use
    pub ai_model: Option<String>,
    /// AI schema for extraction
    pub ai_schema: Option<String>,
    /// Whether to extract structured data with AI
    pub extract_data: bool,
    /// Whether to capture network requests in browser mode
    pub capture_network: bool,
    /// Channel sender for incremental results (for checkpoint saving)
    pub result_sender: Option<mpsc::Sender<CrawlResult>>,
}

impl Default for CrawlOptions {
    fn default() -> Self {
        Self {
            max_depth: 2,
            max_pages: 100,
            concurrency: 5,
            rate_limit: None,
            delay_ms: 100,
            scope: Scope::default(),
            discover: DiscoverOptions::default(),
            output: OutputOptions::default(),
            browser_escalation: BrowserEscalation::Never,
            ai_provider: None,
            ai_model: None,
            ai_schema: None,
            extract_data: false,
            capture_network: false,
            result_sender: None,
        }
    }
}

impl CrawlOptions {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_max_depth(mut self, depth: usize) -> Self {
        self.max_depth = depth;
        self
    }

    pub fn with_max_pages(mut self, pages: usize) -> Self {
        self.max_pages = pages;
        self
    }

    pub fn with_scope(mut self, scope: Scope) -> Self {
        self.scope = scope;
        self
    }

    pub fn with_rate_limit(mut self, rate: u64) -> Self {
        self.rate_limit = Some(rate);
        self
    }

    pub fn with_browser_escalation(mut self, mode: BrowserEscalation) -> Self {
        self.browser_escalation = mode;
        self
    }

    pub fn with_ai_provider(mut self, provider: &str) -> Self {
        self.ai_provider = Some(provider.to_string());
        self
    }

    pub fn with_ai_model(mut self, model: &str) -> Self {
        self.ai_model = Some(model.to_string());
        self
    }

    pub fn with_ai_schema(mut self, schema: &str) -> Self {
        self.ai_schema = Some(schema.to_string());
        self.extract_data = true;
        self
    }

    pub fn with_extract_data(mut self, extract: bool) -> Self {
        self.extract_data = extract;
        self
    }

    pub fn with_capture_network(mut self, capture: bool) -> Self {
        self.capture_network = capture;
        self
    }

    /// Set a channel sender for incremental results
    pub fn with_result_sender(mut self, sender: mpsc::Sender<CrawlResult>) -> Self {
        self.result_sender = Some(sender);
        self
    }
}

/// Scope configuration for URL filtering
#[derive(Debug, Clone)]
pub struct Scope {
    /// Scope mode
    pub mode: ScopeMode,
    /// Include subdomains when in domain mode
    pub include_subdomains: bool,
    /// Regex patterns to include
    pub regex_include: Vec<Regex>,
    /// Regex patterns to exclude
    pub regex_exclude: Vec<Regex>,
}

impl Default for Scope {
    fn default() -> Self {
        Self {
            mode: ScopeMode::Domain,
            include_subdomains: true,
            regex_include: Vec::new(),
            regex_exclude: Vec::new(),
        }
    }
}

impl Scope {
    pub fn new(mode: ScopeMode) -> Self {
        Self {
            mode,
            ..Default::default()
        }
    }

    pub fn host() -> Self {
        Self::new(ScopeMode::Host)
    }

    pub fn domain() -> Self {
        Self::new(ScopeMode::Domain)
    }

    pub fn subdomain() -> Self {
        Self::new(ScopeMode::Subdomain)
    }

    pub fn with_include_subdomains(mut self, include: bool) -> Self {
        self.include_subdomains = include;
        self
    }

    pub fn add_include_regex(mut self, pattern: &str) -> Result<Self, regex::Error> {
        self.regex_include.push(Regex::new(pattern)?);
        Ok(self)
    }

    pub fn add_exclude_regex(mut self, pattern: &str) -> Result<Self, regex::Error> {
        self.regex_exclude.push(Regex::new(pattern)?);
        Ok(self)
    }
}

/// Scope mode for URL filtering
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ScopeMode {
    /// Same host only (no subdomains)
    #[default]
    Host,
    /// Same domain (includes subdomains)
    Domain,
    /// Include subdomains explicitly
    Subdomain,
    /// Custom regex-based scope
    Custom,
}

/// What to discover from pages
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DiscoverOptions {
    /// Discover URLs in anchor tags
    pub anchors: bool,
    /// Discover form actions
    pub forms: bool,
    /// Discover script sources
    pub scripts: bool,
    /// Discover iframe sources
    pub iframes: bool,
    /// Discover meta tag URLs
    pub meta: bool,
    /// Discover canonical URLs
    pub canonical: bool,
    /// Follow redirects
    pub redirects: bool,
    /// Discover from sitemap.xml
    pub sitemap: bool,
    /// Discover from robots.txt
    pub robots: bool,
}

impl Default for DiscoverOptions {
    fn default() -> Self {
        Self {
            anchors: true,
            forms: true,
            scripts: true,
            iframes: false,
            meta: false,
            canonical: false,
            redirects: false,
            sitemap: false,
            robots: false,
        }
    }
}

impl DiscoverOptions {
    /// Create options that discover everything
    pub fn all() -> Self {
        Self {
            anchors: true,
            forms: true,
            scripts: true,
            iframes: true,
            meta: true,
            canonical: true,
            redirects: true,
            sitemap: true,
            robots: true,
        }
    }

    /// Create options for basic crawling (anchors only)
    pub fn basic() -> Self {
        Self {
            anchors: true,
            ..Default::default()
        }
    }

    /// Create options for standard crawling
    pub fn standard() -> Self {
        Self {
            anchors: true,
            forms: true,
            scripts: true,
            iframes: true,
            redirects: true,
            ..Default::default()
        }
    }

    /// Enable anchor tag discovery
    pub fn with_anchors(mut self) -> Self {
        self.anchors = true;
        self
    }

    /// Enable form discovery
    pub fn with_forms(mut self) -> Self {
        self.forms = true;
        self
    }

    /// Enable script discovery
    pub fn with_scripts(mut self) -> Self {
        self.scripts = true;
        self
    }
}

/// Output options for crawl results
#[derive(Debug, Clone, Default)]
pub struct OutputOptions {
    /// Output format
    pub format: OutputFormat,
    /// Output file path
    pub file: Option<String>,
    /// Include response body in output
    pub include_body: bool,
    /// Include raw HTTP response
    pub include_raw: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OutputFormat {
    #[default]
    Text,
    Json,
    Csv,
    Jsonl, // JSON Lines
}

/// Browser escalation mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BrowserEscalation {
    /// Never use browser, HTTP only
    #[default]
    Never,
    /// Automatically escalate when needed
    Auto,
    /// Always use browser
    Always,
}

/// Result of a crawled page
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrawlResult {
    /// Timestamp of the crawl
    pub timestamp: DateTime<Utc>,
    /// The crawled URL
    pub url: String,
    /// Parent URL that led to this URL
    pub source_url: Option<String>,
    /// Crawl depth
    pub depth: usize,
    /// HTTP status code
    pub status: u16,
    /// Content-Type header
    pub content_type: Option<String>,
    /// Content-Length header
    pub content_length: Option<i64>,
    /// Page title
    pub title: Option<String>,
    /// Number of links found
    pub links_found: usize,
    /// Number of forms found
    pub forms_found: usize,
    /// Source type (anchor, form, script, etc.)
    pub source: String,
    /// Error message if crawl failed
    pub error: Option<String>,
    /// Whether browser was used
    pub used_browser: bool,
    /// AI-extracted data (JSON)
    pub extracted_data: Option<serde_json::Value>,
    /// Detection: is this an SPA/JS-heavy page?
    pub is_js_heavy: Option<bool>,
    /// Detection: does page need interaction?
    pub needs_interaction: Option<bool>,
}

impl CrawlResult {
    pub fn new(url: String) -> Self {
        Self {
            timestamp: Utc::now(),
            url,
            source_url: None,
            depth: 0,
            status: 0,
            content_type: None,
            content_length: None,
            title: None,
            links_found: 0,
            forms_found: 0,
            source: "direct".to_string(),
            error: None,
            used_browser: false,
            extracted_data: None,
            is_js_heavy: None,
            needs_interaction: None,
        }
    }

    pub fn with_status(mut self, status: u16) -> Self {
        self.status = status;
        self
    }

    pub fn with_source_url(mut self, url: String) -> Self {
        self.source_url = Some(url);
        self
    }

    pub fn with_depth(mut self, depth: usize) -> Self {
        self.depth = depth;
        self
    }

    pub fn with_title(mut self, title: String) -> Self {
        self.title = Some(title);
        self
    }

    pub fn with_error(mut self, error: String) -> Self {
        self.error = Some(error);
        self
    }

    pub fn with_source(mut self, source: String) -> Self {
        self.source = source;
        self
    }

    pub fn with_links_found(mut self, count: usize) -> Self {
        self.links_found = count;
        self
    }

    pub fn with_forms_found(mut self, count: usize) -> Self {
        self.forms_found = count;
        self
    }

    pub fn with_used_browser(mut self, used: bool) -> Self {
        self.used_browser = used;
        self
    }

    pub fn with_extracted_data(mut self, data: serde_json::Value) -> Self {
        self.extracted_data = Some(data);
        self
    }

    pub fn with_js_heavy(mut self, is_js_heavy: bool) -> Self {
        self.is_js_heavy = Some(is_js_heavy);
        self
    }

    pub fn with_needs_interaction(mut self, needs: bool) -> Self {
        self.needs_interaction = Some(needs);
        self
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }

    pub fn to_csv_row(&self) -> String {
        format!(
            "{},{},{},{},{},{},{},{},{}",
            self.timestamp.to_rfc3339(),
            escape_csv(&self.url),
            self.source_url.as_deref().unwrap_or(""),
            self.depth,
            self.status,
            self.title.as_deref().unwrap_or(""),
            self.links_found,
            self.forms_found,
            self.error.as_deref().unwrap_or("")
        )
    }

    pub fn csv_header() -> String {
        "timestamp,url,source_url,depth,status,title,links_found,forms_found,error".to_string()
    }
}

/// Escape a string for CSV
fn escape_csv(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}
