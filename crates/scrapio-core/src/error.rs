//! Error types

use thiserror::Error;

#[derive(Error, Debug)]
pub enum ScrapioError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

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
}

pub type ScrapioResult<T> = Result<T, ScrapioError>;
