//! Classic scraping for Scrapio

use scraper::{Html, Selector};
use scrapio_core::{ScrapioResult, error::*, http::HttpClient};

pub mod pipeline;
pub mod spider;

pub struct Scraper {
    http: HttpClient,
}

impl Scraper {
    pub fn new() -> Self {
        Self {
            http: HttpClient::new(),
        }
    }

    pub async fn scrape(&self, url: &str) -> ScrapioResult<Response> {
        if !scrapio_core::utils::url::is_valid(url) {
            return Err(ScrapioError::Parse(format!("Invalid URL: {}", url)));
        }

        let response = self.http.client().get(url).send().await?;
        let status = response.status().as_u16();
        let html = response.text().await?;
        let document = Html::parse_document(&html);

        Ok(Response {
            url: url.to_string(),
            status,
            html,
            document,
        })
    }
}

impl Default for Scraper {
    fn default() -> Self {
        Self::new()
    }
}

pub struct Response {
    pub url: String,
    pub status: u16,
    pub html: String,
    pub document: Html,
}

impl Response {
    pub fn title(&self) -> Option<String> {
        let selector = Selector::parse("title").ok()?;
        self.document
            .select(&selector)
            .next()
            .map(|el| el.inner_html().trim().to_string())
    }

    pub fn links(&self) -> Vec<String> {
        let selector = match Selector::parse("a[href]") {
            Ok(s) => s,
            Err(_) => return vec![],
        };

        self.document
            .select(&selector)
            .filter_map(|el| el.value().attr("href").map(|s| s.to_string()))
            .collect()
    }

    pub fn select(&self, sel: &str) -> Vec<scraper::ElementRef<'_>> {
        match Selector::parse(sel) {
            Ok(selector) => self.document.select(&selector).collect(),
            Err(_) => vec![],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use scraper::Html;

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
        let response = create_test_response(html, "https://example.com");
        assert_eq!(response.title(), Some("Test Page".to_string()));
    }

    #[test]
    fn test_response_title_missing() {
        let html = r#"<html><head></head><body></body></html>"#;
        let response = create_test_response(html, "https://example.com");
        assert_eq!(response.title(), None);
    }

    #[test]
    fn test_response_links() {
        let html = r#"<html><body><a href="https://example.com">Link1</a><a href="/path">Link2</a></body></html>"#;
        let response = create_test_response(html, "https://example.com");
        let links = response.links();
        assert_eq!(links.len(), 2);
        assert!(links.contains(&"https://example.com".to_string()));
        assert!(links.contains(&"/path".to_string()));
    }

    #[test]
    fn test_response_select() {
        let html = r#"<html><body><h1>Title</h1><h2>Subtitle</h2><p>Content</p></body></html>"#;
        let response = create_test_response(html, "https://example.com");
        let h1_elements = response.select("h1");
        assert_eq!(h1_elements.len(), 1);
    }

    #[test]
    fn test_response_select_invalid_selector() {
        let html = r#"<html><body><p>Content</p></body></html>"#;
        let response = create_test_response(html, "https://example.com");
        let elements = response.select("invalid[ selector");
        assert_eq!(elements.len(), 0);
    }
}
