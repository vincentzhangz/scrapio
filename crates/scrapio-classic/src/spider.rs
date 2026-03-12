//! Spider System - crawling framework
//!
//! Provides a trait-based system for defining spiders that can crawl
//! websites and extract structured data.

use crate::{Response, Scraper};

/// A Spider is the main component for defining crawl behavior.
/// It defines what URLs to crawl and how to parse the responses.
pub trait Spider: Send + Sync {
    /// The name of the spider. Must be unique.
    fn name(&self) -> &str;

    /// The starting URLs for the spider.
    fn start_urls(&self) -> Vec<String>;

    /// Parse the response and return extracted items or new requests.
    fn parse(&self, response: &Response) -> SpiderOutput;
}

/// Output from a spider's parse method
#[derive(Debug, Clone)]
pub enum SpiderOutput {
    /// Extracted items (raw data)
    Items(Vec<Item>),
    /// New URLs to follow
    Requests(Vec<Request>),
    /// Both items and requests
    Both(Vec<Item>, Vec<Request>),
    /// No output
    None,
}

/// A Request represents a URL to be crawled
#[derive(Debug, Clone)]
pub struct Request {
    pub url: String,
    pub method: Method,
    pub callback: Option<String>,
    pub priority: i32,
}

impl Request {
    pub fn get(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            method: Method::GET,
            callback: None,
            priority: 0,
        }
    }

    pub fn post(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            method: Method::POST,
            callback: None,
            priority: 0,
        }
    }

    pub fn with_callback(mut self, callback: impl Into<String>) -> Self {
        self.callback = Some(callback.into());
        self
    }

    pub fn with_priority(mut self, priority: i32) -> Self {
        self.priority = priority;
        self
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Method {
    GET,
    POST,
}

/// An Item represents scraped data from web pages
pub type Item = serde_json::Map<String, serde_json::Value>;

/// SpiderRunner executes spiders and manages the crawling process
pub struct SpiderRunner {
    scraper: Scraper,
    max_depth: usize,
    concurrent_requests: usize,
    download_delay: std::time::Duration,
}

impl SpiderRunner {
    pub fn new() -> Self {
        Self {
            scraper: Scraper::new(),
            max_depth: 3,
            concurrent_requests: 5,
            download_delay: std::time::Duration::from_millis(500),
        }
    }

    pub fn with_max_depth(mut self, depth: usize) -> Self {
        self.max_depth = depth;
        self
    }

    pub fn with_concurrent_requests(mut self, count: usize) -> Self {
        self.concurrent_requests = count;
        self
    }

    pub fn with_download_delay(mut self, delay: std::time::Duration) -> Self {
        self.download_delay = delay;
        self
    }

    /// Run a spider and collect all items
    pub async fn run<S: Spider>(&self, spider: &S) -> Vec<Item> {
        let mut all_items = Vec::new();
        let mut pending_requests: Vec<(String, usize)> = spider
            .start_urls()
            .into_iter()
            .map(|url| (url, 0))
            .collect();
        let mut seen_urls = std::collections::HashSet::new();

        while !pending_requests.is_empty() {
            // Take up to concurrent_requests URLs
            let batch: Vec<(String, usize)> = pending_requests
                .drain(..self.concurrent_requests.min(pending_requests.len()))
                .collect();

            for (url, depth) in batch {
                if seen_urls.contains(&url) || depth > self.max_depth {
                    continue;
                }
                seen_urls.insert(url.clone());

                // Download delay
                tokio::time::sleep(self.download_delay).await;

                // Fetch the page
                match self.scraper.scrape(&url).await {
                    Ok(response) => {
                        let output = spider.parse(&response);

                        let enqueue =
                            |reqs: Vec<Request>,
                             seen: &std::collections::HashSet<String>,
                             pending: &mut Vec<(String, usize)>| {
                                for req in reqs {
                                    if !seen.contains(&req.url) {
                                        pending.push((req.url, depth + 1));
                                    }
                                }
                            };

                        match output {
                            SpiderOutput::Items(items) => {
                                all_items.extend(items);
                            }
                            SpiderOutput::Requests(reqs) => {
                                enqueue(reqs, &seen_urls, &mut pending_requests);
                            }
                            SpiderOutput::Both(items, reqs) => {
                                all_items.extend(items);
                                enqueue(reqs, &seen_urls, &mut pending_requests);
                            }
                            SpiderOutput::None => {}
                        }
                    }
                    Err(e) => {
                        eprintln!("Failed to scrape {}: {}", url, e);
                    }
                }
            }
        }

        all_items
    }
}

impl Default for SpiderRunner {
    fn default() -> Self {
        Self::new()
    }
}

/// Utility to create an empty item
pub fn make_item() -> Item {
    Item::new()
}

/// Macro to create items with key-value pairs
#[macro_export]
macro_rules! item {
    ($($key:expr => $value:expr),* $(,)?) => {{
        let mut item = serde_json::Map::new();
        $(item.insert($key.to_string(), $value);)*
        item
    }};
}
