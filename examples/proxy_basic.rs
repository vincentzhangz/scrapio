//! Proxy Basic Example - scrape through a single proxy
//!
//! Run with: cargo run --example proxy_basic
//!
//! This example demonstrates:
//! - Creating a ProxyConfig from a URL
//! - Using a proxy with the HTTP scraper
//! - Validating proxy health before use

use scrapio_core::proxy::{ProxyConfig, ProxyManager};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Parse proxy from URL string
    // Format: http://host:port or http://user:pass@host:port
    let proxy = ProxyConfig::parse("http://proxy.example.com:8080")
        .unwrap_or_else(|_| ProxyConfig::new("http://proxy.example.com:8080"));

    println!("Testing proxy: {}", proxy);

    // Optional: validate proxy before use
    let manager = ProxyManager::new();
    println!("Checking proxy health...");

    let health = manager.validate_proxy(&proxy).await?;

    if health.is_alive {
        println!("Proxy is healthy! Latency: {}ms", health.latency_ms);

        // Use proxy with scraper
        let scraper = scrapio_classic::Scraper::with_proxy(proxy.clone())?;

        match scraper.scrape("https://example.com").await {
            Ok(response) => {
                println!("Successfully scraped through proxy!");
                println!("Title: {:?}", response.title());
            }
            Err(e) => {
                eprintln!("Failed to scrape: {}", e);
            }
        }
    } else {
        println!("Proxy is not reachable. Falling back to direct connection.");

        // Fall back to direct connection
        let scraper = scrapio_classic::Scraper::new();
        match scraper.scrape("https://example.com").await {
            Ok(response) => {
                println!("Successfully scraped directly!");
                println!("Title: {:?}", response.title());
            }
            Err(e) => {
                eprintln!("Failed to scrape: {}", e);
            }
        }
    }

    Ok(())
}
