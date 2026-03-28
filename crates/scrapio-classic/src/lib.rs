//! Classic scraping for Scrapio

use scraper::{Html, Selector};
use scrapio_core::{ScrapioResult, error::*, http::HttpClient};
use tracing::{debug, info, instrument};

pub mod crawler;
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

    #[instrument(skip(self), fields(url = %url))]
    pub async fn scrape(&self, url: &str) -> ScrapioResult<Response> {
        info!("Starting classic scrape");

        if !scrapio_core::utils::url::is_valid(url) {
            return Err(ScrapioError::Parse(format!("Invalid URL: {}", url)));
        }

        debug!("Sending HTTP request");
        let response = self.http.client().get(url).send().await?;
        let status = response.status().as_u16();
        debug!(status, "Received HTTP response");
        let html = response.text().await?;
        let document = Html::parse_document(&html);

        info!("Classic scrape completed");
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
