//! Scrapio Browser - Stealth browser automation for Scrapio

pub mod browser;
pub mod cdp;
pub mod chromedriver;
pub mod stealth;

pub use browser::StealthBrowser;
pub use chromedriver::{ChromeDriverChannel, ChromeDriverError, ChromeDriverManager};
pub use scrapio_core::error::ScrapioError;
pub use stealth::{StealthConfig, StealthLevel};

pub use scrapio_core::{Browser, RotationStrategy, UserAgentManager, profiles};

/// Result type for browser operations
pub type Result<T> = std::result::Result<T, ScrapioError>;
