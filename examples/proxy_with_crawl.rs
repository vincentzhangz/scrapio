//! Proxy with Crawl Example - large-scale crawling with proxy rotation
//!
//! Run with: cargo run --example proxy_with_crawl
//!
//! This example demonstrates:
//! - Setting up proxy rotation for large-scale crawling
//! - Loading proxies from a file
//! - Handling proxy failures gracefully
//! - Tracking proxy usage per domain

use scrapio_classic::crawler::{CrawlOptions, Crawler};
use scrapio_core::proxy::{ProxyConfig, RotationStrategy};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Example 1: Single proxy for all requests
    println!("=== Single Proxy Example ===");

    if let Ok(proxy) = ProxyConfig::parse("http://proxy.example.com:8080") {
        let options = CrawlOptions::new()
            .with_max_depth(2)
            .with_max_pages(10)
            .with_proxy(proxy);

        println!("Crawling with single proxy...");
        let mut crawler = Crawler::new("https://example.com", options)?;
        crawler.init().await;
        let results = crawler.crawl().await?;
        println!("Crawled {} pages", results.len());
    }

    // Example 2: Proxy list with rotation
    println!("\n=== Proxy Rotation Example ===");

    let proxies = vec![
        ProxyConfig::new("http://proxy1.example.com:8080"),
        ProxyConfig::new("http://proxy2.example.com:8080"),
        ProxyConfig::new("http://proxy3.example.com:8080"),
    ];

    let options = CrawlOptions::new()
        .with_max_depth(2)
        .with_max_pages(10)
        .with_proxy_rotation(proxies.clone(), RotationStrategy::RoundRobin);

    println!("Crawling with {} proxies (round-robin)...", proxies.len());
    let mut crawler = Crawler::new("https://example.com", options)?;
    crawler.init().await;
    let results = crawler.crawl().await?;
    println!("Crawled {} pages", results.len());

    // Example 3: Load proxies from file
    println!("\n=== Load Proxies from File ===");

    // Create a sample proxy file for demonstration
    std::fs::write(
        "proxies.txt",
        "# Proxy list\nhttp://proxy1.example.com:8080\nhttp://proxy2.example.com:8080\nhttp://proxy3.example.com:8080\n",
    )?;

    let mut options = CrawlOptions::new().with_max_depth(2).with_max_pages(10);

    match options.with_proxy_list_file("proxies.txt", RotationStrategy::Random) {
        Ok(_) => {
            println!("Loaded proxies from proxies.txt");
            println!("Crawling with random rotation...");
            let mut crawler = Crawler::new("https://example.com", options)?;
            crawler.init().await;
            let results = crawler.crawl().await?;
            println!("Crawled {} pages", results.len());
        }
        Err(e) => {
            eprintln!("Failed to load proxies from file: {}", e);
        }
    }

    // Clean up demo file
    let _ = std::fs::remove_file("proxies.txt");

    // Example 4: Per-domain proxy assignment (maintains sessions per domain)
    println!("\n=== Per-Domain Proxy Assignment ===");

    let proxies = vec![
        ProxyConfig::new("http://proxy-a.example.com:8080"),
        ProxyConfig::new("http://proxy-b.example.com:8080"),
    ];

    let options = CrawlOptions::new()
        .with_max_depth(1)
        .with_max_pages(5)
        .with_proxy_rotation(proxies, RotationStrategy::PerDomain);

    println!("Crawling with per-domain proxy assignment...");
    let mut crawler = Crawler::new("https://example.com", options)?;
    crawler.init().await;
    let results = crawler.crawl().await?;

    for result in &results {
        println!("  {} (used_browser: {})", result.url, result.used_browser);
    }

    Ok(())
}
