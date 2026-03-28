//! Crawler module - production-ready web crawling system
//!
//! This module provides a production-ready crawler with:
//! - URL frontier/queue management
//! - Scope-based URL filtering
//! - Multi-source URL discovery
//! - Configurable crawl options

pub mod frontier;
pub mod parser;
pub mod robots;
pub mod scope;
pub mod state;
pub mod types;

pub use frontier::{Frontier, FrontierEntry, FrontierStats, UrlSource};
pub use parser::{ResponseParser, count_forms, count_links, parse_sitemap};
pub use robots::{PolitenessConfig, RobotsTxtManager};
pub use scope::ScopeValidator;
pub use state::{CrawlMetadata, CrawlState};
pub use types::CrawlResult;
pub use types::*;

use tokio::time::{Duration, sleep};
use tracing::{debug, info, instrument, warn};

use self::robots::DomainRateLimiter;

/// Main crawler that orchestrates crawling
pub struct Crawler {
    options: CrawlOptions,
    frontier: Frontier,
    scope: ScopeValidator,
    parser: ResponseParser,
    results: Vec<CrawlResult>,
    robots: RobotsTxtManager,
    rate_limiter: DomainRateLimiter,
    state: Option<CrawlState>,
    error_count: usize,
    backoff_level: u32,
}

impl Crawler {
    /// Create a new crawler from a root URL and options
    pub fn new(root_url: &str, options: CrawlOptions) -> Result<Self, CrawlerError> {
        let scope = ScopeValidator::new(root_url, options.scope.clone())
            .map_err(CrawlerError::ScopeError)?;

        let frontier = Frontier::new();
        let parser = ResponseParser::new(options.discover);

        // Initialize robots.txt manager with latest Chrome user agent
        let user_agent = scrapio_core::user_agent::UserAgentManager::new()
            .with_browser(scrapio_core::user_agent::Browser::Chrome);
        let robots = RobotsTxtManager::new(&user_agent.get_user_agent());

        // Initialize rate limiter with politeness config
        let politeness = PolitenessConfig {
            min_delay_ms: options.politeness.min_delay_ms,
            max_delay_ms: options.politeness.max_delay_ms,
            respect_crawl_delay: options.politeness.respect_robots_txt,
            backoff_on_error: options.politeness.backoff_on_error,
            initial_backoff_ms: options.politeness.initial_backoff_ms,
            max_backoff_ms: options.politeness.max_backoff_ms,
        };
        let rate_limiter = DomainRateLimiter::new(politeness);

        // Initialize state persistence if enabled
        let state = if options.persistence.enabled {
            Some(CrawlState::new(&options.persistence.state_dir))
        } else {
            None
        };

        Ok(Self {
            options,
            frontier,
            scope,
            parser,
            results: Vec::new(),
            robots,
            rate_limiter,
            state,
            error_count: 0,
            backoff_level: 0,
        })
    }

    /// Initialize the crawler with root URL in the frontier
    pub async fn init(&self) {
        let entry = FrontierEntry::new(self.scope.root_url().to_string(), 0);
        self.frontier.push(entry).await;
    }

    /// Initialize crawler with optional sitemap/robots.txt discovery
    pub async fn init_with_discovery(&self, discover_sitemap: bool, discover_robots: bool) {
        // First add root URL
        self.init().await;

        // Optionally discover from sitemap
        if discover_sitemap {
            self.discover_from_sitemap().await;
        }

        // Optionally discover from robots.txt
        if discover_robots {
            self.discover_from_robots().await;
        }
    }

    /// Discover URLs from sitemap.xml
    #[instrument(skip(self))]
    pub async fn discover_from_sitemap(&self) {
        let root = self.scope.root_url();

        // Common sitemap locations
        let sitemap_urls = vec![
            format!("{}/sitemap.xml", root),
            format!("{}/sitemap_index.xml", root),
            format!("{}/sitemap-index.xml", root),
        ];

        let client = reqwest::Client::new();

        for sitemap_url in sitemap_urls {
            match client.get(&sitemap_url).send().await {
                Ok(resp) if resp.status().is_success() => match resp.text().await {
                    Ok(xml) => {
                        info!("Found sitemap: {}", sitemap_url);
                        let urls = parse_sitemap(&xml);
                        for url in urls {
                            if self.scope.is_in_scope(&url) {
                                let canonical = self.scope.canonicalize(&url);
                                let entry = FrontierEntry::new(canonical, 0)
                                    .with_source(UrlSource::Sitemap);
                                self.frontier.push(entry).await;
                            }
                        }
                    }
                    Err(e) => warn!("Failed to read sitemap: {}", e),
                },
                _ => {}
            }
        }
    }

    /// Discover URLs from robots.txt
    #[instrument(skip(self))]
    pub async fn discover_from_robots(&self) {
        let root = self.scope.root_url();
        let robots_url = format!("{}/robots.txt", root);

        let client = reqwest::Client::new();

        match client.get(&robots_url).send().await {
            Ok(resp) if resp.status().is_success() => {
                match resp.text().await {
                    Ok(content) => {
                        info!("Found robots.txt: {}", robots_url);
                        // Parse robots.txt for allowed paths
                        // This is a simple implementation - extracts Allow: directives
                        for line in content.lines() {
                            let line = line.trim();
                            if line.starts_with("Allow:") {
                                let path = line.trim_start_matches("Allow:").trim();
                                if !path.is_empty() && path != "/" {
                                    let full_url = format!("{}{}", root, path);
                                    if self.scope.is_in_scope(&full_url) {
                                        let canonical = self.scope.canonicalize(&full_url);
                                        let entry = FrontierEntry::new(canonical, 0)
                                            .with_source(UrlSource::Robots);
                                        self.frontier.push(entry).await;
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => warn!("Failed to read robots.txt: {}", e),
                }
            }
            _ => {}
        }
    }

    /// Run the crawl and collect results
    #[instrument(skip(self))]
    pub async fn crawl(&mut self) -> Result<Vec<CrawlResult>, CrawlerError> {
        let mut pages_crawled = 0;
        info!(
            "Starting crawl with max_pages={} and max_depth={}",
            self.options.max_pages, self.options.max_depth
        );

        // Try to resume from previous state if enabled
        if self.options.persistence.resume {
            self.resume().await;
        }

        // Fetch robots.txt for the root domain
        if self.options.politeness.respect_robots_txt {
            let root = self.scope.root_url().to_string();
            if let Err(e) = self.robots.fetch(&root).await {
                warn!("Failed to fetch robots.txt: {}", e);
            }
        }

        while !self.frontier.is_empty().await {
            // Check max pages limit
            if pages_crawled >= self.options.max_pages {
                info!("Reached max pages limit ({})", self.options.max_pages);
                break;
            }

            // Check if we're in backoff mode
            if self.backoff_level > 0 && self.options.politeness.backoff_on_error {
                let backoff = self.rate_limiter.calculate_backoff(self.backoff_level);
                debug!("Backing off for {}ms", backoff.as_millis());
                sleep(backoff).await;
            }

            // Get batch of URLs to process
            let batch_size = self.options.concurrency.min(10);
            let mut batch = Vec::new();

            for _ in 0..batch_size {
                if let Some(entry) = self.frontier.pop().await {
                    batch.push(entry);
                }
            }

            if batch.is_empty() {
                break;
            }

            // Process batch
            for entry in batch {
                // Check depth limit
                if entry.depth > self.options.max_depth {
                    continue;
                }

                // Apply scope filter
                if !self.scope.is_in_scope(&entry.url) {
                    continue;
                }

                // Apply robots.txt compliance
                if self.options.politeness.respect_robots_txt {
                    if !self.robots.is_allowed(&entry.url).await {
                        info!("Blocked by robots.txt: {}", entry.url);
                        continue;
                    }

                    // Wait for crawl-delay if specified
                    self.robots.wait_until_allowed(&entry.url).await;
                }

                // Apply domain rate limiting (politeness)
                if let Ok(domain) = url::Url::parse(&entry.url)
                    && let Some(host) = domain.host_str()
                {
                    self.rate_limiter.wait_for(host).await;
                }

                // Crawl the page
                let result = self.crawl_page(&entry).await;

                // Handle errors - increase backoff
                if result.error.is_some() {
                    self.error_count += 1;
                    self.backoff_level = (self.backoff_level + 1).min(10);
                } else {
                    // Success - decrease backoff
                    self.backoff_level = self.backoff_level.saturating_sub(1);
                }

                self.results.push(result.clone());

                // Send to channel if set (for incremental saving)
                if let Some(ref sender) = self.options.result_sender {
                    let _ = sender.send(result).await;
                }

                // Persist state periodically
                if self.options.persistence.enabled
                    && pages_crawled > 0
                    && pages_crawled % self.options.persistence.save_interval == 0
                {
                    self.save_state(pages_crawled).await;
                }

                pages_crawled += 1;

                // Apply rate limiting from options
                if let Some(rate) = self.options.rate_limit {
                    let delay = Duration::from_millis(1000 / rate);
                    sleep(delay).await;
                } else {
                    sleep(Duration::from_millis(self.options.delay_ms)).await;
                }
            }
        }

        // Final save
        if self.options.persistence.enabled {
            self.save_state(pages_crawled).await;
        }

        info!("Crawl completed: {} pages crawled", pages_crawled);
        Ok(std::mem::take(&mut self.results))
    }

    /// Save crawl state for resumption
    #[instrument(skip(self), fields(pages_crawled))]
    async fn save_state(&self, pages_crawled: usize) {
        if let (Some(state), Some(name)) = (&self.state, &self.options.persistence.state_name) {
            let root = self.scope.root_url().to_string();

            // Save frontier (top 1000 entries only to avoid huge files)
            let entries = self.frontier.peek_many(1000).await;
            let _ = state.save_frontier(name, &entries).await;

            // Save metadata
            let meta = CrawlMetadata {
                root_url: root,
                started_at: chrono::Utc::now(),
                last_saved: chrono::Utc::now(),
                pages_crawled,
                pages_queued: self.frontier.len().await,
                error_count: self.error_count,
            };
            let _ = state.save_metadata(name, &meta).await;

            // Append latest results
            let _ = state.save_results(name, &self.results).await;

            info!(
                "Saved state: {} pages crawled, {} queued",
                pages_crawled,
                self.frontier.len().await
            );
        }
    }

    /// Resume from previous state
    #[instrument(skip(self))]
    async fn resume(&mut self) {
        if let (Some(state), Some(name)) = (&self.state, &self.options.persistence.state_name) {
            // Load frontier
            if let Ok(entries) = state.load_frontier(name).await {
                for entry in entries {
                    self.frontier.push(entry).await;
                }
                let queued = self.frontier.len().await;
                if queued > 0 {
                    info!("Resumed with {} queued URLs", queued);
                }
            }

            // Load metadata
            if let Ok(Some(meta)) = state.load_metadata(name).await {
                info!(
                    "Previous crawl: {} pages, {} errors",
                    meta.pages_crawled, meta.error_count
                );
            }

            // Load existing results
            if let Ok(results) = state.load_results(name).await
                && !results.is_empty()
            {
                info!("Loaded {} previous results", results.len());
            }
        }
    }

    /// Crawl a single page with optional AI assistance
    #[instrument(skip(self), fields(url = %entry.url, depth = entry.depth))]
    async fn crawl_page(&mut self, entry: &FrontierEntry) -> CrawlResult {
        let mut result = CrawlResult::new(entry.url.clone())
            .with_depth(entry.depth)
            .with_source(entry.source.to_string());

        if let Some(ref source_url) = entry.source_url {
            result = result.with_source_url(source_url.clone());
        }

        // Check if we should use AI-assisted detection
        let _use_ai_detection = self.options.extract_data && self.options.ai_provider.is_some();

        // First, try HTTP fetch
        let scraper = crate::Scraper::new();

        match scraper.scrape(&entry.url).await {
            Ok(resp) => {
                result = result.with_status(resp.status);
                result = result.with_links_found(resp.links().len());
                result = result.with_forms_found(count_forms(&resp));

                if let Some(title) = resp.title() {
                    result = result.with_title(title);
                }

                // Heuristic: analyze page for JS indicators
                let (is_js_heavy, needs_interaction) = self.analyze_page_complexity(&resp);
                result = result.with_js_heavy(is_js_heavy);
                result = result.with_needs_interaction(needs_interaction);

                // Decide whether to use browser based on escalation mode
                let should_use_browser = match self.options.browser_escalation {
                    BrowserEscalation::Always => true,
                    BrowserEscalation::Never => false,
                    BrowserEscalation::Auto => {
                        // Use browser if page is JS-heavy or needs interaction
                        is_js_heavy || needs_interaction
                    }
                };

                if should_use_browser {
                    info!(
                        "Using browser for: {} (JS-heavy: {}, needs interaction: {})",
                        entry.url, is_js_heavy, needs_interaction
                    );
                    result = result.with_used_browser(true);

                    // Re-fetch with browser to get rendered HTML
                    match self.fetch_with_browser(&entry.url).await {
                        Ok(browser_html) => {
                            // Use browser-rendered HTML for URL parsing
                            // The browser HTML has already been processed via analyze_page_complexity above
                            // We still use the HTTP response for basic info, but URL discovery uses browser
                            let new_entries =
                                self.parser.parse_from_html(&browser_html, &entry.url);
                            for mut new_entry in new_entries {
                                new_entry.depth = entry.depth + 1;
                                if self.scope.is_in_scope(&new_entry.url) {
                                    let canonical = self.scope.canonicalize(&new_entry.url);
                                    new_entry.url = canonical;
                                    self.frontier.push(new_entry).await;
                                }
                            }
                            return result;
                        }
                        Err(e) => {
                            warn!("Browser fetch failed, falling back to HTTP: {}", e);
                            // Continue with HTTP-based parsing
                        }
                    }
                }

                // If extract_data is enabled, use AI to extract
                if self.options.extract_data && self.options.ai_provider.is_some() {
                    // For now, just mark as ready for extraction
                    // Full AI extraction would require async spawn
                    if let Some(ref schema) = self.options.ai_schema {
                        debug!("Ready for AI extraction with schema: {}", schema);
                    }
                }

                // Parse new URLs for frontier
                let new_entries = self.parser.parse(&resp, &entry.url);
                for mut new_entry in new_entries {
                    new_entry.depth = entry.depth + 1;
                    if self.scope.is_in_scope(&new_entry.url) {
                        let canonical = self.scope.canonicalize(&new_entry.url);
                        new_entry.url = canonical;
                        self.frontier.push(new_entry).await;
                    }
                }

                Some(resp)
            }
            Err(e) => {
                result = result.with_error(e.to_string());
                None
            }
        };

        result
    }

    /// Analyze page complexity to determine if browser is needed
    fn analyze_page_complexity(&self, response: &crate::Response) -> (bool, bool) {
        let html = &response.html;
        let forms_count = count_forms(response);

        // JS-heavy indicators
        let js_indicators = [
            html.contains("angular"),
            html.contains("react"),
            html.contains("vue."),
            html.contains("svelte"),
            html.contains("Backbone"),
            html.contains("Ember"),
            html.contains("webcomponents"),
            html.contains("SPA") || html.contains("spa"),
            // Single page app patterns
            html.contains("router") && html.contains("pushState"),
            // Heavy JS usage
            html.matches("import ").count() > 5,
            html.contains("webpack"),
            html.contains("babel"),
        ];

        let is_js_heavy =
            js_indicators.iter().filter(|&&x| x).count() >= 1 || (html.len() > 500_000); // Very large page

        // Needs interaction indicators
        let needs_interaction = [
            forms_count > 3, // Many forms
            html.contains("autocomplete"),
            html.contains("type=\"search\""),
            html.contains("infinite scroll"),
            html.contains("load more"),
            html.contains("show more"),
            html.contains("modal"),
            html.contains("tab-content"),
        ];

        let needs_interaction =
            needs_interaction.iter().filter(|&&x| x).count() >= 1 || forms_count > 0;

        (is_js_heavy, needs_interaction)
    }

    /// Fetch a page using headless browser and return rendered HTML
    async fn fetch_with_browser(&self, url: &str) -> Result<String, String> {
        use scrapio_browser::{ChromeDriverSession, StealthBrowser, StealthConfig, StealthLevel};

        // Start ChromeDriver session
        let driver = ChromeDriverSession::start()
            .await
            .map_err(|e| format!("Failed to start ChromeDriver: {}", e))?;

        // Create browser with stealth configuration
        let mut browser = StealthBrowser::with_webdriver(driver.webdriver_url())
            .headless(true)
            .stealth(StealthConfig::new(StealthLevel::Full))
            .timeout(std::time::Duration::from_secs(30))
            .init()
            .await
            .map_err(|e| format!("Failed to initialize browser: {}", e))?;

        // Enable network request capture
        let _ = browser.enable_network_capture().await;

        // Navigate to URL
        browser
            .goto(url)
            .await
            .map_err(|e| format!("Failed to navigate to {}: {}", url, e))?;

        // Wait a bit for JS to render and network requests to complete
        tokio::time::sleep(std::time::Duration::from_millis(2000)).await;

        // Get network requests
        match browser.get_network_requests().await {
            Ok(requests) => {
                if !requests.is_empty() {
                    debug!("Captured {} network requests", requests.len());
                }
            }
            Err(e) => {
                warn!("Network capture error: {}", e);
            }
        }

        // Get rendered HTML
        let html = browser
            .html()
            .await
            .map_err(|e| format!("Failed to get HTML: {}", e))?;

        // Close browser
        let _ = browser.close().await;

        Ok(html)
    }

    /// Get statistics about the crawl
    pub async fn stats(&self) -> FrontierStats {
        self.frontier.stats().await
    }

    /// Get the results collected so far
    pub fn results(&self) -> &[CrawlResult] {
        &self.results
    }
}

/// Crawler-related errors
#[derive(Debug, thiserror::Error)]
pub enum CrawlerError {
    #[error("Scope error: {0}")]
    ScopeError(#[from] scope::ScopeError),

    #[error("HTTP error: {0}")]
    HttpError(#[from] scrapio_core::error::ScrapioError),
}
