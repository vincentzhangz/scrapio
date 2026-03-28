//! Error types for Scrapio
//!
//! Provides a unified error type (`ScrapioError`) used across all crates,
//! and a convenient `ScrapioResult` type alias.

use thiserror::Error;

/// Main error type for Scrapio operations
#[derive(Error, Debug)]
pub enum ScrapioError {
    #[error("HTTP error: {0}")]
    Http(String),

    #[error("Parse error: {0}")]
    Parse(String),

    #[error("Storage error: {0}")]
    Storage(String),

    #[error("AI error: {0}")]
    Ai(String),

    #[error("Runtime error: {0}")]
    Runtime(String),

    #[error("Config error: {0}")]
    Config(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Browser error: {0}")]
    Browser(String),
}

impl From<reqwest::Error> for ScrapioError {
    fn from(err: reqwest::Error) -> Self {
        ScrapioError::Http(err.to_string())
    }
}

/// Result type alias using ScrapioError as the error type
pub type ScrapioResult<T> = Result<T, ScrapioError>;
