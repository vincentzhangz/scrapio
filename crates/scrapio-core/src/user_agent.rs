//! User agent management for browser automation

use serde::{Deserialize, Serialize};

/// Common browser user agents
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Browser {
    #[default]
    Chrome,
    Firefox,
    Safari,
    Edge,
}

impl Browser {
    /// Get user agent string for this browser
    pub fn user_agent(self, version: Option<&str>) -> String {
        let ver = version.unwrap_or("122.0.0.0");
        match self {
            Browser::Chrome => format!(
                "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/{} Safari/537.36",
                ver
            ),
            Browser::Firefox => format!(
                "Mozilla/5.0 (Macintosh; Intel Mac OS X 10.15; rv:109.0) Gecko/20100101 Firefox/{}",
                ver
            ),
            Browser::Safari => format!(
                "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/{} Safari/605.1.15",
                ver
            ),
            Browser::Edge => format!(
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/{} Safari/537.36 Edg/{}",
                ver, ver
            ),
        }
    }
}

/// User agent rotation strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RotationStrategy {
    /// Use the same user agent always (recommended for stealth)
    Fixed,
    /// Rotate user agent on each new session
    PerSession,
    /// Rotate user agent periodically based on time
    Timed,
}

/// User agent manager for handling user agent configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserAgentManager {
    /// Primary browser to use
    pub browser: Browser,
    /// Specific version to use (None = latest stable)
    pub version: Option<String>,
    /// Custom user agent string (overrides browser + version)
    pub custom: Option<String>,
    /// Rotation strategy
    pub rotation: RotationStrategy,
    /// Time interval for timed rotation (in seconds)
    pub rotation_interval: Option<u64>,
    /// Whether to match the automation to the user agent
    pub match_automation: bool,
}

impl Default for UserAgentManager {
    fn default() -> Self {
        Self {
            browser: Browser::Chrome,
            version: None,
            custom: None,
            rotation: RotationStrategy::Fixed,
            rotation_interval: None,
            match_automation: true, // Hide automation flags when UA is set
        }
    }
}

impl UserAgentManager {
    /// Create a new user agent manager with defaults
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the browser type
    pub fn with_browser(mut self, browser: Browser) -> Self {
        self.browser = browser;
        self
    }

    /// Set a specific browser version
    pub fn with_version(mut self, version: impl Into<String>) -> Self {
        self.version = Some(version.into());
        self
    }

    /// Set a completely custom user agent string
    pub fn with_custom(mut self, ua: impl Into<String>) -> Self {
        self.custom = Some(ua.into());
        self
    }

    /// Enable user agent rotation per session
    pub fn with_rotation_per_session(mut self) -> Self {
        self.rotation = RotationStrategy::PerSession;
        self
    }

    /// Enable timed user agent rotation
    pub fn with_timed_rotation(mut self, interval_secs: u64) -> Self {
        self.rotation = RotationStrategy::Timed;
        self.rotation_interval = Some(interval_secs);
        self
    }

    /// Disable automation matching (show webdriver)
    pub fn show_automation(mut self) -> Self {
        self.match_automation = false;
        self
    }

    /// Get the current user agent string
    pub fn get_user_agent(&self) -> String {
        if let Some(ref custom) = self.custom {
            return custom.clone();
        }
        self.browser.user_agent(self.version.as_deref())
    }

    /// Check if automation should be hidden based on configuration
    pub fn should_hide_automation(&self) -> bool {
        self.match_automation
    }
}

/// Predefined user agent profiles for common use cases
pub mod profiles {
    use super::*;

    /// Standard desktop Chrome user agent
    pub fn chrome_desktop() -> UserAgentManager {
        UserAgentManager::new()
            .with_browser(Browser::Chrome)
    }

    /// Standard desktop Firefox user agent
    pub fn firefox_desktop() -> UserAgentManager {
        UserAgentManager::new()
            .with_browser(Browser::Firefox)
    }

    /// Standard desktop Safari user agent
    pub fn safari_desktop() -> UserAgentManager {
        UserAgentManager::new()
            .with_browser(Browser::Safari)
    }

    /// Standard desktop Edge user agent
    pub fn edge_desktop() -> UserAgentManager {
        UserAgentManager::new()
            .with_browser(Browser::Edge)
    }

    /// iPhone Safari user agent
    pub fn iphone() -> String {
        "Mozilla/5.0 (iPhone; CPU iPhone OS 17_0 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.0 Mobile/15E148 Safari/604.1".to_string()
    }

    /// Android Chrome user agent
    pub fn android() -> String {
        "Mozilla/5.0 (Linux; Android 14; SM-S918B) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/122.0.0.0 Mobile Safari/537.36".to_string()
    }

    /// iPad Safari user agent
    pub fn ipad() -> String {
        "Mozilla/5.0 (iPad; CPU OS 17_0 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.0 CriOS/122.0.0.0 Mobile/15E148 Safari/604.1".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_manager() {
        let manager = UserAgentManager::new();
        assert_eq!(manager.browser, Browser::Chrome);
        assert_eq!(manager.rotation, RotationStrategy::Fixed);
    }

    #[test]
    fn test_chrome_ua() {
        let ua = Browser::Chrome.user_agent(Some("120.0.0.0"));
        assert!(ua.contains("Chrome/120.0.0.0"));
        assert!(ua.contains("Macintosh"));
    }

    #[test]
    fn test_custom_ua() {
        let manager = UserAgentManager::new()
            .with_custom("Custom UA/1.0");
        assert_eq!(manager.get_user_agent(), "Custom UA/1.0");
    }

    #[test]
    fn test_iphone_ua() {
        let ua = profiles::iphone();
        assert!(ua.contains("iPhone"));
        assert!(ua.contains("Mobile"));
    }
}