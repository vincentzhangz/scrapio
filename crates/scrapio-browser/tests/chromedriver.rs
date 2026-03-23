//! Tests for chromedriver module (backward compatibility)

// Note: The ChromeDriverManager.channel field is private, so we only test public APIs

#[test]
fn test_chromedriver_module_exists() {
    // Just verify the module is accessible
    use scrapio_browser::ChromeDriverManager;
    let _manager = ChromeDriverManager::new();
}
