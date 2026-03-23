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
        let ver = version.unwrap_or("146.0.7680.153");
        match self {
            Browser::Chrome => format!(
                "Mozilla/5.0 (Macintosh; Intel Mac OS X 15_2) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/{} Safari/537.36",
                ver
            ),
            Browser::Firefox => format!(
                "Mozilla/5.0 (Macintosh; Intel Mac OS X 15.2; rv:128.0) Gecko/20100101 Firefox/{}",
                ver
            ),
            Browser::Safari => "Mozilla/5.0 (Macintosh; Intel Mac OS X 15_2) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/18.2 Safari/605.1.15".to_string(),
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
        UserAgentManager::new().with_browser(Browser::Chrome)
    }

    /// Standard desktop Firefox user agent
    pub fn firefox_desktop() -> UserAgentManager {
        UserAgentManager::new().with_browser(Browser::Firefox)
    }

    /// Standard desktop Safari user agent
    pub fn safari_desktop() -> UserAgentManager {
        UserAgentManager::new().with_browser(Browser::Safari)
    }

    /// Standard desktop Edge user agent
    pub fn edge_desktop() -> UserAgentManager {
        UserAgentManager::new().with_browser(Browser::Edge)
    }

    /// iPhone Safari user agent
    pub fn iphone() -> String {
        "Mozilla/5.0 (iPhone; CPU iPhone OS 26_3_1 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/26.0 Mobile/15E148 Safari/604.1".to_string()
    }

    /// Android Chrome user agent
    pub fn android() -> String {
        "Mozilla/5.0 (Linux; Android 15; SM-S918B) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/146.0.7680.153 Mobile Safari/537.36".to_string()
    }

    /// iPad Safari user agent
    pub fn ipad() -> String {
        "Mozilla/5.0 (iPad; CPU OS 26_3_1 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/26.0 CriOS/146.0.7680.153 Mobile/15E148 Safari/604.1".to_string()
    }

    /// Collection of pre-defined user agents for different platforms
    pub mod collection {
        /// Chrome on Windows 11
        pub fn chrome_windows() -> String {
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/146.0.7680.153 Safari/537.36".to_string()
        }

        /// Chrome on macOS Sequoia (15.x)
        pub fn chrome_mac() -> String {
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 15_2) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/146.0.7680.153 Safari/537.36".to_string()
        }

        /// Firefox on Windows 11
        pub fn firefox_windows() -> String {
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:128.0) Gecko/20100101 Firefox/148.0"
                .to_string()
        }

        /// Firefox on macOS Sequoia
        pub fn firefox_mac() -> String {
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 15.2; rv:128.0) Gecko/20100101 Firefox/148.0"
                .to_string()
        }

        /// Safari on macOS Sequoia
        pub fn safari_mac() -> String {
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 15_2) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/18.2 Safari/605.1.15".to_string()
        }

        /// Edge on Windows 11
        pub fn edge_windows() -> String {
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/146.0.7680.153 Safari/537.36 Edg/146.0.7680.153".to_string()
        }

        /// Safari on iPhone (iOS 26)
        pub fn safari_iphone() -> String {
            "Mozilla/5.0 (iPhone; CPU iPhone OS 26_3_1 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/26.0 Mobile/15E148 Safari/604.1".to_string()
        }

        /// Safari on iPad (iPadOS 26)
        pub fn safari_ipad() -> String {
            "Mozilla/5.0 (iPad; CPU OS 26_3_1 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/26.0 CriOS/146.0.7680.153 Mobile/15E148 Safari/604.1".to_string()
        }

        /// Chrome on Android 15
        pub fn chrome_android() -> String {
            "Mozilla/5.0 (Linux; Android 15; SM-S918B) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/146.0.7680.153 Mobile Safari/537.36".to_string()
        }

        /// Chrome on Linux
        pub fn chrome_linux() -> String {
            "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/146.0.7680.153 Safari/537.36".to_string()
        }
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
        let manager = UserAgentManager::new().with_custom("Custom UA/1.0");
        assert_eq!(manager.get_user_agent(), "Custom UA/1.0");
    }

    #[test]
    fn test_iphone_ua() {
        let ua = profiles::iphone();
        assert!(ua.contains("iPhone"));
        assert!(ua.contains("Mobile"));
    }

    #[test]
    fn test_firefox_browser() {
        let ua = Browser::Firefox.user_agent(Some("148.0"));
        assert!(ua.contains("Firefox/148.0"));
        assert!(ua.contains("Gecko/20100101"));
    }

    #[test]
    fn test_safari_browser() {
        let ua = Browser::Safari.user_agent(None);
        assert!(ua.contains("Safari"));
        assert!(ua.contains("Version/18.2"));
    }

    #[test]
    fn test_edge_browser() {
        let ua = Browser::Edge.user_agent(Some("146.0.7680.153"));
        assert!(ua.contains("Edg/146.0.7680.153"));
    }

    #[test]
    fn test_collection_chrome_windows() {
        let ua = profiles::collection::chrome_windows();
        assert!(ua.contains("Chrome/146"));
        assert!(ua.contains("Windows NT 10.0"));
    }

    #[test]
    fn test_collection_chrome_mac() {
        let ua = profiles::collection::chrome_mac();
        assert!(ua.contains("Chrome/146"));
        assert!(ua.contains("Mac OS X 15"));
    }

    #[test]
    fn test_collection_firefox_windows() {
        let ua = profiles::collection::firefox_windows();
        assert!(ua.contains("Firefox/148"));
        assert!(ua.contains("Windows NT 10.0"));
    }

    #[test]
    fn test_collection_safari_mac() {
        let ua = profiles::collection::safari_mac();
        assert!(ua.contains("Safari/605.1.15"));
        assert!(ua.contains("Version/18.2"));
    }

    #[test]
    fn test_collection_android() {
        let ua = profiles::collection::chrome_android();
        assert!(ua.contains("Android 15"));
        assert!(ua.contains("Mobile Safari"));
    }

    #[test]
    fn test_profiles_android() {
        let ua = profiles::android();
        assert!(ua.contains("Android"));
        assert!(ua.contains("Chrome/146"));
    }

    #[test]
    fn test_profiles_ipad() {
        let ua = profiles::ipad();
        assert!(ua.contains("iPad"));
        assert!(ua.contains("Safari"));
    }
}
