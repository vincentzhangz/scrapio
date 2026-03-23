//! Tests for crawler types module

use scrapio_classic::crawler::{
    BrowserEscalation, CrawlOptions, CrawlResult, DiscoverOptions, OutputFormat, OutputOptions,
    PersistenceOptions, PolitenessOptions, Scope, ScopeMode, escape_csv,
};

#[test]
fn test_crawl_options_default() {
    let options = CrawlOptions::default();
    assert_eq!(options.max_depth, 2);
    assert_eq!(options.max_pages, 100);
    assert_eq!(options.concurrency, 5);
}

#[test]
fn test_crawl_options_new() {
    let options = CrawlOptions::new();
    assert_eq!(options.max_depth, 2);
}

#[test]
fn test_crawl_options_with_max_depth() {
    let options = CrawlOptions::new().with_max_depth(5);
    assert_eq!(options.max_depth, 5);
}

#[test]
fn test_crawl_options_with_max_pages() {
    let options = CrawlOptions::new().with_max_pages(1000);
    assert_eq!(options.max_pages, 1000);
}

#[test]
fn test_crawl_options_with_rate_limit() {
    let options = CrawlOptions::new().with_rate_limit(10);
    assert_eq!(options.rate_limit, Some(10));
}

#[test]
fn test_crawl_options_with_browser_escalation() {
    let options = CrawlOptions::new().with_browser_escalation(BrowserEscalation::Always);
    assert_eq!(options.browser_escalation, BrowserEscalation::Always);
}

#[test]
fn test_crawl_options_polite() {
    let options = CrawlOptions::new().polite();
    assert_eq!(options.politeness.min_delay_ms, 1000);
}

#[test]
fn test_crawl_options_aggressive() {
    let options = CrawlOptions::new().aggressive();
    assert_eq!(options.politeness.min_delay_ms, 50);
    assert!(!options.politeness.respect_robots_txt);
}

#[test]
fn test_crawl_options_persist_as() {
    let options = CrawlOptions::new().persist_as("my_crawl");
    assert!(options.persistence.enabled);
    assert_eq!(options.persistence.state_name, Some("my_crawl".to_string()));
}

#[test]
fn test_crawl_options_ignore_robots_txt() {
    let options = CrawlOptions::new().ignore_robots_txt();
    assert!(!options.politeness.respect_robots_txt);
}

#[test]
fn test_scope_default() {
    let scope = Scope::default();
    assert_eq!(scope.mode, ScopeMode::Domain);
    assert!(scope.include_subdomains);
}

#[test]
fn test_scope_new() {
    let scope = Scope::new(ScopeMode::Host);
    assert_eq!(scope.mode, ScopeMode::Host);
}

#[test]
fn test_scope_host() {
    let scope = Scope::host();
    assert_eq!(scope.mode, ScopeMode::Host);
}

#[test]
fn test_scope_domain() {
    let scope = Scope::domain();
    assert_eq!(scope.mode, ScopeMode::Domain);
}

#[test]
fn test_scope_subdomain() {
    let scope = Scope::subdomain();
    assert_eq!(scope.mode, ScopeMode::Subdomain);
}

#[test]
fn test_scope_with_include_subdomains() {
    let scope = Scope::domain().with_include_subdomains(false);
    assert!(!scope.include_subdomains);
}

#[test]
fn test_politeness_options_default() {
    let opts = PolitenessOptions::default();
    assert_eq!(opts.min_delay_ms, 100);
    assert_eq!(opts.max_delay_ms, 5000);
    assert!(opts.respect_robots_txt);
}

#[test]
fn test_politeness_options_aggressive() {
    let opts = PolitenessOptions::aggressive();
    assert_eq!(opts.min_delay_ms, 50);
    assert!(!opts.respect_robots_txt);
}

#[test]
fn test_politeness_options_polite() {
    let opts = PolitenessOptions::polite();
    assert_eq!(opts.min_delay_ms, 1000);
    assert!(opts.respect_robots_txt);
}

#[test]
fn test_persistence_options_default() {
    let opts = PersistenceOptions::default();
    assert!(!opts.enabled);
    assert_eq!(opts.state_dir, ".scrapio-state");
    assert_eq!(opts.save_interval, 10);
}

#[test]
fn test_persistence_options_with_name() {
    let opts = PersistenceOptions::default().with_name("test");
    assert!(opts.enabled);
    assert_eq!(opts.state_name, Some("test".to_string()));
}

#[test]
fn test_persistence_options_with_resume() {
    let opts = PersistenceOptions::default().with_resume();
    assert!(opts.resume);
}

#[test]
fn test_discover_options_default() {
    let opts = DiscoverOptions::default();
    assert!(opts.anchors);
    assert!(opts.forms);
    assert!(opts.scripts);
}

#[test]
fn test_output_options_default() {
    let opts = OutputOptions::default();
    assert_eq!(opts.format, OutputFormat::Text);
}

#[test]
fn test_browser_escalation_variants() {
    assert!(matches!(BrowserEscalation::Never, BrowserEscalation::Never));
    assert!(matches!(BrowserEscalation::Auto, BrowserEscalation::Auto));
    assert!(matches!(
        BrowserEscalation::Always,
        BrowserEscalation::Always
    ));
}

#[test]
fn test_scope_mode_variants() {
    assert!(matches!(ScopeMode::Host, ScopeMode::Host));
    assert!(matches!(ScopeMode::Domain, ScopeMode::Domain));
    assert!(matches!(ScopeMode::Subdomain, ScopeMode::Subdomain));
    assert!(matches!(ScopeMode::Custom, ScopeMode::Custom));
}

#[test]
fn test_crawl_result_new() {
    let result = CrawlResult::new("http://example.com".to_string());
    assert_eq!(result.url, "http://example.com");
    assert_eq!(result.depth, 0);
    assert_eq!(result.status, 0);
}

#[test]
fn test_crawl_result_with_status() {
    let result = CrawlResult::new("http://example.com".to_string()).with_status(200);
    assert_eq!(result.status, 200);
}

#[test]
fn test_crawl_result_with_title() {
    let result = CrawlResult::new("http://example.com".to_string())
        .with_title("Test Title".to_string());
    assert_eq!(result.title, Some("Test Title".to_string()));
}

#[test]
fn test_crawl_result_with_error() {
    let result = CrawlResult::new("http://example.com".to_string())
        .with_error("Error message".to_string());
    assert_eq!(result.error, Some("Error message".to_string()));
}

#[test]
fn test_crawl_result_to_json() {
    let result = CrawlResult::new("http://example.com".to_string());
    let json = result.to_json();
    assert!(json.contains("http://example.com"));
}

#[test]
fn test_crawl_result_csv_header() {
    let header = CrawlResult::csv_header();
    assert!(header.contains("timestamp"));
    assert!(header.contains("url"));
}

#[test]
fn test_escape_csv_simple() {
    assert_eq!(escape_csv("hello"), "hello");
}

#[test]
fn test_escape_csv_with_comma() {
    assert_eq!(escape_csv("hello,world"), "\"hello,world\"");
}

#[test]
fn test_escape_csv_with_quotes() {
    // Input: he said "hi" (with literal quotes)
    // Output: "he said ""hi""" (quotes escaped by doubling, wrapped in quotes)
    assert_eq!(escape_csv("he said \"hi\""), "\"he said \"\"hi\"\"\"");
}

#[test]
fn test_escape_csv_with_newline() {
    assert_eq!(escape_csv("hello\nworld"), "\"hello\nworld\"");
}