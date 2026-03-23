//! Tests for frontier module

use scrapio_classic::crawler::{Frontier, FrontierEntry, UrlSource};

#[tokio::test]
async fn test_frontier_basic_operations() {
    let frontier = Frontier::new();

    // Push an entry
    let entry = FrontierEntry::new("http://example.com".to_string(), 0);
    let result = frontier.push(entry).await;
    assert!(result);

    // Check not empty
    assert!(!frontier.is_empty().await);

    // Pop entry
    let popped = frontier.pop().await;
    assert!(popped.is_some());
    assert_eq!(popped.unwrap().url, "http://example.com");
}

#[tokio::test]
async fn test_frontier_push_batch() {
    let frontier = Frontier::new();

    let entries = vec![
        FrontierEntry::new("http://example.com/1".to_string(), 0),
        FrontierEntry::new("http://example.com/2".to_string(), 0),
        FrontierEntry::new("http://example.com/3".to_string(), 0),
    ];

    let count = frontier.push_batch(entries).await;
    assert_eq!(count, 3);
}

#[tokio::test]
async fn test_frontier_with_source() {
    let entry = FrontierEntry::new("http://example.com".to_string(), 0)
        .with_source(UrlSource::RobotsTxt);

    assert_eq!(entry.url, "http://example.com");
    assert_eq!(entry.source, Some(UrlSource::RobotsTxt));
}