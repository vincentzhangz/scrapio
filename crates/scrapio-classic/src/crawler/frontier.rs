//! Frontier - URL queue management for crawling

use std::collections::{HashSet, VecDeque};
use std::sync::Arc;
use tokio::sync::RwLock;

/// URL source type for tracking discovered URLs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UrlSource {
    /// Found in anchor tag
    Anchor,
    /// Found in form action
    Form,
    /// Found in script src
    Script,
    /// Found in iframe src
    Iframe,
    /// Found in meta tag
    Meta,
    /// Found in canonical link
    Canonical,
    /// Found in redirect
    Redirect,
    /// Found in sitemap.xml
    Sitemap,
    /// Found in robots.txt
    Robots,
}

impl std::fmt::Display for UrlSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UrlSource::Anchor => write!(f, "a"),
            UrlSource::Form => write!(f, "form"),
            UrlSource::Script => write!(f, "script"),
            UrlSource::Iframe => write!(f, "iframe"),
            UrlSource::Meta => write!(f, "meta"),
            UrlSource::Canonical => write!(f, "canonical"),
            UrlSource::Redirect => write!(f, "redirect"),
            UrlSource::Sitemap => write!(f, "sitemap"),
            UrlSource::Robots => write!(f, "robots"),
        }
    }
}

/// A URL entry in the crawl queue
#[derive(Debug, Clone)]
pub struct FrontierEntry {
    /// The URL to crawl
    pub url: String,
    /// Parent URL that discovered this one
    pub source_url: Option<String>,
    /// Crawl depth
    pub depth: usize,
    /// Source type (how this URL was discovered)
    pub source: UrlSource,
    /// HTML attribute where URL was found
    pub attribute: String,
}

impl FrontierEntry {
    pub fn new(url: String, depth: usize) -> Self {
        Self {
            url,
            source_url: None,
            depth,
            source: UrlSource::Anchor,
            attribute: "href".to_string(),
        }
    }

    pub fn with_source(mut self, source: UrlSource) -> Self {
        self.source = source;
        self
    }

    pub fn with_attribute(mut self, attr: impl Into<String>) -> Self {
        self.attribute = attr.into();
        self
    }

    pub fn with_source_url(mut self, url: String) -> Self {
        self.source_url = Some(url);
        self
    }
}

/// Frontier manages the crawling queue and deduplication
#[derive(Clone)]
pub struct Frontier {
    queue: Arc<RwLock<VecDeque<FrontierEntry>>>,
    seen: Arc<RwLock<HashSet<String>>>,
    stats: Arc<RwLock<FrontierStats>>,
}

#[derive(Debug, Default, Clone)]
pub struct FrontierStats {
    pub queued: usize,
    pub processed: usize,
    pub seen_count: usize,
    pub skipped_duplicates: usize,
}

impl Frontier {
    pub fn new() -> Self {
        Self {
            queue: Arc::new(RwLock::new(VecDeque::new())),
            seen: Arc::new(RwLock::new(HashSet::new())),
            stats: Arc::new(RwLock::new(FrontierStats::default())),
        }
    }

    /// Add a URL to the frontier if not already seen
    pub async fn push(&self, entry: FrontierEntry) -> bool {
        // Check if already seen (deduplication)
        if self.is_seen(&entry.url).await {
            let mut stats = self.stats.write().await;
            stats.skipped_duplicates += 1;
            return false;
        }

        // Mark as seen
        self.mark_seen(entry.url.clone()).await;

        // Add to queue
        let mut queue = self.queue.write().await;
        queue.push_back(entry);

        let mut stats = self.stats.write().await;
        stats.queued = queue.len();

        true
    }

    /// Add multiple entries at once
    pub async fn push_batch(&self, entries: Vec<FrontierEntry>) -> usize {
        let mut added = 0;
        for entry in entries {
            if self.push(entry).await {
                added += 1;
            }
        }
        added
    }

    /// Pop the next URL from the frontier
    pub async fn pop(&self) -> Option<FrontierEntry> {
        let mut queue = self.queue.write().await;
        let entry = queue.pop_front();

        if entry.is_some() {
            let mut stats = self.stats.write().await;
            stats.queued = queue.len();
            stats.processed += 1;
        }

        entry
    }

    /// Peek at the next URL without removing it
    pub async fn peek(&self) -> Option<FrontierEntry> {
        let queue = self.queue.read().await;
        queue.front().cloned()
    }

    /// Check if frontier is empty
    pub async fn is_empty(&self) -> bool {
        let queue = self.queue.read().await;
        queue.is_empty()
    }

    /// Get the number of queued URLs
    pub async fn len(&self) -> usize {
        let queue = self.queue.read().await;
        queue.len()
    }

    /// Check if a URL has been seen
    pub async fn is_seen(&self, url: &str) -> bool {
        let seen = self.seen.read().await;
        seen.contains(url)
    }

    /// Mark a URL as seen
    pub async fn mark_seen(&self, url: String) {
        let mut seen = self.seen.write().await;
        seen.insert(url);
        let mut stats = self.stats.write().await;
        stats.seen_count = seen.len();
    }

    /// Get current statistics
    pub async fn stats(&self) -> FrontierStats {
        let stats = self.stats.read().await;
        stats.clone()
    }

    /// Clear the frontier
    pub async fn clear(&self) {
        let mut queue = self.queue.write().await;
        queue.clear();
        let mut seen = self.seen.write().await;
        seen.clear();
        let mut stats = self.stats.write().await;
        *stats = FrontierStats::default();
    }
}

impl Default for Frontier {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_frontier_basic_operations() {
        let frontier = Frontier::new();

        // Initially empty
        assert!(frontier.is_empty().await);

        // Push an entry
        let entry = FrontierEntry::new("https://example.com".to_string(), 0);
        let added = frontier.push(entry).await;
        assert!(added);
        assert!(!frontier.is_empty().await);

        // Push duplicate should fail
        let entry = FrontierEntry::new("https://example.com".to_string(), 1);
        let added = frontier.push(entry).await;
        assert!(!added);

        // Pop the entry
        let popped = frontier.pop().await;
        assert!(popped.is_some());
        assert_eq!(popped.unwrap().url, "https://example.com");

        // Now empty again
        assert!(frontier.is_empty().await);
    }

    #[tokio::test]
    async fn test_frontier_stats() {
        let frontier = Frontier::new();

        frontier
            .push(FrontierEntry::new("https://example.com/1".to_string(), 0))
            .await;
        frontier
            .push(FrontierEntry::new("https://example.com/2".to_string(), 0))
            .await;
        frontier
            .push(FrontierEntry::new("https://example.com/1".to_string(), 1))
            .await; // duplicate

        let stats = frontier.stats().await;
        assert_eq!(stats.queued, 2);
        assert_eq!(stats.skipped_duplicates, 1);
        assert_eq!(stats.seen_count, 2);
    }
}
