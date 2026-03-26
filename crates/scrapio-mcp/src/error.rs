//! Error types for the Scrapio MCP server.

use thiserror::Error;

/// Error types specific to the MCP server layer.
#[derive(Error, Debug)]
pub enum ScrapioMcpError {
    #[error("Scraping failed: {0}")]
    ScrapingFailed(String),

    #[error("AI extraction failed: {0}")]
    AiFailed(String),

    #[error("Browser error: {0}")]
    BrowserFailed(String),

    #[error("Storage error: {0}")]
    StorageFailed(String),

    #[error("Crawl error: {0}")]
    CrawlFailed(String),

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Resource not found: {0}")]
    ResourceNotFound(String),
}
