//! Proxy Rotation Example - rotate through multiple proxies
//!
//! Run with: cargo run --example proxy_rotation
//!
//! This example demonstrates:
//! - Loading multiple proxies from a list
//! - Using different rotation strategies (round-robin, random, per-domain)
//! - Filtering unhealthy proxies

use scrapio_core::proxy::{ProxyConfig, ProxyManager, ProxyRotationConfig, RotationStrategy};
use scrapio_classic::crawler::CrawlOptions;
use scrapio_classic::crawler::Crawler;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Define a list of proxies (in practice, load from file or environment)
    let proxies = vec![
        ProxyConfig::parse("http://proxy1.example.com:8080")
            .unwrap_or_else(|_| ProxyConfig::new("http://proxy1.example.com:8080")),
        ProxyConfig::parse("http://proxy2.example.com:8080")
            .unwrap_or_else(|_| ProxyConfig::new("http://proxy2.example.com:8080")),
        ProxyConfig::parse("http://proxy3.example.com:8080")
            .unwrap_or_else(|_| ProxyConfig::new("http://proxy3.example.com:8080")),
    ];

    println!("Loaded {} proxies", proxies.len());

    // Step 1: Validate all proxies
    let manager = ProxyManager::new();
    println!("\nValidating proxies...");

    let health_results = manager.validate_all(&proxies).await;

    // Filter to healthy proxies only
    let healthy_proxies = ProxyManager::filter_healthy(&health_results);
    println!("{} of {} proxies are healthy", healthy_proxies.len(), proxies.len());

    // Sort by latency (fastest first)
    let mut sorted_results = health_results.clone();
    ProxyManager::sort_by_latency(&mut sorted_results);

    println!("\nProxies by latency:");
    for (proxy, health) in &sorted_results {
        if health.is_alive {
            println!("  {} - {}ms", proxy.url, health.latency_ms);
        } else {
            println!("  {} - UNREACHABLE", proxy.url);
        }
    }

    if healthy_proxies.is_empty() {
        println!("\nNo healthy proxies available. Exiting.");
        return Ok(());
    }

    // Step 2: Create rotation config with different strategies
    println!("\n--- Round Robin Rotation ---");
    let mut round_robin = ProxyRotationConfig::new(healthy_proxies.clone(), RotationStrategy::RoundRobin);

    // Simulate requests
    for i in 0..6 {
        if let Some(proxy) = round_robin.get_proxy(None) {
            println!("Request {}: Using proxy {}", i + 1, proxy.url);
        }
    }

    println!("\n--- Random Rotation ---");
    let mut random = ProxyRotationConfig::new(healthy_proxies.clone(), RotationStrategy::Random);

    for i in 0..6 {
        if let Some(proxy) = random.get_proxy(None) {
            println!("Request {}: Using proxy {}", i + 1, proxy.url);
        }
    }

    println!("\n--- Per-Domain Rotation ---");
    let mut per_domain = ProxyRotationConfig::new(healthy_proxies.clone(), RotationStrategy::PerDomain);

    let domains = ["example.com", "test.com", "example.com", "other.com", "example.com"];
    for (i, domain) in domains.iter().enumerate() {
        if let Some(proxy) = per_domain.get_proxy(Some(domain)) {
            println!("Request {} ({}): Using proxy {}", i + 1, domain, proxy.url);
        }
    }

    // Step 3: Use with Crawler
    println!("\n--- Using with Crawler ---");
    let options = CrawlOptions::new()
        .with_max_depth(1)
        .with_max_pages(5)
        .with_proxy_rotation(healthy_proxies, RotationStrategy::RoundRobin);

    let mut crawler = Crawler::new("https://example.com", options)?;
    crawler.init().await;
    let results = crawler.crawl().await?;

    println!("\nCrawled {} pages", results.len());
    for result in &results {
        println!("  {} - Status: {}", result.url, result.status);
    }

    Ok(())
}
