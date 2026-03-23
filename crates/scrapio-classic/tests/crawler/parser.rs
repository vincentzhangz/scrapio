//! Tests for parser module

use scrapio_classic::crawler::{resolve_url, is_valid_nav_url, extract_refresh_url};

#[test]
fn test_resolve_url_absolute() {
    assert_eq!(
        resolve_url("http://example.com/", "https://other.com/page"),
        "https://other.com/page"
    );
}

#[test]
fn test_resolve_url_relative() {
    assert_eq!(
        resolve_url("http://example.com/page/", "about"),
        "http://example.com/page/about"
    );
}

#[test]
fn test_resolve_url_fragment() {
    assert_eq!(
        resolve_url("http://example.com/page", "#section"),
        "http://example.com/page"
    );
}

#[test]
fn test_resolve_url_root() {
    assert_eq!(
        resolve_url("http://example.com", "/about"),
        "http://example.com/about"
    );
}

#[test]
fn test_is_valid_nav_url() {
    assert!(is_valid_nav_url("http://example.com"));
    assert!(is_valid_nav_url("https://example.com"));
    assert!(is_valid_nav_url("/path"));
    assert!(!is_valid_nav_url(""));
    assert!(!is_valid_nav_url("javascript:alert(1)"));
    assert!(!is_valid_nav_url("mailto:test@example.com"));
    assert!(!is_valid_nav_url("data:text/html,<h1>"));
}

#[test]
fn test_extract_refresh_url_direct() {
    assert_eq!(
        extract_refresh_url("http://example.com"),
        Some("http://example.com".to_string())
    );
}

#[test]
fn test_extract_refresh_url_with_timeout() {
    assert_eq!(
        extract_refresh_url("5;URL=http://example.com/page"),
        Some("http://example.com/page".to_string())
    );
}