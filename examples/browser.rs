//! Browser automation example with stealth mode and automatic ChromeDriver management

use scrapio_browser::{ChromeDriverManager, StealthBrowser, StealthLevel};
use scrapio_core::{profiles, UserAgentManager};
use scrapio_runtime::{Runtime, TokioRuntime};

fn main() {
    let runtime = TokioRuntime::default();
    runtime.block_on(async {
        println!("Starting browser automation example...\n");

        // Step 1: Download and start ChromeDriver automatically
        println!("=== Step 1: Setup ChromeDriver ===");
        let mut driver_manager = ChromeDriverManager::new();

        // Optional: Set specific version (or leave empty for latest stable)
        // driver_manager = driver_manager.with_version("146.0.7680.72");

        match driver_manager.download_and_start(9515).await {
            Ok(_child) => {
                println!("ChromeDriver started successfully on port 9515");
                // Give ChromeDriver time to start
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            }
            Err(e) => {
                eprintln!("Failed to start ChromeDriver: {}", e);
                return;
            }
        }

        // Step 2: Create browser with full stealth
        println!("\n=== Step 2: Create Browser ===");
        let mut browser = StealthBrowser::new()
            .headless(true)
            .stealth_level(StealthLevel::Full);

        // Navigate to rust-lang.org
        match browser.goto("https://www.rust-lang.org").await {
            Ok(_) => println!("Successfully navigated to rust-lang.org"),
            Err(e) => {
                eprintln!("Failed to navigate: {}", e);
                return;
            }
        }

        // Get page title
        match browser.title().await {
            Ok(title) => println!("Page title: {}", title),
            Err(e) => eprintln!("Failed to get title: {}", e),
        }

        // Get page URL
        match browser.url().await {
            Ok(url) => println!("Page URL: {}", url),
            Err(e) => eprintln!("Failed to get URL: {}", e),
        }

        // Get page HTML
        match browser.html().await {
            Ok(html) => {
                let preview = if html.len() > 200 {
                    format!("{}...", &html[..200])
                } else {
                    html
                };
                println!("HTML preview:\n{}", preview);
            }
            Err(e) => eprintln!("Failed to get HTML: {}", e),
        }

        // Find elements
        match browser.find_element("h1").await {
            Ok(_elem) => println!("Found H1 element"),
            Err(e) => println!("No H1 element found: {}", e),
        }

        // Close browser
        if let Err(e) = browser.close().await {
            eprintln!("Warning: Failed to close browser: {}", e);
        }

        println!("\n--- Using UserAgentManager ---");

        // Demonstrate UserAgentManager
        let ua = UserAgentManager::new().with_browser(scrapio_core::Browser::Firefox);
        println!("Firefox UA: {}", ua.get_user_agent());

        let iphone_ua = profiles::iphone();
        println!("iPhone UA: {}", iphone_ua);

        let android_ua = profiles::android();
        println!("Android UA: {}", android_ua);

        println!("\nExample complete!");
    });
}