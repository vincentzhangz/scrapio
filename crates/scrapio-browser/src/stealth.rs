//! Stealth configuration and script generation for browser automation

use scrapio_core::user_agent::UserAgentManager;
use serde::{Deserialize, Serialize};

/// Stealth level determines which anti-detection measures are applied
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StealthLevel {
    /// No stealth measures - uses regular browser automation
    None,
    /// Basic stealth - removes navigator.webdriver flag
    Basic,
    /// Advanced stealth - adds canvas fingerprint randomization, WebGL spoofing
    Advanced,
    /// Full stealth - viewport randomization, timezone/locale settings
    #[default]
    Full,
}

/// Configuration for stealth browser features
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StealthConfig {
    /// Stealth level to apply
    pub level: StealthLevel,
    /// User agent manager
    pub user_agent: Option<UserAgentManager>,
    /// Canvas fingerprint randomization seed
    pub canvas_seed: Option<u64>,
    /// Viewport width range for randomization (min, max)
    pub viewport_range: Option<(u32, u32)>,
    /// Timezone to simulate (e.g., "America/New_York")
    pub timezone: Option<String>,
    /// Locale to simulate (e.g., "en-US")
    pub locale: Option<String>,
}

impl Default for StealthConfig {
    fn default() -> Self {
        Self {
            level: StealthLevel::Full,
            user_agent: None,
            canvas_seed: None,
            viewport_range: None,
            timezone: None,
            locale: None,
        }
    }
}

impl StealthConfig {
    /// Create a new stealth config with the given level
    pub fn new(level: StealthLevel) -> Self {
        Self {
            level,
            ..Default::default()
        }
    }

    /// Set user agent manager
    pub fn with_user_agent(mut self, user_agent: UserAgentManager) -> Self {
        self.user_agent = Some(user_agent);
        self
    }

    /// Set canvas seed for fingerprint randomization
    pub fn with_canvas_seed(mut self, seed: u64) -> Self {
        self.canvas_seed = Some(seed);
        self
    }

    /// Set viewport range for randomization
    pub fn with_viewport_range(mut self, min: u32, max: u32) -> Self {
        self.viewport_range = Some((min, max));
        self
    }

    /// Set timezone to simulate
    pub fn with_timezone(mut self, timezone: impl Into<String>) -> Self {
        self.timezone = Some(timezone.into());
        self
    }

    /// Set locale to simulate
    pub fn with_locale(mut self, locale: impl Into<String>) -> Self {
        self.locale = Some(locale.into());
        self
    }

    /// Generate stealth JavaScript based on config
    pub fn generate_script(&self) -> String {
        let mut scripts = Vec::new();

        // Always remove webdriver flag for any stealth level above None
        if self.level != StealthLevel::None {
            scripts.push(get_basic_stealth_script());
        }

        if self.level == StealthLevel::Advanced || self.level == StealthLevel::Full {
            scripts.push(get_advanced_stealth_script());
            if let Some(seed) = self.canvas_seed {
                scripts.push(get_canvas_randomization_script(seed));
            } else {
                scripts.push(get_canvas_randomization_script(42));
            }
        }

        if self.level == StealthLevel::Full {
            scripts.push(get_full_stealth_script());

            if let Some(ref timezone) = self.timezone {
                scripts.push(get_timezone_script(timezone));
            }

            if let Some(ref locale) = self.locale {
                scripts.push(get_locale_script(locale));
            }
        }

        scripts.join("\n")
    }

    /// Get the user agent string if configured
    pub fn get_user_agent(&self) -> Option<String> {
        self.user_agent.as_ref().map(|ua| ua.get_user_agent())
    }

    /// Check if automation should be hidden
    pub fn should_hide_automation(&self) -> bool {
        self.user_agent
            .as_ref()
            .map(|ua| ua.should_hide_automation())
            .unwrap_or(self.level != StealthLevel::None)
    }
}

/// Get basic stealth script - removes navigator.webdriver flag
fn get_basic_stealth_script() -> String {
    r#"
Object.defineProperty(navigator, 'webdriver', {
    get: () => undefined,
    configurable: true
});

// Override chrome runtime
if (window.chrome) {
    window.chrome.runtime = {
        connect: () => {},
        sendMessage: () => {}
    };
}

// Remove detection flags
window.navigator.__proto__ = new Proxy(window.navigator.__proto__, {
    get(target, property) {
        if (property === 'webdriver') {
            return undefined;
        }
        return target[property];
    }
});
"#
    .to_string()
}

/// Get advanced stealth script - canvas fingerprint randomization, WebGL spoofing
fn get_advanced_stealth_script() -> String {
    r#"
// Override getParameter to randomize WebGL fingerprint
const getParameter = WebGLRenderingContext.prototype.getParameter;
WebGLRenderingContext.prototype.getParameter = function(parameter) {
    if (parameter === 37445) {
        return 'Intel Inc.';
    }
    if (parameter === 37446) {
        return 'Intel Iris OpenGL Engine';
    }
    return getParameter.apply(this, arguments);
};

// Spoof permissions
const originalQuery = window.navigator.permissions.query;
window.navigator.permissions.query = (parameters) => (
    parameters.name === 'notifications' ?
        Promise.resolve({ state: Notification.permission }) :
        originalQuery(parameters)
);

// Randomize plugins
Object.defineProperty(navigator, 'plugins', {
    get: () => [1, 2, 3, 4, 5],
    configurable: true
});

// Randomize languages
Object.defineProperty(navigator, 'languages', {
    get: () => ['en-US', 'en'],
    configurable: true
});
"#
    .to_string()
}

/// Get canvas randomization script
fn get_canvas_randomization_script(seed: u64) -> String {
    format!(
        r#"
// Canvas fingerprint randomization
const __canvasSeed = {};
const __originalToDataURL = HTMLCanvasElement.prototype.toDataURL;
const __originalGetImageData = CanvasRenderingContext2D.prototype.getImageData;

let __noiseOffset = __canvasSeed;

function __addNoise() {{
    __noiseOffset = (__noiseOffset * 9301 + 49297) % 233280;
    return __noiseOffset / 233280 - 0.5;
}}

HTMLCanvasElement.prototype.toDataURL = function() {{
    const args = arguments;
    const result = __originalToDataURL.apply(this, args);
    if (this.width > 0 && this.height > 0) {{
        return result + '?noise=' + __addNoise();
    }}
    return result;
}};

CanvasRenderingContext2D.prototype.getImageData = function() {{
    const args = arguments;
    const result = __originalGetImageData.apply(this, args);
    if (this.canvas.width > 0 && this.canvas.height > 0) {{
        for (let i = 0; i < result.data.length; i += 4) {{
            const noise = __addNoise() * 10;
            result.data[i] = Math.max(0, Math.min(255, result.data[i] + noise));
            result.data[i + 1] = Math.max(0, Math.min(255, result.data[i + 1] + noise));
            result.data[i + 2] = Math.max(0, Math.min(255, result.data[i + 2] + noise));
        }}
    }}
    return result;
}};
"#,
        seed
    )
}

/// Get full stealth script - viewport randomization, timezone, locale
fn get_full_stealth_script() -> String {
    r#"
// Override screen properties for viewport randomization
const __randomInt = (min, max) => Math.floor(Math.random() * (max - min + 1)) + min;

Object.defineProperty(screen, 'width', {
    get: () => __randomInt(1200, 1920),
    configurable: true
});

Object.defineProperty(screen, 'height', {
    get: () => __randomInt(700, 1080),
    configurable: true
});

Object.defineProperty(screen, 'availWidth', {
    get: () => __randomInt(1200, 1920),
    configurable: true
});

Object.defineProperty(screen, 'availHeight', {
    get: () => __randomInt(700, 1080),
    configurable: true
});

// Override document.visibilityState
Object.defineProperty(document, 'visibilityState', {
    get: () => 'visible',
    configurable: true
});

Object.defineProperty(document, 'hidden', {
    get: () => false,
    configurable: true
});

// Prevent focused element detection
window.isSecureContext = true;

// Add fake performance entries
const __originalPerformanceNow = performance.now;
performance.now = () => __originalPerformanceNow() + __randomInt(0, 100);

// Override Hardware Concurrency
Object.defineProperty(navigator, 'hardwareConcurrency', {
    get: () => __randomInt(4, 8),
    configurable: true
});

// Override Device Memory
Object.defineProperty(navigator, 'deviceMemory', {
    get: () => 8,
    configurable: true
});
"#
    .to_string()
}

/// Get timezone script
fn get_timezone_script(timezone: &str) -> String {
    format!(
        r#"
// Timezone override
Intl.DateTimeFormat = new Proxy(Intl.DateTimeFormat, {{
    construct(target, args) {{
        const opts = args[1] || {{}};
        if (!opts.timeZone) {{
            opts.timeZone = '{}';
        }}
        return new target(args[0], opts);
    }}
}});
"#,
        timezone
    )
}

/// Get locale script
fn get_locale_script(locale: &str) -> String {
    format!(
        r#"
// Locale override
Object.defineProperty(navigator, 'language', {{
    get: () => '{}',
    configurable: true
}});

Object.defineProperty(navigator, 'languages', {{
    get: () => ['{}'],
    configurable: true
}});
"#,
        locale, locale
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stealth_config_default() {
        let config = StealthConfig::default();
        assert_eq!(config.level, StealthLevel::Full);
    }

    #[test]
    fn test_stealth_config_builder() {
        use scrapio_core::UserAgentManager;

        let config = StealthConfig::new(StealthLevel::Basic)
            .with_user_agent(UserAgentManager::new().with_custom("Custom Agent"))
            .with_canvas_seed(12345);

        assert_eq!(config.level, StealthLevel::Basic);
        assert!(config.user_agent.is_some());
        assert_eq!(config.canvas_seed, Some(12345));
    }

    #[test]
    fn test_generate_script_basic() {
        let config = StealthConfig::new(StealthLevel::Basic);
        let script = config.generate_script();
        assert!(script.contains("Object.defineProperty"));
        assert!(script.contains("navigator"));
    }

    #[test]
    fn test_generate_script_full() {
        let config = StealthConfig::new(StealthLevel::Full)
            .with_timezone("America/New_York")
            .with_locale("en-US");
        let script = config.generate_script();

        assert!(script.contains("Object.defineProperty"));
        assert!(script.contains("navigator"));
        assert!(script.contains("WebGLRenderingContext"));
        assert!(script.contains("America/New_York"));
        assert!(script.contains("en-US"));
    }
}
