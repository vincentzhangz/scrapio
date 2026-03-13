//! Complex browser automation example with ChromeDriverManager, stealth, screenshots, and element interaction

use scrapio_browser::{
    ChromeDriverChannel, ChromeDriverManager, StealthBrowser, StealthConfig, StealthLevel,
};
use scrapio_core::{Browser, UserAgentManager};
use scrapio_runtime::{Runtime, TokioRuntime};

fn main() {
    let runtime = TokioRuntime::default();
    runtime.block_on(async {
        println!("=== Complex Browser Automation Example ===\n");

        // Step 1: Setup ChromeDriver with auto-download
        println!("--- Step 1: Setup ChromeDriver ---");
        let mut driver_manager =
            ChromeDriverManager::new().with_channel(ChromeDriverChannel::Stable);

        // Optional: Use specific version
        // let mut driver_manager = ChromeDriverManager::new()
        //     .with_version("146.0.7680.72");

        match driver_manager.download_and_start(9515).await {
            Ok(_child) => {
                println!("ChromeDriver started on port 9515");
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            }
            Err(e) => {
                eprintln!("Failed to start ChromeDriver: {}", e);
                return;
            }
        }

        // Step 2: Setup browser with custom UserAgent and Stealth
        println!("\n--- Step 2: Setup Browser with Stealth ---");
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

        // Navigate to rust-lang.org and screenshot
        println!("\n--- Step 3: Go to rust-lang.org ---");
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

        // Step 4: Click install button and wait for navigation
        println!("\n--- Step 4: Click Install Button ---");
        let install_clicked = match browser.click("a[href*='install']").await {
            Ok(_) => {
                println!("Clicked install link");
                true
            }
            Err(_) => {
                // Try direct navigation
                println!("Could not find install button, navigating directly...");
                match browser.goto("https://www.rust-lang.org/install.html").await {
                    Ok(_) => {
                        println!("Navigated to install page directly");
                        true
                    }
                    Err(e) => {
                        println!("Failed: {}", e);
                        false
                    }
                }
            }
        };

        if install_clicked {
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;

            if let Ok(title) = browser.title().await {
                println!("New page title: {}", title);
            }

            // Screenshot after navigation
            if let Ok(screenshot) = browser.screenshot().await {
                std::fs::write("rust_install.png", &screenshot).ok();
                println!("Screenshot saved to rust_install.png");
            }
        }

        // Step 5: Scroll to bottom and screenshot
        println!("\n--- Step 5: Scroll to Bottom ---");
        if let Err(e) = browser.scroll_to_bottom().await {
            eprintln!("Scroll to bottom failed: {}", e);
        } else {
            println!("Scrolled to bottom");
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        }

        if let Ok(screenshot) = browser.screenshot().await {
            std::fs::write("rust_bottom.png", &screenshot).ok();
            println!("Screenshot saved to rust_bottom.png");
        }

        // Step 6: Element interaction examples
        println!("\n--- Step 6: Element Interaction ---");
        match browser.find_elements("a").await {
            Ok(elems) => println!("Found {} links", elems.len()),
            Err(e) => println!("Error: {}", e),
        }

        // JavaScript execution
        let js_result = browser.execute_script("document.title").await;
        println!("Title via JS: {:?}", js_result);

        // Close browser
        if let Err(e) = browser.close().await {
            eprintln!("Failed to close: {}", e);
        }

        // Show User Agent profiles
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

        println!("\n=== Example Complete ===");
        println!("Screenshots: rust_home.png, rust_install.png, rust_bottom.png");
    });
}
