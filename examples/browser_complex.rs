//! Complex browser automation example with stealth, screenshots, and element interaction

use scrapio_browser::{StealthBrowser, StealthConfig, StealthLevel};
use scrapio_core::{Browser, UserAgentManager};
use scrapio_runtime::{Runtime, TokioRuntime};

fn main() {
    let runtime = TokioRuntime::default();
    runtime.block_on(async {
        println!("=== Complex Browser Automation Example ===\n");

        // Setup: Custom UserAgent with Stealth
        println!("Setting up browser with stealth config...");
        let ua = UserAgentManager::new()
            .with_browser(Browser::Chrome)
            .with_version("120.0");

        let config = StealthConfig::new(StealthLevel::Full)
            .with_user_agent(ua)
            .with_timezone("America/New_York")
            .with_locale("en-US");

        let mut browser = StealthBrowser::new()
            .headless(true)
            .stealth(config)
            .window_size(1920, 1080);

        // Step 1: Navigate to rust-lang.org and screenshot
        println!("\n--- Step 1: Go to rust-lang.org ---");
        if let Err(e) = browser.goto("https://www.rust-lang.org").await {
            eprintln!("Failed to navigate: {}", e);
            return;
        }

        if let Ok(title) = browser.title().await {
            println!("Page title: {}", title);
        }

        // Screenshot before click
        if let Ok(screenshot) = browser.screenshot().await {
            std::fs::write("rust_home.png", &screenshot).ok();
            println!("Screenshot saved to rust_home.png");
        }

        // Step 2: Click the install button and wait for navigation
        println!("\n--- Step 2: Click Install Button ---");

        // Try to find and click install link/button
        // The install button on rust-lang.org is typically in nav or has specific selectors
        let install_clicked = match browser.click("a[href*='install']").await {
            Ok(_) => {
                println!("Clicked install link");
                true
            }
            Err(_) => {
                // Try alternative selectors
                match browser.click(".install-button a").await {
                    Ok(_) => {
                        println!("Clicked .install-button a");
                        true
                    }
                    Err(_) => {
                        // Try direct navigation to install page
                        println!("Could not find install button, navigating directly...");
                        match browser.goto("https://www.rust-lang.org/install.html").await {
                            Ok(_) => {
                                println!("Navigated to install page directly");
                                true
                            }
                            Err(e) => {
                                println!("Failed to navigate to install: {}", e);
                                false
                            }
                        }
                    }
                }
            }
        };

        if install_clicked {
            // Wait for page to navigate/load
            println!("Waiting for page to load...");
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;

            // Get new page title
            if let Ok(title) = browser.title().await {
                println!("New page title: {}", title);
            }

            // Screenshot after navigation
            if let Ok(screenshot) = browser.screenshot().await {
                std::fs::write("rust_install.png", &screenshot).ok();
                println!("Screenshot saved to rust_install.png");
            }
        }

        // Step 3: Scroll to bottom and screenshot
        println!("\n--- Step 3: Scroll to Bottom ---");

        // First scroll to bottom
        if let Err(e) = browser.scroll_to_bottom().await {
            eprintln!("Scroll to bottom failed: {}", e);
        } else {
            println!("Scrolled to bottom");
            // Wait for scroll to complete and page to render
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        }

        // Screenshot at bottom
        if let Ok(screenshot) = browser.screenshot().await {
            std::fs::write("rust_bottom.png", &screenshot).ok();
            println!("Screenshot saved to rust_bottom.png");
        }

        // Example: User Agent Profiles
        println!("\n--- User Agent Profiles ---");
        println!(
            "Chrome: {}",
            scrapio_core::profiles::chrome_desktop().get_user_agent()
        );
        println!(
            "Firefox: {}",
            scrapio_core::profiles::firefox_desktop().get_user_agent()
        );
        println!("iPhone: {}", scrapio_core::profiles::iphone());
        println!("Android: {}", scrapio_core::profiles::android());

        // Close browser
        if let Err(e) = browser.close().await {
            eprintln!("Failed to close: {}", e);
        }

        println!("\n=== Example Complete ===");
        println!("Screenshots created: rust_home.png, rust_install.png, rust_bottom.png");
    });
}