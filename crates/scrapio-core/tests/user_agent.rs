//! Tests for user_agent module

use scrapio_core::{Browser, RotationStrategy, UserAgentManager, profiles};

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
fn test_rotation_per_session() {
    let manager = UserAgentManager::new().with_rotation_per_session();
    assert_eq!(manager.rotation, RotationStrategy::PerSession);
}
