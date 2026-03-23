//! Tests for browser module

use scrapio_browser::{BrowserType, StealthBrowser, StealthLevel};

#[test]
fn test_browser_type_parse() {
    assert_eq!(BrowserType::parse("chrome"), Some(BrowserType::Chrome));
    assert_eq!(BrowserType::parse("CHROME"), Some(BrowserType::Chrome));
    assert_eq!(BrowserType::parse("firefox"), Some(BrowserType::Firefox));
    assert_eq!(BrowserType::parse("Firefox"), Some(BrowserType::Firefox));
    assert_eq!(BrowserType::parse("edge"), Some(BrowserType::Edge));
    assert_eq!(BrowserType::parse("invalid"), None);
}

#[test]
fn test_browser_type_default_port() {
    assert_eq!(BrowserType::Chrome.default_port(), 9515);
    assert_eq!(BrowserType::Firefox.default_port(), 4444);
    assert_eq!(BrowserType::Edge.default_port(), 9516);
}

#[test]
fn test_stealth_browser_builder() {
    let _browser = StealthBrowser::new()
        .headless(false)
        .stealth_level(StealthLevel::Basic)
        .window_size(1280, 720)
        .arg("--disable-extensions");
}

#[test]
fn test_browser_builder_all_browsers() {
    // Test all browser types can be created
    let _chrome = StealthBrowser::with_browser_type(BrowserType::Chrome).headless(true);
    let _firefox = StealthBrowser::with_browser_type(BrowserType::Firefox).headless(true);
    let _edge = StealthBrowser::with_browser_type(BrowserType::Edge).headless(true);
}
