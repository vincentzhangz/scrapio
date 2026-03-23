//! Tests for scope module

use regex::Regex;
use scrapio_classic::crawler::{Scope, ScopeValidator};

#[test]
fn test_scope_host_mode() {
    let scope = Scope::host();
    let validator = ScopeValidator::new("https://www.example.com", scope).unwrap();

    // Same host
    assert!(validator.is_in_scope("https://www.example.com/about"));
    assert!(validator.is_in_scope("https://www.example.com/page.html"));

    // Different host
    assert!(!validator.is_in_scope("https://api.example.com/page"));
    assert!(!validator.is_in_scope("https://example.com/page"));
    assert!(!validator.is_in_scope("https://other.com/page"));
}

#[test]
fn test_scope_domain_mode_with_subdomains() {
    let mut scope = Scope::domain().with_include_subdomains(true);
    scope.regex_exclude.push(Regex::new(r".*\.png$").unwrap());

    let validator = ScopeValidator::new("https://www.example.com", scope).unwrap();

    // Same domain with subdomains
    assert!(validator.is_in_scope("https://www.example.com/page"));
    assert!(validator.is_in_scope("https://api.example.com/page"));
    assert!(validator.is_in_scope("https://deep.sub.example.com/page"));

    // Different domain
    assert!(!validator.is_in_scope("https://example.org/page"));

    // Excluded by regex
    assert!(!validator.is_in_scope("https://www.example.com/image.png"));
}

#[test]
fn test_canonicalize() {
    let scope = Scope::domain();
    let validator = ScopeValidator::new("https://www.example.com", scope).unwrap();

    // Path normalization
    let canonical = validator.canonicalize("https://www.example.com/page/");
    assert_eq!(canonical, "https://www.example.com/page");

    // Case normalization
    let canonical = validator.canonicalize("HTTPS://WWW.EXAMPLE.COM/PAGE");
    assert_eq!(canonical, "https://www.example.com/page");

    // Query params
    let canonical = validator.canonicalize("https://www.example.com/page?id=123");
    assert_eq!(canonical, "https://www.example.com/page?id=123");
}