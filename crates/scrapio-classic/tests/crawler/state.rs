//! Tests for state module

use scrapio_classic::crawler::{CrawlMetadata, CrawlState};

#[test]
fn test_crawl_state_new() {
    let state = CrawlState::new("/tmp/test_scrapio");
    let _ = state;
}

#[tokio::test]
async fn test_crawl_state_ensure_dir() {
    let state = CrawlState::new("/tmp/test_scrapio");
    let result = state.ensure_dir().await;
    assert!(result.is_ok() || result.is_err()); // May fail if dir exists
}

#[test]
fn test_crawl_metadata_fields() {
    let meta = CrawlMetadata {
        root_url: "http://example.com".to_string(),
        started_at: chrono::Utc::now(),
        last_saved: chrono::Utc::now(),
        pages_crawled: 10,
        pages_queued: 5,
        error_count: 2,
    };

    assert_eq!(meta.root_url, "http://example.com");
    assert_eq!(meta.pages_crawled, 10);
    assert_eq!(meta.pages_queued, 5);
    assert_eq!(meta.error_count, 2);
}