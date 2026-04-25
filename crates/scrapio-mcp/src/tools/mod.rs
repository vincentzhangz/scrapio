//! MCP tools for Scrapio.

#![allow(dead_code)]

use crate::error::ScrapioMcpError;
use scrapio_ai::{AiConfig, AiScraper};
use scrapio_classic::Scraper;
use scrapio_storage::Storage;
use serde::{Deserialize, Serialize};
use tracing::{debug, info, instrument};

/// Input/Output Types

#[derive(Debug, Deserialize)]
pub struct ClassicScrapeInput {
    pub url: String,
    #[serde(default)]
    pub selector: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ClassicScrapeOutput {
    pub url: String,
    pub status: u16,
    pub title: Option<String>,
    pub links: Vec<String>,
    pub html_preview: String,
}

#[derive(Debug, Deserialize)]
pub struct AiScrapeInput {
    pub url: String,
    #[serde(default)]
    pub schema: Option<String>,
    #[serde(default = "default_provider")]
    pub provider: String,
    #[serde(default)]
    pub model: Option<String>,
}

fn default_provider() -> String {
    "openai".to_string()
}

#[derive(Debug, Serialize)]
pub struct AiScrapeOutput {
    pub url: String,
    pub data: serde_json::Value,
    pub links: Vec<String>,
    pub model: String,
    pub mode: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fallback_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_error: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CrawlStartInput {
    pub url: String,
    #[serde(default)]
    pub depth: Option<usize>,
    #[serde(default)]
    pub max_pages: Option<usize>,
    #[serde(default = "default_scope")]
    pub scope: String,
    #[serde(default = "default_respect_robots")]
    pub respect_robots: bool,
    #[serde(default = "default_browser_escalation")]
    pub browser_escalation: String,
}

fn default_scope() -> String {
    "domain".to_string()
}

fn default_respect_robots() -> bool {
    true
}

fn default_browser_escalation() -> String {
    "auto".to_string()
}

#[derive(Debug, Serialize)]
pub struct CrawlStartOutput {
    pub crawl_id: String,
    pub started_at: String,
    pub status: String,
}

#[derive(Debug, Deserialize)]
pub struct CrawlStatusInput {
    pub crawl_id: String,
}

#[derive(Debug, Serialize)]
pub struct CrawlResultEntry {
    pub url: String,
    pub status: u16,
    pub title: Option<String>,
    pub depth: usize,
    pub links_found: usize,
    pub error: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CrawlStatusOutput {
    pub crawl_id: String,
    pub status: String,
    pub pages_visited: usize,
    pub results: Vec<CrawlResultEntry>,
    pub is_complete: bool,
}

#[derive(Debug, Deserialize)]
pub struct BrowserNavigateInput {
    pub url: String,
    #[serde(default = "default_headless")]
    pub headless: bool,
    #[serde(default = "default_browser")]
    pub browser: String,
    #[serde(default)]
    pub stealth: Option<String>,
}

fn default_headless() -> bool {
    true
}

fn default_browser() -> String {
    "chrome".to_string()
}

#[derive(Debug, Serialize)]
pub struct BrowserOutput {
    pub success: bool,
    pub url: Option<String>,
    pub title: Option<String>,
    pub html: Option<String>,
    pub message: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct StorageSaveInput {
    pub url: String,
    pub content: String,
    #[serde(default = "default_db_path")]
    pub database: String,
}

fn default_db_path() -> String {
    ":memory:".to_string()
}

#[derive(Debug, Serialize)]
pub struct StorageSaveOutput {
    pub id: i64,
}

#[derive(Debug, Deserialize)]
pub struct StorageGetInput {
    pub id: i64,
    #[serde(default = "default_db_path")]
    pub database: String,
}

#[derive(Debug, Serialize)]
pub struct StorageResultOutput {
    pub id: i64,
    pub url: String,
    pub status: u16,
    pub title: Option<String>,
    pub content: String,
    pub links: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct StorageListInput {
    #[serde(default = "default_limit")]
    pub limit: usize,
    #[serde(default = "default_db_path")]
    pub database: String,
}

fn default_limit() -> usize {
    10
}

/// Tool Implementations

#[instrument(skip(input), fields(url = %input.url))]
pub async fn classic_scrape_impl(
    input: ClassicScrapeInput,
) -> Result<ClassicScrapeOutput, ScrapioMcpError> {
    info!("Executing classic_scrape tool");
    let scraper = Scraper::new();

    let resp = scraper
        .scrape(&input.url)
        .await
        .map_err(|e| ScrapioMcpError::ScrapingFailed(e.to_string()))?;

    // CRITICAL: Extract all data before any await point.
    // Response contains Cell<usize> which is not Send.
    let url = resp.url.clone();
    let status = resp.status;
    let title = resp.title();
    let links = resp.links();
    let html_preview = resp.html.chars().take(500).collect();

    let output = ClassicScrapeOutput {
        url,
        status,
        title,
        links,
        html_preview,
    };

    debug!("classic_scrape completed successfully");
    Ok(output)
}

#[instrument(skip(input), fields(url = %input.url, provider = %input.provider))]
pub async fn ai_scrape_impl(input: AiScrapeInput) -> Result<AiScrapeOutput, ScrapioMcpError> {
    info!("Executing ai_scrape tool");
    let mut config = AiConfig::new().with_provider(&input.provider);

    if let Some(ref model) = input.model
        && !model.is_empty()
    {
        config = config.with_model(model);
    }

    let scraper = AiScraper::with_config(config);
    let schema = input.schema.unwrap_or_else(|| "{}".to_string());

    let result = scraper
        .scrape(&input.url, &schema)
        .await
        .map_err(|e| ScrapioMcpError::AiFailed(e.to_string()))?;

    Ok(AiScrapeOutput {
        url: result.url,
        data: result.data,
        links: result.links,
        model: result.model,
        mode: format!("{:?}", result.mode),
        fallback_reason: result.fallback_reason.map(|r| format!("{:?}", r)),
        provider_error: result.provider_error,
    })
}

#[instrument(skip(input), fields(url = %input.url))]
pub async fn crawl_start_impl(input: CrawlStartInput) -> Result<CrawlStartOutput, ScrapioMcpError> {
    info!("Executing crawl_start tool");
    use std::time::SystemTime;
    let crawl_id = format!(
        "crawl_{}",
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    let started_at = format!("{:?}", SystemTime::now());

    Ok(CrawlStartOutput {
        crawl_id,
        started_at,
        status: "started".to_string(),
    })
}

#[instrument(skip(input), fields(crawl_id = %input.crawl_id))]
pub async fn crawl_status_impl(
    input: CrawlStatusInput,
) -> Result<CrawlStatusOutput, ScrapioMcpError> {
    debug!("Executing crawl_status tool");
    Ok(CrawlStatusOutput {
        crawl_id: input.crawl_id,
        status: "unknown".to_string(),
        pages_visited: 0,
        results: vec![],
        is_complete: false,
    })
}

#[instrument(skip(input), fields(url = %input.url, browser = %input.browser))]
pub async fn browser_navigate_impl(
    input: BrowserNavigateInput,
) -> Result<BrowserOutput, ScrapioMcpError> {
    info!("Executing browser_navigate tool");
    use scrapio_browser::{BrowserType, StealthBrowser, StealthConfig, StealthLevel};

    let browser_type = BrowserType::parse(&input.browser).unwrap_or(BrowserType::Chrome);

    let driver_url = "http://localhost:9517";

    let stealth_level = match input.stealth.as_deref() {
        Some("basic") => StealthLevel::Basic,
        Some("advanced") => StealthLevel::Advanced,
        Some("full") => StealthLevel::Full,
        _ => StealthLevel::None,
    };

    let mut builder =
        StealthBrowser::with_webdriver_and_type(driver_url, browser_type).headless(input.headless);

    if stealth_level != StealthLevel::None {
        let config = StealthConfig::new(stealth_level);
        builder = builder.stealth(config);
    }

    let mut browser = builder
        .init()
        .await
        .map_err(|e| ScrapioMcpError::BrowserFailed(e.to_string()))?;

    browser
        .goto(&input.url)
        .await
        .map_err(|e| ScrapioMcpError::BrowserFailed(e.to_string()))?;

    let url = browser
        .url()
        .await
        .map_err(|e| ScrapioMcpError::BrowserFailed(e.to_string()))?;
    let title = browser
        .title()
        .await
        .map_err(|e| ScrapioMcpError::BrowserFailed(e.to_string()))?;
    let html = browser
        .html()
        .await
        .map_err(|e| ScrapioMcpError::BrowserFailed(e.to_string()))?;

    let _ = browser.close().await;

    debug!("browser_navigate completed successfully");
    Ok(BrowserOutput {
        success: true,
        url: Some(url),
        title: Some(title),
        html: Some(html),
        message: None,
    })
}

#[instrument(skip(input), fields(url = %input.url, database = %input.database))]
pub async fn storage_save_impl(
    input: StorageSaveInput,
) -> Result<StorageSaveOutput, ScrapioMcpError> {
    info!("Executing storage_save tool");
    let storage = Storage::new(&input.database)
        .await
        .map_err(|e| ScrapioMcpError::StorageFailed(e.to_string()))?;

    let (status, title, links) = {
        let scraper = Scraper::new();
        let resp = scraper
            .scrape(&input.url)
            .await
            .map_err(|e| ScrapioMcpError::ScrapingFailed(e.to_string()))?;

        // CRITICAL: Extract all data before any await point.
        // Response contains Cell<usize> which is not Send.
        (resp.status, resp.title(), resp.links())
    };

    let id = storage
        .save_result(&input.url, status, title.as_deref(), &input.content, &links)
        .await
        .map_err(|e| ScrapioMcpError::StorageFailed(e.to_string()))?;

    debug!(id, "storage_save completed");
    Ok(StorageSaveOutput { id })
}

#[instrument(skip(input), fields(id, database = %input.database))]
pub async fn storage_get_impl(
    input: StorageGetInput,
) -> Result<Option<StorageResultOutput>, ScrapioMcpError> {
    debug!("Executing storage_get tool");
    let storage = Storage::new(&input.database)
        .await
        .map_err(|e| ScrapioMcpError::StorageFailed(e.to_string()))?;

    // Use get_result_by_id since input.id is i64
    let result = storage
        .get_result_by_id(input.id)
        .await
        .map_err(|e| ScrapioMcpError::StorageFailed(e.to_string()))?;

    Ok(result.map(|r| StorageResultOutput {
        id: r.id,
        url: r.url,
        status: r.status,
        title: r.title,
        content: r.content,
        links: r.links,
    }))
}

#[instrument(skip(input), fields(limit, database = %input.database))]
pub async fn storage_list_impl(
    input: StorageListInput,
) -> Result<Vec<StorageResultOutput>, ScrapioMcpError> {
    debug!("Executing storage_list tool");
    let storage = Storage::new(&input.database)
        .await
        .map_err(|e| ScrapioMcpError::StorageFailed(e.to_string()))?;

    let results = storage
        .get_all_results(input.limit)
        .await
        .map_err(|e| ScrapioMcpError::StorageFailed(e.to_string()))?;

    Ok(results
        .into_iter()
        .map(|r| StorageResultOutput {
            id: r.id,
            url: r.url,
            status: r.status,
            title: r.title,
            content: r.content,
            links: r.links,
        })
        .collect())
}
