//! Scrapio Browser - Stealth browser automation for Scrapio
//!
//! This crate provides browser automation with anti-detection features.
//! It uses a dual-layer architecture combining WebDriver and CDP.
//!
//! ## Architecture
//!
//! - **[`browser`](browser::StealthBrowser)**: Main browser interface using WebDriver
//! - **[`cdp`](cdp)**: Chrome DevTools Protocol for stealth configuration
//! - **[`chromedriver`](chromedriver)**: ChromeDriver lifecycle management
//! - **[`stealth`](stealth)**: Stealth script generation
//!
//! See the [`cdp`](cdp) module for detailed documentation on the WebDriver vs CDP split.

pub mod browser;
pub mod cdp;
pub mod chromedriver;
pub mod stealth;

pub use browser::StealthBrowser;
pub use chromedriver::{
    ChromeDriverChannel, ChromeDriverError, ChromeDriverManager, ChromeDriverSession,
};
pub use scrapio_core::error::ScrapioError;
pub use stealth::{StealthConfig, StealthLevel};

pub use scrapio_core::{Browser, RotationStrategy, UserAgentManager, profiles};

/// Result type for browser operations
pub type Result<T> = std::result::Result<T, ScrapioError>;
