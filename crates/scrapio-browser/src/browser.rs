//! Browser management and automation for Scrapio
//!
//! This module provides the main `StealthBrowser` struct for controlling
//! a browser with stealth anti-detection features. It supports multiple
//! browser types through a pluggable capabilities system.

use fantoccini::{Client, ClientBuilder};
use scrapio_core::error::ScrapioError;
use scrapio_core::proxy::ProxyConfig;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::PathBuf;
use std::time::Duration;
use tokio::sync::OnceCell;
use tracing::{debug, info, instrument, warn};

pub use crate::stealth::{StealthConfig, StealthLevel};

/// Represents a captured network request (XHR or fetch)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkRequest {
    /// Request type: "fetch" or "xhr"
    pub request_type: String,
    /// Request URL
    pub url: String,
    /// HTTP method
    pub method: String,
    /// Timestamp when request was made
    pub timestamp: u64,
}

/// Browser type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BrowserType {
    /// Google Chrome (default)
    #[default]
    Chrome,
    /// Mozilla Firefox
    Firefox,
    /// Microsoft Edge
    Edge,
}

impl BrowserType {
    /// Get the default WebDriver port for this browser type
    pub fn default_port(&self) -> u16 {
        match self {
            BrowserType::Chrome => 9515,
            BrowserType::Firefox => 4444,
            BrowserType::Edge => 9516,
        }
    }

    /// Get the default WebDriver URL for this browser type
    pub fn default_webdriver_url(&self) -> String {
        format!("http://localhost:{}", self.default_port())
    }

    /// Parse from string (case-insensitive)
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "chrome" | "chromium" => Some(BrowserType::Chrome),
            "firefox" | "ff" | "gecko" => Some(BrowserType::Firefox),
            "edge" | "msedge" => Some(BrowserType::Edge),
            _ => None,
        }
    }
}

impl std::fmt::Display for BrowserType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BrowserType::Chrome => write!(f, "chrome"),
            BrowserType::Firefox => write!(f, "firefox"),
            BrowserType::Edge => write!(f, "edge"),
        }
    }
}

/// Browser capabilities trait - implement this to add support for new browsers
pub trait BrowserCapabilities: Send + Sync + std::fmt::Debug {
    /// Get the browser name for WebDriver capabilities
    fn browser_name(&self) -> &str;

    /// Get the WebDriver options key (e.g., "goog:chromeOptions", "moz:firefoxOptions")
    fn options_key(&self) -> &str;

    /// Build browser-specific arguments
    fn build_args(&self, config: &BrowserConfig) -> Vec<Value>;

    /// Build browser options (binary path, etc.)
    fn build_options(&self, config: &BrowserConfig) -> serde_json::Map<String, Value>;

    /// Whether this browser supports stealth patching at the binary level
    fn supports_stealth_patching(&self) -> bool {
        false
    }

    /// Get the default WebDriver port
    fn default_port(&self) -> u16 {
        9515
    }
}

/// Chrome browser capabilities
#[derive(Debug, Default)]
pub struct ChromeCapabilities;

impl BrowserCapabilities for ChromeCapabilities {
    fn browser_name(&self) -> &str {
        "chrome"
    }

    fn options_key(&self) -> &str {
        "goog:chromeOptions"
    }

    fn build_args(&self, config: &BrowserConfig) -> Vec<Value> {
        let mut args: Vec<Value> = vec![
            Value::String("--no-sandbox".to_string()),
            Value::String("--disable-dev-shm-usage".to_string()),
            Value::String("--disable-blink-features=AutomationControlled".to_string()),
        ];

        if config.headless {
            args.push(Value::String("--headless=new".to_string()));
            args.push(Value::String("--disable-gpu".to_string()));
        }

        // Add proxy if configured
        if let Some(ref proxy) = config.proxy {
            let proxy_url = proxy.with_auth();
            args.push(Value::String(format!("--proxy-server={}", proxy_url)));
            // Bypass proxy for localhost if needed
            args.push(Value::String("--proxy-bypass-list=<local>".to_string()));
        }

        // Add window size if specified
        if let Some((width, height)) = config.window_size {
            args.push(Value::String(format!("--window-size={},{}", width, height)));
        }

        // Add custom arguments
        for arg in &config.args {
            args.push(Value::String(arg.clone()));
        }

        // Add user agent if stealth is configured
        if let Some(ref stealth) = config.stealth
            && let Some(ua) = stealth.get_user_agent()
        {
            args.push(Value::String(format!("--user-agent={}", ua)));
        }

        args
    }

    fn build_options(&self, config: &BrowserConfig) -> serde_json::Map<String, Value> {
        let mut options = serde_json::Map::new();
        options.insert("args".to_string(), Value::Array(self.build_args(config)));

        if let Some(ref path) = config.browser_path {
            options.insert(
                "binary".to_string(),
                Value::String(path.display().to_string()),
            );
        }

        options
    }

    fn supports_stealth_patching(&self) -> bool {
        true
    }

    fn default_port(&self) -> u16 {
        9515
    }
}

/// Firefox browser capabilities
#[derive(Debug, Default)]
pub struct FirefoxCapabilities;

impl BrowserCapabilities for FirefoxCapabilities {
    fn browser_name(&self) -> &str {
        "firefox"
    }

    fn options_key(&self) -> &str {
        "moz:firefoxOptions"
    }

    fn build_args(&self, config: &BrowserConfig) -> Vec<Value> {
        let mut args: Vec<Value> = Vec::new();

        // Firefox uses different arguments
        if config.headless {
            args.push(Value::String("--headless".to_string()));
        }

        // Add proxy if configured
        if let Some(ref proxy) = config.proxy
            && let Some((host, port)) = proxy.host_port()
        {
            args.push(Value::String(format!("--proxy={}:{}", host, port)));
        }

        // Add window size if specified
        if let Some((width, height)) = config.window_size {
            args.push(Value::String(format!("--width={}", width)));
            args.push(Value::String(format!("--height={}", height)));
        }

        // Add custom arguments
        for arg in &config.args {
            // Convert Chrome-style args to Firefox if needed
            if arg.starts_with("--user-agent=") {
                // Skip, will be set via prefs
                continue;
            }
            args.push(Value::String(arg.clone()));
        }

        args
    }

    fn build_options(&self, config: &BrowserConfig) -> serde_json::Map<String, Value> {
        let mut options = serde_json::Map::new();

        // Build args
        options.insert("args".to_string(), Value::Array(self.build_args(config)));

        // Firefox binary path
        if let Some(ref path) = config.browser_path {
            options.insert(
                "binary".to_string(),
                Value::String(path.display().to_string()),
            );
        }

        // Build prefs for stealth settings and proxy
        let mut prefs = serde_json::Map::new();

        // Set custom user agent via prefs if stealth is configured
        if let Some(ref stealth) = config.stealth
            && let Some(ua) = stealth.get_user_agent()
        {
            prefs.insert("general.useragent.override".to_string(), Value::String(ua));
        }

        // Set Firefox-specific stealth prefs
        prefs.insert(
            "privacy.resistFingerprinting".to_string(),
            Value::Bool(true),
        );
        prefs.insert("webgl.disabled".to_string(), Value::Bool(true));

        // Proxy preferences
        if let Some(ref proxy) = config.proxy {
            prefs.insert("network.proxy.type".to_string(), Value::Number(serde_json::Number::from(1))); // Manual proxy
            if let Some((host, port)) = proxy.host_port() {
                prefs.insert("network.proxy.http".to_string(), Value::String(host.clone()));
                prefs.insert("network.proxy.http_port".to_string(), Value::Number(serde_json::Number::from(port)));
                prefs.insert("network.proxy.ssl".to_string(), Value::String(host));
                prefs.insert("network.proxy.ssl_port".to_string(), Value::Number(serde_json::Number::from(port)));
                // Share HTTP proxy for SSL
                prefs.insert("network.proxy.share_proxies".to_string(), Value::Bool(true));
            }
            // Note: Firefox requires about:config changes for auth
            // Users may need to set network.proxy.socks_remote_dns for DNS over proxy
        }

        if !prefs.is_empty() {
            options.insert("prefs".to_string(), Value::Object(prefs));
        }

        options
    }

    fn supports_stealth_patching(&self) -> bool {
        false // Firefox doesn't need binary patching
    }

    fn default_port(&self) -> u16 {
        4444
    }
}

/// Edge browser capabilities
#[derive(Debug, Default)]
pub struct EdgeCapabilities;

impl BrowserCapabilities for EdgeCapabilities {
    fn browser_name(&self) -> &str {
        "msedge"
    }

    fn options_key(&self) -> &str {
        "ms:edgeOptions"
    }

    fn build_args(&self, config: &BrowserConfig) -> Vec<Value> {
        let mut args: Vec<Value> = vec![
            Value::String("--no-sandbox".to_string()),
            Value::String("--disable-dev-shm-usage".to_string()),
            Value::String("--disable-blink-features=AutomationControlled".to_string()),
        ];

        if config.headless {
            args.push(Value::String("--headless=new".to_string()));
            args.push(Value::String("--disable-gpu".to_string()));
        }

        // Add proxy if configured (Edge is Chromium-based, same as Chrome)
        if let Some(ref proxy) = config.proxy {
            let proxy_url = proxy.with_auth();
            args.push(Value::String(format!("--proxy-server={}", proxy_url)));
            args.push(Value::String("--proxy-bypass-list=<local>".to_string()));
        }

        // Add window size if specified
        if let Some((width, height)) = config.window_size {
            args.push(Value::String(format!("--window-size={},{}", width, height)));
        }

        // Add custom arguments
        for arg in &config.args {
            args.push(Value::String(arg.clone()));
        }

        // Add user agent if stealth is configured
        if let Some(ref stealth) = config.stealth
            && let Some(ua) = stealth.get_user_agent()
        {
            args.push(Value::String(format!("--user-agent={}", ua)));
        }

        args
    }

    fn build_options(&self, config: &BrowserConfig) -> serde_json::Map<String, Value> {
        let mut options = serde_json::Map::new();
        options.insert("args".to_string(), Value::Array(self.build_args(config)));

        if let Some(ref path) = config.browser_path {
            options.insert(
                "binary".to_string(),
                Value::String(path.display().to_string()),
            );
        }

        options
    }

    fn supports_stealth_patching(&self) -> bool {
        false // Edge is Chromium-based but ChromeDriver patches should work
    }

    fn default_port(&self) -> u16 {
        9516
    }
}

/// Get capabilities for a browser type
pub fn get_capabilities(browser_type: BrowserType) -> Box<dyn BrowserCapabilities> {
    match browser_type {
        BrowserType::Chrome => Box::new(ChromeCapabilities),
        BrowserType::Firefox => Box::new(FirefoxCapabilities),
        BrowserType::Edge => Box::new(EdgeCapabilities),
    }
}

/// Configuration for the browser
#[derive(Debug, Clone)]
pub struct BrowserConfig {
    /// Whether to run browser in headless mode
    pub headless: bool,
    /// Stealth configuration
    pub stealth: Option<StealthConfig>,
    /// Path to browser binary
    pub browser_path: Option<PathBuf>,
    /// Path to WebDriver
    pub driver_path: Option<PathBuf>,
    /// Additional browser arguments
    pub args: Vec<String>,
    /// Page load timeout
    pub timeout: Duration,
    /// Window size (width, height)
    pub window_size: Option<(u32, u32)>,
    /// Proxy configuration
    pub proxy: Option<ProxyConfig>,
}

impl Default for BrowserConfig {
    fn default() -> Self {
        Self {
            headless: true,
            stealth: Some(StealthConfig::default()),
            browser_path: None,
            driver_path: None,
            args: Vec::new(),
            timeout: Duration::from_secs(30),
            window_size: Some((1920, 1080)),
            proxy: None,
        }
    }
}

/// A stealth browser instance for automated web browsing
///
/// # Example
///
/// ```ignore
/// use scrapio_browser::{StealthBrowser, StealthConfig, StealthLevel, BrowserType};
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let mut browser = StealthBrowser::new()
///         .headless(true)
///         .browser_type(BrowserType::Firefox)
///         .stealth(StealthConfig::new(StealthLevel::Basic))?;
///
///     browser.goto("https://www.rust-lang.org").await?;
///
///     let title = browser.title().await?;
///     println!("Page title: {}", title);
///
///     browser.close().await?;
///     Ok(())
/// }
/// ```
pub struct StealthBrowser {
    client: OnceCell<Client>,
    config: BrowserConfig,
    webdriver_url: String,
    browser_type: BrowserType,
    capabilities: Box<dyn BrowserCapabilities>,
}

impl StealthBrowser {
    /// Create a new StealthBrowser with default configuration (Chrome)
    pub fn new() -> Self {
        Self::with_browser_type(BrowserType::Chrome)
    }

    /// Create a new StealthBrowser with explicit browser type
    pub fn with_browser_type(browser_type: BrowserType) -> Self {
        Self {
            client: OnceCell::new(),
            config: BrowserConfig::default(),
            webdriver_url: browser_type.default_webdriver_url(),
            browser_type,
            capabilities: get_capabilities(browser_type),
        }
    }

    /// Create a new StealthBrowser with explicit WebDriver URL
    pub fn with_webdriver(url: impl Into<String>) -> Self {
        Self {
            client: OnceCell::new(),
            config: BrowserConfig::default(),
            webdriver_url: url.into(),
            browser_type: BrowserType::Chrome,
            capabilities: Box::new(ChromeCapabilities),
        }
    }

    /// Create a new StealthBrowser with explicit WebDriver URL and browser type
    pub fn with_webdriver_and_type(url: impl Into<String>, browser_type: BrowserType) -> Self {
        Self {
            client: OnceCell::new(),
            config: BrowserConfig::default(),
            webdriver_url: url.into(),
            browser_type,
            capabilities: get_capabilities(browser_type),
        }
    }

    /// Get the browser type
    pub fn get_browser_type(&self) -> BrowserType {
        self.browser_type
    }

    /// Set the browser type (requires WebDriver to be running for that browser)
    pub fn set_browser_type(mut self, browser_type: BrowserType) -> Self {
        self.browser_type = browser_type;
        self.capabilities = get_capabilities(browser_type);
        // Update WebDriver URL if using default
        if self.webdriver_url == BrowserType::Chrome.default_webdriver_url()
            || self.webdriver_url == BrowserType::default().default_webdriver_url()
        {
            self.webdriver_url = browser_type.default_webdriver_url();
        }
        self
    }

    /// Set the browser to run in headless mode
    pub fn headless(mut self, headless: bool) -> Self {
        self.config.headless = headless;
        self
    }

    /// Set stealth configuration
    pub fn stealth(mut self, config: StealthConfig) -> Self {
        self.config.stealth = Some(config);
        self
    }

    /// Set the stealth level (convenience method)
    pub fn stealth_level(mut self, level: StealthLevel) -> Self {
        self.config.stealth = Some(StealthConfig::new(level));
        self
    }

    /// Set path to browser binary
    pub fn browser_path(mut self, path: PathBuf) -> Self {
        self.config.browser_path = Some(path);
        self
    }

    /// Set path to browser binary (alias for browser_path for backward compatibility)
    #[deprecated(since = "0.2.0", note = "Use browser_path() instead")]
    pub fn chrome_path(self, path: PathBuf) -> Self {
        self.browser_path(path)
    }

    /// Set path to WebDriver
    pub fn driver_path(mut self, path: PathBuf) -> Self {
        self.config.driver_path = Some(path);
        self
    }

    /// Add additional browser arguments
    pub fn arg(mut self, arg: impl Into<String>) -> Self {
        self.config.args.push(arg.into());
        self
    }

    /// Set proxy configuration
    pub fn proxy(mut self, proxy: ProxyConfig) -> Self {
        self.config.proxy = Some(proxy);
        self
    }

    /// Set page load timeout
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.config.timeout = timeout;
        self
    }

    /// Set window size
    pub fn window_size(mut self, width: u32, height: u32) -> Self {
        self.config.window_size = Some((width, height));
        self
    }

    /// Get the underlying WebDriver client (initializes if needed)
    #[instrument(skip(self))]
    async fn get_client(&self) -> Result<&Client, ScrapioError> {
        if let Some(client) = self.client.get() {
            return Ok(client);
        }

        debug!("Initializing WebDriver client");

        // Build browser capabilities
        let mut builder = ClientBuilder::native();
        builder.capabilities(self.build_capabilities());

        let client = builder
            .connect(&self.webdriver_url)
            .await
            .map_err(|e| ScrapioError::Browser(format!("Failed to connect to WebDriver: {}", e)))?;

        // Apply stealth scripts if configured
        if let Some(ref stealth) = self.config.stealth {
            let script = stealth.generate_script();
            if !script.is_empty() {
                debug!("Applying stealth script");
                if let Err(e) = client.execute(&script, vec![]).await {
                    warn!("Failed to apply stealth script: {}", e);
                }
            }
        }

        self.client
            .set(client)
            .map_err(|_| ScrapioError::Browser("Client already initialized".to_string()))?;

        info!("WebDriver client initialized successfully");
        Ok(self.client.get().unwrap())
    }

    fn build_capabilities(&self) -> fantoccini::wd::Capabilities {
        let mut caps = fantoccini::wd::Capabilities::new();
        let options = self.capabilities.build_options(&self.config);

        // Set browser name
        caps.insert(
            "browserName".to_string(),
            Value::String(self.capabilities.browser_name().to_string()),
        );

        // Set browser-specific options
        caps.insert(
            self.capabilities.options_key().to_string(),
            Value::Object(options),
        );

        // Set timeouts
        caps.insert(
            "timeouts".to_string(),
            serde_json::json!({
                "pageLoad": self.config.timeout.as_millis() as u64
            }),
        );

        caps
    }

    /// Navigate to a URL
    ///
    /// # Errors
    /// Returns an error if navigation fails or WebDriver is not available
    #[instrument(skip(self), fields(url = %url))]
    pub async fn goto(&mut self, url: &str) -> Result<(), ScrapioError> {
        info!("Navigating to URL");
        let client = self.get_client().await?;

        client
            .goto(url)
            .await
            .map_err(|e| ScrapioError::Browser(format!("Failed to navigate: {}", e)))?;

        // Re-apply stealth scripts after navigation (for page loads that reset them)
        if let Some(ref stealth) = self.config.stealth {
            let script = stealth.generate_script();
            if !script.is_empty()
                && let Err(e) = client.execute(&script, vec![]).await
            {
                debug!("Stealth script re-application warning: {}", e);
            }
        }

        Ok(())
    }

    /// Get the current page title
    ///
    /// # Errors
    /// Returns an error if the operation fails
    #[instrument(skip(self))]
    pub async fn title(&self) -> Result<String, ScrapioError> {
        let client = self.get_client().await?;

        client
            .title()
            .await
            .map_err(|e| ScrapioError::Browser(format!("Failed to get title: {}", e)))
    }

    /// Get the current page URL
    ///
    /// # Errors
    /// Returns an error if the operation fails
    #[instrument(skip(self))]
    pub async fn url(&self) -> Result<String, ScrapioError> {
        let client = self.get_client().await?;

        let url = client
            .current_url()
            .await
            .map_err(|e| ScrapioError::Browser(format!("Failed to get URL: {}", e)))?;

        Ok(url.to_string())
    }

    /// Find a single element using CSS selector
    ///
    /// # Errors
    /// Returns an error if the element is not found or operation fails
    #[instrument(skip(self), fields(selector))]
    pub async fn find_element(
        &self,
        selector: &str,
    ) -> Result<fantoccini::elements::Element, ScrapioError> {
        let client = self.get_client().await?;

        client
            .find(fantoccini::Locator::Css(selector))
            .await
            .map_err(|e| ScrapioError::Browser(format!("Element not found: {}", e)))
    }

    /// Find multiple elements using CSS selector
    ///
    /// # Errors
    /// Returns an error if operation fails
    #[instrument(skip(self), fields(selector))]
    pub async fn find_elements(
        &self,
        selector: &str,
    ) -> Result<Vec<fantoccini::elements::Element>, ScrapioError> {
        let client = self.get_client().await?;

        client
            .find_all(fantoccini::Locator::Css(selector))
            .await
            .map_err(|e| ScrapioError::Browser(format!("Failed to find elements: {}", e)))
    }

    /// Execute JavaScript in the browser context
    ///
    /// # Errors
    /// Returns an error if script execution fails
    #[instrument(skip(self, script), fields(script_len = script.len()))]
    pub async fn execute_script(&self, script: &str) -> Result<serde_json::Value, ScrapioError> {
        let client = self.get_client().await?;

        client
            .execute(script, vec![])
            .await
            .map_err(|e| ScrapioError::Browser(format!("Script execution failed: {}", e)))
    }

    /// Execute async JavaScript in the browser context
    ///
    /// # Errors
    /// Returns an error if script execution fails
    pub async fn execute_script_async(
        &self,
        script: &str,
    ) -> Result<serde_json::Value, ScrapioError> {
        let client = self.get_client().await?;

        client
            .execute_async(script, vec![])
            .await
            .map_err(|e| ScrapioError::Browser(format!("Async script execution failed: {}", e)))
    }

    /// Take a screenshot of the current page
    ///
    /// # Errors
    /// Returns an error if screenshot fails
    #[instrument(skip(self))]
    pub async fn screenshot(&self) -> Result<Vec<u8>, ScrapioError> {
        let client = self.get_client().await?;

        client
            .screenshot()
            .await
            .map_err(|e| ScrapioError::Browser(format!("Screenshot failed: {}", e)))
    }

    /// Take a screenshot and save to a file
    ///
    /// # Errors
    /// Returns an error if screenshot fails or file cannot be written
    #[instrument(skip(self), fields(path))]
    pub async fn screenshot_to_file(&self, path: &str) -> Result<(), ScrapioError> {
        let screenshot = self.screenshot().await?;

        std::fs::write(path, &screenshot).map_err(ScrapioError::Io)?;

        info!("Screenshot saved");
        Ok(())
    }

    /// Get page HTML content
    ///
    /// # Errors
    /// Returns an error if operation fails
    #[instrument(skip(self))]
    pub async fn html(&self) -> Result<String, ScrapioError> {
        let client = self.get_client().await?;

        client
            .source()
            .await
            .map_err(|e| ScrapioError::Browser(format!("Failed to get page source: {}", e)))
    }

    /// Refresh the current page
    ///
    /// # Errors
    /// Returns an error if operation fails
    pub async fn refresh(&mut self) -> Result<(), ScrapioError> {
        let client = self.get_client().await?;

        client
            .refresh()
            .await
            .map_err(|e| ScrapioError::Browser(format!("Failed to refresh: {}", e)))
    }

    /// Go back in browser history
    ///
    /// # Errors
    /// Returns an error if operation fails
    pub async fn back(&mut self) -> Result<(), ScrapioError> {
        let client = self.get_client().await?;

        client
            .back()
            .await
            .map_err(|e| ScrapioError::Browser(format!("Failed to go back: {}", e)))
    }

    /// Go forward in browser history
    ///
    /// # Errors
    /// Returns an error if operation fails
    pub async fn forward(&mut self) -> Result<(), ScrapioError> {
        let client = self.get_client().await?;

        client
            .forward()
            .await
            .map_err(|e| ScrapioError::Browser(format!("Failed to go forward: {}", e)))
    }

    /// Close the browser
    ///
    /// # Errors
    /// Returns an error if closing fails
    #[instrument(skip(self))]
    pub async fn close(&mut self) -> Result<(), ScrapioError> {
        if self.client.get().is_some() {
            info!("Closing browser");
            let client = std::mem::take(&mut self.client);
            // Take the client from OnceCell and close it
            if let Some(c) = client.into_inner() {
                c.close().await.map_err(|e| {
                    ScrapioError::Browser(format!("Failed to close browser: {}", e))
                })?;
            }
        }
        Ok(())
    }

    /// Enable network request capture
    ///
    /// This injects JavaScript to intercept XHR and fetch requests,
    /// storing them for later retrieval.
    ///
    /// # Errors
    /// Returns an error if script injection fails
    #[instrument(skip(self))]
    pub async fn enable_network_capture(&self) -> Result<(), ScrapioError> {
        let script = r#"
            (function() {
                if (window.__networkCaptureEnabled) return;
                window.__networkCaptureEnabled = true;
                window.__capturedNetworkRequests = [];

                // Intercept fetch
                const originalFetch = window.fetch;
                window.fetch = function(...args) {
                    const request = {
                        type: 'fetch',
                        url: args[0] ? args[0].toString() : '',
                        method: args[1] ? (args[1].method || 'GET') : 'GET',
                        timestamp: Date.now()
                    };
                    window.__capturedNetworkRequests.push(request);

                    return originalFetch.apply(this, args).catch(err => {
                        console.error('Fetch error:', err);
                        throw err;
                    });
                };

                // Intercept XMLHttpRequest
                const originalOpen = XMLHttpRequest.prototype.open;
                const originalSend = XMLHttpRequest.prototype.send;

                XMLHttpRequest.prototype.open = function(method, url, ...rest) {
                    this.__method = method;
                    this.__url = url;
                    return originalOpen.apply(this, [method, url, ...rest]);
                };

                XMLHttpRequest.prototype.send = function(...args) {
                    const request = {
                        type: 'xhr',
                        url: this.__url || '',
                        method: this.__method || 'GET',
                        timestamp: Date.now()
                    };
                    window.__capturedNetworkRequests.push(request);

                    this.addEventListener('load', function() {
                        // Request completed
                    });

                    return originalSend.apply(this, args);
                };

                console.log('Network capture enabled');
            })();
        "#;

        self.execute_script(script).await?;
        Ok(())
    }

    /// Get captured network requests
    ///
    /// Returns all XHR and fetch requests captured since
    /// enable_network_capture was called.
    ///
    /// # Errors
    /// Returns an error if script execution fails
    #[instrument(skip(self))]
    pub async fn get_network_requests(&self) -> Result<Vec<NetworkRequest>, ScrapioError> {
        let script = r#"
            (function() {
                return JSON.stringify(window.__capturedNetworkRequests || []);
            })();
        "#;

        let result = self.execute_script(script).await?;
        let json_str = result.as_str().unwrap_or("[]");

        #[derive(Deserialize)]
        struct CapturedRequest {
            #[serde(rename = "type")]
            request_type: String,
            url: String,
            method: String,
            timestamp: u64,
        }

        let requests: Vec<CapturedRequest> = serde_json::from_str(json_str).unwrap_or_default();

        debug!("Retrieved {} network requests", requests.len());
        Ok(requests
            .into_iter()
            .map(|r| NetworkRequest {
                request_type: r.request_type,
                url: r.url,
                method: r.method,
                timestamp: r.timestamp,
            })
            .collect())
    }

    /// Clear captured network requests
    ///
    /// # Errors
    /// Returns an error if script execution fails
    pub async fn clear_network_requests(&self) -> Result<(), ScrapioError> {
        let script = r#"
            (function() {
                window.__capturedNetworkRequests = [];
            })();
        "#;

        self.execute_script(script).await?;
        Ok(())
    }

    /// Wait for an element to appear
    ///
    /// # Arguments
    /// * `selector` - CSS selector for the element
    /// * `timeout` - Maximum time to wait
    ///
    /// # Errors
    /// Returns an error if element is not found within timeout
    pub async fn wait_for_element(
        &self,
        selector: &str,
        timeout: Duration,
    ) -> Result<fantoccini::elements::Element, ScrapioError> {
        let start = std::time::Instant::now();

        while start.elapsed() < timeout {
            match self.find_element(selector).await {
                Ok(element) => return Ok(element),
                Err(_) => {
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
            }
        }

        Err(ScrapioError::Browser(format!(
            "Element '{}' not found within timeout",
            selector
        )))
    }

    /// Check if an element exists
    ///
    /// # Arguments
    /// * `selector` - CSS selector for the element
    ///
    /// # Errors
    /// Returns an error if operation fails
    pub async fn element_exists(&self, selector: &str) -> Result<bool, ScrapioError> {
        match self.find_element(selector).await {
            Ok(_) => Ok(true),
            Err(e) if e.to_string().contains("not found") => Ok(false),
            Err(e) => Err(e),
        }
    }

    /// Click an element by CSS selector
    ///
    /// # Arguments
    /// * `selector` - CSS selector for the element to click
    ///
    /// # Errors
    /// Returns an error if element not found or click fails
    pub async fn click(&self, selector: &str) -> Result<(), ScrapioError> {
        let element = self.find_element(selector).await?;
        element
            .click()
            .await
            .map_err(|e| ScrapioError::Browser(format!("Failed to click element: {}", e)))
    }

    /// Hover over an element by CSS selector
    ///
    /// # Arguments
    /// * `selector` - CSS selector for the element to hover
    ///
    /// # Errors
    /// Returns an error if element not found or hover fails
    pub async fn hover(&self, selector: &str) -> Result<(), ScrapioError> {
        let _element = self.find_element(selector).await?;
        self.execute_script(&format!(
            "var el = document.querySelector('{}'); if(el) {{ var event = new MouseEvent('mouseover', {{bubbles: true, cancelable: true, view: window}}); el.dispatchEvent(event); }}",
            selector.replace('\'', "\\'")
        ))
        .await?;
        Ok(())
    }

    /// Scroll the page
    ///
    /// # Arguments
    /// * `pixels` - Number of pixels to scroll (positive = down, negative = up)
    ///
    /// # Errors
    /// Returns an error if scroll fails
    pub async fn scroll(&self, pixels: i32) -> Result<(), ScrapioError> {
        self.execute_script(&format!("window.scrollBy(0, {});", pixels))
            .await?;
        Ok(())
    }

    /// Scroll to a specific element
    ///
    /// # Arguments
    /// * `selector` - CSS selector for the element to scroll into view
    ///
    /// # Errors
    /// Returns an error if element not found or scroll fails
    pub async fn scroll_to_element(&self, selector: &str) -> Result<(), ScrapioError> {
        self.execute_script(&format!(
            "var el = document.querySelector('{}'); if(el) {{ el.scrollIntoView({{behavior: 'smooth', block: 'center'}}); }}",
            selector.replace('\'', "\\'")
        ))
        .await?;
        Ok(())
    }

    /// Scroll to top of page
    ///
    /// # Errors
    /// Returns an error if scroll fails
    pub async fn scroll_to_top(&self) -> Result<(), ScrapioError> {
        self.execute_script("window.scrollTo(0, 0);").await?;
        Ok(())
    }

    /// Scroll to bottom of page
    ///
    /// # Errors
    /// Returns an error if scroll fails
    pub async fn scroll_to_bottom(&self) -> Result<(), ScrapioError> {
        self.execute_script("window.scrollTo(0, document.body.scrollHeight);")
            .await?;
        Ok(())
    }
}

impl Default for StealthBrowser {
    fn default() -> Self {
        Self::new()
    }
}

/// Extension trait to make StealthBrowser usable more easily
impl StealthBrowser {
    /// Create a new browser and initialize it in one async call
    pub async fn init(self) -> Result<Self, ScrapioError> {
        // Just trigger client initialization
        self.get_client().await?;
        Ok(self)
    }
}
