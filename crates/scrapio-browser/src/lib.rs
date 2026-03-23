//! Scrapio Browser - Stealth browser automation for Scrapio
//!
//! This crate provides browser automation with anti-detection features.
//! It supports multiple browsers (Chrome, Firefox, Edge) through a pluggable
//! capabilities system.
//!
//! ## Architecture
//!
//! - **[`browser`](browser::StealthBrowser)**: Main browser interface using WebDriver
//! - **[`driver`](driver)**: Generic WebDriver management for all browsers
//! - **[`stealth`](stealth)**: Stealth script generation
//!
//! ## Supported Browsers
//!
//! - Chrome (default, port 9515)
//! - Firefox (port 4444)
//! - Edge (port 9516)
//!
//! ## Usage
//!
//! ```ignore
//! use scrapio_browser::{StealthBrowser, BrowserType, StealthConfig, StealthLevel};
//!
//! // Use Chrome (default)
//! let browser = StealthBrowser::new()
//!     .headless(true)
//!     .stealth_level(StealthLevel::Basic);
//!
//! // Use Firefox
//! let browser = StealthBrowser::with_browser_type(BrowserType::Firefox)
//!     .headless(true);
//! ```

pub mod browser;
pub mod cdp;
pub mod driver;
pub mod stealth;

// Re-export browser types
pub use browser::{
    BrowserCapabilities, BrowserType, ChromeCapabilities, EdgeCapabilities, FirefoxCapabilities,
    NetworkRequest, StealthBrowser, get_capabilities,
};

// Re-export driver types
pub use driver::{
    Arch, DriverChannel, DriverError, DriverManager, DriverType, Os, WebDriverSession,
};

// Backward compatibility - keep old names working
pub use driver::chromedriver::{
    ChromeDriverChannel, ChromeDriverError, ChromeDriverManager, ChromeDriverSession,
};

pub use scrapio_core::error::ScrapioError;
pub use stealth::{StealthConfig, StealthLevel};

pub use scrapio_core::{Browser, RotationStrategy, UserAgentManager, profiles};

/// Result type for browser operations
pub type Result<T> = std::result::Result<T, ScrapioError>;
