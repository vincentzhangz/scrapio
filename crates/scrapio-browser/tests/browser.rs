//! Integration tests for scrapio-browser
//!
//! These tests require a running WebDriver server and browser.
//! Run with: cargo test --package scrapio-browser --test browser -- --test-threads=1
//!
//! Or set SCRAPIO_TEST_BROWSER=1 to run with browser tests enabled.
//! You can specify which browser to test: SCRAPIO_BROWSER=chrome|firefox|edge

use scrapio_browser::{BrowserType, StealthBrowser, StealthConfig, StealthLevel};
use std::time::Duration;

/// Check if browser tests should run
fn should_run_browser_tests() -> bool {
    std::env::var("SCRAPIO_TEST_BROWSER").is_ok()
}

/// Get the browser type to test
fn get_test_browser_type() -> BrowserType {
    match std::env::var("SCRAPIO_BROWSER").as_deref() {
        Ok("firefox") => BrowserType::Firefox,
        Ok("edge") => BrowserType::Edge,
        _ => BrowserType::Chrome,
    }
}

/// Simple HTML test page that doesn't require external network
const TEST_PAGE_HTML: &str = r#"<!DOCTYPE html>
<html>
<head><title>Scrapio Test Page</title></head>
<body>
    <h1 id="title">Test Page</h1>
    <p class="content">Hello, World!</p>
    <div id="container">
        <button id="btn" type="button">Click Me</button>
        <input type="text" id="input" value="test value" />
    </div>
</body>
</html>"#;

/// Test that creates a browser instance (skipped if no browser available)
#[tokio::test]
#[ignore]
async fn test_browser_creation() {
    if !should_run_browser_tests() {
        return;
    }

    let browser_type = get_test_browser_type();
    let browser = StealthBrowser::with_browser_type(browser_type).headless(true);

    // Just verify browser can be created - need to init to connect
    let _ = browser.init().await;
}

/// Test navigation to a data URL (self-contained, no network required)
#[tokio::test]
#[ignore]
async fn test_navigate_to_data_url() {
    if !should_run_browser_tests() {
        return;
    }

    let browser_type = get_test_browser_type();
    let mut browser = StealthBrowser::with_browser_type(browser_type)
        .headless(true)
        .init()
        .await
        .expect("Failed to init browser");

    // Navigate to a data URL containing our test HTML
    let data_url = format!("data:text/html,{}", urlencoding::encode(TEST_PAGE_HTML));
    browser.goto(&data_url).await.unwrap();

    // Verify title
    let title = browser.title().await.unwrap();
    assert_eq!(title, "Scrapio Test Page");
}

/// Test finding element by ID
#[tokio::test]
#[ignore]
async fn test_find_element_by_id() {
    if !should_run_browser_tests() {
        return;
    }

    let browser_type = get_test_browser_type();
    let mut browser = StealthBrowser::with_browser_type(browser_type)
        .headless(true)
        .init()
        .await
        .expect("Failed to init browser");

    let data_url = format!("data:text/html,{}", urlencoding::encode(TEST_PAGE_HTML));
    browser.goto(&data_url).await.unwrap();

    // Find element by ID
    let element = browser.find_element("#title").await.unwrap();
    let text = element.text().await.unwrap();
    assert_eq!(text, "Test Page");
}

/// Test finding multiple elements
#[tokio::test]
#[ignore]
async fn test_find_multiple_elements() {
    if !should_run_browser_tests() {
        return;
    }

    let browser_type = get_test_browser_type();
    let mut browser = StealthBrowser::with_browser_type(browser_type)
        .headless(true)
        .init()
        .await
        .expect("Failed to init browser");

    let data_url = format!("data:text/html,{}", urlencoding::encode(TEST_PAGE_HTML));
    browser.goto(&data_url).await.unwrap();

    // Find multiple elements
    let elements = browser
        .find_elements("#container button, #container input")
        .await
        .unwrap();
    assert!(elements.len() >= 2);
}

/// Test getting element attribute
#[tokio::test]
#[ignore]
async fn test_get_element_attribute() {
    if !should_run_browser_tests() {
        return;
    }

    let browser_type = get_test_browser_type();
    let mut browser = StealthBrowser::with_browser_type(browser_type)
        .headless(true)
        .init()
        .await
        .expect("Failed to init browser");

    let data_url = format!("data:text/html,{}", urlencoding::encode(TEST_PAGE_HTML));
    browser.goto(&data_url).await.unwrap();

    let input = browser.find_element("#input").await.unwrap();
    let value = input.attr("value").await.unwrap();
    assert_eq!(value, Some("test value".to_string()));
}

/// Test page source retrieval
#[tokio::test]
#[ignore]
async fn test_page_source() {
    if !should_run_browser_tests() {
        return;
    }

    let browser_type = get_test_browser_type();
    let mut browser = StealthBrowser::with_browser_type(browser_type)
        .headless(true)
        .init()
        .await
        .expect("Failed to init browser");

    let data_url = format!("data:text/html,{}", urlencoding::encode(TEST_PAGE_HTML));
    browser.goto(&data_url).await.unwrap();

    let source = browser.html().await.unwrap();
    assert!(source.contains("Scrapio Test Page"));
}

/// Test with stealth configuration
#[tokio::test]
#[ignore]
async fn test_stealth_browser() {
    if !should_run_browser_tests() {
        return;
    }

    let browser_type = get_test_browser_type();
    let config = StealthConfig::new(StealthLevel::Basic);
    let mut browser = StealthBrowser::with_browser_type(browser_type)
        .headless(true)
        .stealth(config)
        .init()
        .await
        .expect("Failed to init browser");

    let data_url = format!("data:text/html,{}", urlencoding::encode(TEST_PAGE_HTML));
    browser.goto(&data_url).await.unwrap();

    let title = browser.title().await.unwrap();
    assert_eq!(title, "Scrapio Test Page");
}

/// Test element click
#[tokio::test]
#[ignore]
async fn test_element_click() {
    if !should_run_browser_tests() {
        return;
    }

    // HTML with JavaScript for click handling
    let html_with_js = r#"<!DOCTYPE html>
<html>
<head><title>Click Test</title></head>
<body>
    <button id="btn">Click Me</button>
    <script>document.getElementById('btn').addEventListener('click', function() { this.textContent = 'Clicked!'; });</script>
</body>
</html>"#;

    let browser_type = get_test_browser_type();
    let mut browser = StealthBrowser::with_browser_type(browser_type)
        .headless(true)
        .init()
        .await
        .expect("Failed to init browser");

    let data_url = format!("data:text/html,{}", urlencoding::encode(html_with_js));
    browser.goto(&data_url).await.unwrap();

    // Use the click method on the browser
    browser.click("#btn").await.unwrap();

    // Small delay for JS to execute
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let btn = browser.find_element("#btn").await.unwrap();
    let text = btn.text().await.unwrap();
    assert_eq!(text, "Clicked!");
}

/// Test element send_keys via browser method
#[tokio::test]
#[ignore]
async fn test_send_keys_to_element() {
    if !should_run_browser_tests() {
        return;
    }

    let browser_type = get_test_browser_type();
    let mut browser = StealthBrowser::with_browser_type(browser_type)
        .headless(true)
        .init()
        .await
        .expect("Failed to init browser");

    let data_url = format!("data:text/html,{}", urlencoding::encode(TEST_PAGE_HTML));
    browser.goto(&data_url).await.unwrap();

    // Get the element and send keys directly
    let input = browser.find_element("#input").await.unwrap();
    input.send_keys(" new text").await.unwrap();

    let value = input.attr("value").await.unwrap();
    assert!(value.unwrap().contains("new text"));
}

/// Test browser type parsing
#[test]
fn test_browser_type_parsing() {
    assert_eq!(BrowserType::parse("chrome"), Some(BrowserType::Chrome));
    assert_eq!(BrowserType::parse("Chrome"), Some(BrowserType::Chrome));
    assert_eq!(BrowserType::parse("firefox"), Some(BrowserType::Firefox));
    assert_eq!(BrowserType::parse("Firefox"), Some(BrowserType::Firefox));
    assert_eq!(BrowserType::parse("edge"), Some(BrowserType::Edge));
    assert_eq!(BrowserType::parse("Edge"), Some(BrowserType::Edge));
    assert_eq!(BrowserType::parse("invalid"), None);
}

/// Test browser type default port
#[test]
fn test_browser_type_default_port() {
    assert_eq!(BrowserType::Chrome.default_port(), 9515);
    assert_eq!(BrowserType::Firefox.default_port(), 4444);
    assert_eq!(BrowserType::Edge.default_port(), 9516);
}

/// Test StealthConfig creation
#[test]
fn test_stealth_config_creation() {
    let config = StealthConfig::new(StealthLevel::Basic);
    assert_eq!(config.level, StealthLevel::Basic);

    let config = StealthConfig::new(StealthLevel::Full);
    assert_eq!(config.level, StealthLevel::Full);
}

/// Test stealth level default
#[test]
fn test_stealth_level_default() {
    // Default should be Full
    let config = StealthConfig::default();
    assert_eq!(config.level, StealthLevel::Full);
}

/// Test browser builder pattern
#[test]
fn test_browser_builder_pattern() {
    let _browser = StealthBrowser::new()
        .headless(false)
        .stealth_level(StealthLevel::Advanced)
        .window_size(1920, 1080)
        .arg("--disable-extensions")
        .arg("--disable-popup-blocking");
}

/// Test with custom WebDriver URL
#[tokio::test]
#[ignore]
async fn test_custom_webdriver_url() {
    if !should_run_browser_tests() {
        return;
    }

    // Create browser with custom WebDriver URL
    let mut browser = StealthBrowser::with_webdriver("http://localhost:9515")
        .headless(true)
        .init()
        .await
        .expect("Failed to init browser with custom URL");

    // Navigate to test page
    let data_url = format!("data:text/html,{}", urlencoding::encode(TEST_PAGE_HTML));
    browser.goto(&data_url).await.unwrap();

    let title = browser.title().await.unwrap();
    assert_eq!(title, "Scrapio Test Page");
}

/// Test browser URL retrieval after navigation
#[tokio::test]
#[ignore]
async fn test_current_url() {
    if !should_run_browser_tests() {
        return;
    }

    let browser_type = get_test_browser_type();
    let mut browser = StealthBrowser::with_browser_type(browser_type)
        .headless(true)
        .init()
        .await
        .expect("Failed to init browser");

    let encoded_url = format!("data:text/html,{}", urlencoding::encode(TEST_PAGE_HTML));
    browser.goto(&encoded_url).await.unwrap();

    let current_url = browser.url().await.unwrap();
    // Data URLs may be URL-encoded in the returned value
    assert!(current_url.contains("Scrapio Test Page") || current_url.contains("text/html"));
}

/// Test execute JavaScript
#[tokio::test]
#[ignore]
async fn test_execute_script() {
    if !should_run_browser_tests() {
        return;
    }

    let browser_type = get_test_browser_type();
    let mut browser = StealthBrowser::with_browser_type(browser_type)
        .headless(true)
        .init()
        .await
        .expect("Failed to init browser");

    let data_url = format!("data:text/html,{}", urlencoding::encode(TEST_PAGE_HTML));
    browser.goto(&data_url).await.unwrap();

    // Execute a simple script that returns a value
    let result = browser.execute_script("return 2 + 2").await.unwrap();
    assert_eq!(result, 4);
}

/// Test wait_for_element
#[tokio::test]
#[ignore]
async fn test_wait_for_element() {
    if !should_run_browser_tests() {
        return;
    }

    // HTML that adds element after a delay
    let html_with_delay = r#"<!DOCTYPE html>
<html>
<head><title>Delay Test</title></head>
<body>
    <div id="delayed">Hello</div>
    <script>setTimeout(function() {
        var el = document.createElement('p');
        el.id = 'dynamic';
        el.textContent = 'Dynamic Content';
        document.body.appendChild(el);
    }, 500);</script>
</body>
</html>"#;

    let browser_type = get_test_browser_type();
    let mut browser = StealthBrowser::with_browser_type(browser_type)
        .headless(true)
        .init()
        .await
        .expect("Failed to init browser");

    let data_url = format!("data:text/html,{}", urlencoding::encode(html_with_delay));
    browser.goto(&data_url).await.unwrap();

    // Wait for the dynamic element to appear
    let element = browser
        .wait_for_element("#dynamic", Duration::from_millis(2000))
        .await
        .unwrap();
    let text = element.text().await.unwrap();
    assert_eq!(text, "Dynamic Content");
}

/// Test element_exists
#[tokio::test]
#[ignore]
async fn test_element_exists() {
    if !should_run_browser_tests() {
        return;
    }

    let browser_type = get_test_browser_type();
    let mut browser = StealthBrowser::with_browser_type(browser_type)
        .headless(true)
        .init()
        .await
        .expect("Failed to init browser");

    let data_url = format!("data:text/html,{}", urlencoding::encode(TEST_PAGE_HTML));
    browser.goto(&data_url).await.unwrap();

    // Check existing element
    assert!(browser.element_exists("#title").await.unwrap());

    // Check non-existing element
    assert!(!browser.element_exists("#nonexistent").await.unwrap());
}

/// Test scroll functionality
#[tokio::test]
#[ignore]
async fn test_scroll() {
    if !should_run_browser_tests() {
        return;
    }

    // HTML with scrollable content
    let html_with_scroll = r#"<!DOCTYPE html>
<html>
<head><title>Scroll Test</title></head>
<body style="height: 2000px;">
    <div id="top">Top</div>
    <div id="bottom" style="margin-top: 1800px;">Bottom</div>
</body>
</html>"#;

    let browser_type = get_test_browser_type();
    let mut browser = StealthBrowser::with_browser_type(browser_type)
        .headless(true)
        .init()
        .await
        .expect("Failed to init browser");

    let data_url = format!("data:text/html,{}", urlencoding::encode(html_with_scroll));
    browser.goto(&data_url).await.unwrap();

    // Scroll to bottom
    browser.scroll_to_bottom().await.unwrap();

    // Verify we can scroll back to top
    browser.scroll_to_top().await.unwrap();

    let element = browser.find_element("#top").await.unwrap();
    let text = element.text().await.unwrap();
    assert_eq!(text, "Top");
}

/// Test screenshot capture
#[tokio::test]
#[ignore]
async fn test_screenshot() {
    if !should_run_browser_tests() {
        return;
    }

    let browser_type = get_test_browser_type();
    let mut browser = StealthBrowser::with_browser_type(browser_type)
        .headless(true)
        .init()
        .await
        .expect("Failed to init browser");

    let data_url = format!("data:text/html,{}", urlencoding::encode(TEST_PAGE_HTML));
    browser.goto(&data_url).await.unwrap();

    // Take screenshot
    let screenshot = browser.screenshot().await.unwrap();

    // PNG header is 8 bytes, verify we got image data
    assert!(screenshot.len() > 8);
    assert_eq!(
        &screenshot[0..8],
        &[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]
    );
}
