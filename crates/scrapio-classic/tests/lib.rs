//! Tests for scrapio-classic lib module

use scraper::Html;
use scrapio_classic::Response;

fn create_test_response(html: &str, url: &str) -> Response {
    let document = Html::parse_document(html);
    Response {
        url: url.to_string(),
        status: 200,
        html: html.to_string(),
        document,
    }
}

#[test]
fn test_response_title() {
    let html = r#"<html><head><title>Test Page</title></head><body></body></html>"#;
    let response = create_test_response(html, "https://www.rust-lang.org");
    assert_eq!(response.title(), Some("Test Page".to_string()));
}

#[test]
fn test_response_title_missing() {
    let html = r#"<html><head></head><body></body></html>"#;
    let response = create_test_response(html, "https://www.rust-lang.org");
    assert_eq!(response.title(), None);
}

#[test]
fn test_response_links() {
    let html = r#"<html><body><a href="https://www.rust-lang.org">Link1</a><a href="/path">Link2</a></body></html>"#;
    let response = create_test_response(html, "https://www.rust-lang.org");
    let links = response.links();
    assert_eq!(links.len(), 2);
    assert!(links.contains(&"https://www.rust-lang.org".to_string()));
    assert!(links.contains(&"/path".to_string()));
}

#[test]
fn test_response_select() {
    let html = r#"<html><body><h1>Title</h1><h2>Subtitle</h2><p>Content</p></body></html>"#;
    let response = create_test_response(html, "https://www.rust-lang.org");
    let h1_elements = response.select("h1");
    assert_eq!(h1_elements.len(), 1);
}

#[test]
fn test_response_select_invalid_selector() {
    let html = r#"<html><body><p>Content</p></body></html>"#;
    let response = create_test_response(html, "https://www.rust-lang.org");
    let elements = response.select("invalid[ selector");
    assert_eq!(elements.len(), 0);
}
