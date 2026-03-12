//! CLI command handlers

use scrapio_ai::AiScraper;
use scrapio_classic::Scraper;
use scrapio_runtime::Runtime;
use scrapio_storage::Storage;

fn run_async<F: std::future::Future>(f: F) -> F::Output {
    scrapio_runtime::TokioRuntime::default().block_on(f)
}

pub fn handle_classic(url: &str) {
    run_async(async {
        let scraper = Scraper::new();
        match scraper.scrape(url).await {
            Ok(resp) => {
                println!("Status: {}", resp.status);
                if let Some(title) = resp.title() {
                    println!("Title: {}", title);
                }
                println!("Links: {}", resp.links().len());
            }
            Err(e) => eprintln!("Error: {}", e),
        }
    });
}

pub fn handle_ai(url: &str, schema: Option<String>, provider: &str, model: &str) {
    run_async(async {
        let mut config = scrapio_ai::AiConfig::new().with_provider(provider);
        if !model.is_empty() {
            config = config.with_model(model);
        }
        let scraper = AiScraper::with_config(config);
        let schema = schema.unwrap_or_else(|| "{}".to_string());
        match scraper.scrape(url, &schema).await {
            Ok(result) => {
                println!("URL: {}", result.url);
                println!("Model: {}", result.model);
                println!("Used fallback: {}", result.used_fallback);
                println!("Links found: {}", result.links.len());
                println!(
                    "Data: {}",
                    serde_json::to_string_pretty(&result.data).unwrap_or_default()
                );
            }
            Err(e) => eprintln!("Error: {}", e),
        }
    });
}

pub fn handle_crawl(url: &str, depth: usize) {
    run_async(async {
        let scraper = Scraper::new();
        let mut visited = std::collections::HashSet::new();
        let mut queue = vec![url.to_string()];
        let mut current_depth = 0;

        while current_depth < depth && !queue.is_empty() {
            let urls: Vec<String> = std::mem::take(&mut queue);
            println!("Depth {}: processing {} URLs", current_depth, urls.len());

            for url in urls {
                if visited.contains(&url) {
                    continue;
                }
                visited.insert(url.clone());
                match scraper.scrape(&url).await {
                    Ok(resp) => {
                        println!("  - {} (status: {})", url, resp.status);
                        if let Some(title) = resp.title() {
                            println!("    Title: {}", title.trim());
                        }
                        if current_depth + 1 < depth {
                            for link in resp.links() {
                                if !visited.contains(&link) && link.starts_with("http") {
                                    queue.push(link);
                                }
                            }
                        }
                    }
                    Err(e) => eprintln!("  - {} Error: {}", url, e),
                }
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            }
            current_depth += 1;
        }
        println!("\nCrawl complete! Visited {} pages", visited.len());
    });
}

pub fn handle_save(url: &str, database: &str) {
    run_async(async {
        let scraper = Scraper::new();
        match Storage::new(database).await {
            Ok(storage) => match scraper.scrape(url).await {
                Ok(resp) => {
                    let title = resp.title();
                    let links = resp.links();
                    match storage
                        .save_result(url, resp.status, title.as_deref(), &resp.html, &links)
                        .await
                    {
                        Ok(id) => println!("Saved result with ID: {}", id),
                        Err(e) => eprintln!("Save error: {}", e),
                    }
                }
                Err(e) => eprintln!("Scrape error: {}", e),
            },
            Err(e) => eprintln!("Database error: {}", e),
        }
    });
}

pub fn handle_list(database: &str, limit: usize) {
    run_async(async {
        match Storage::new(database).await {
            Ok(storage) => match storage.get_all_results(limit).await {
                Ok(results) => {
                    println!("Found {} results:\n", results.len());
                    for r in results {
                        println!("  {} - {} - {}", r.id, r.status, r.url);
                        if let Some(title) = &r.title {
                            println!("    Title: {}", title);
                        }
                    }
                }
                Err(e) => eprintln!("Error: {}", e),
            },
            Err(e) => eprintln!("Database error: {}", e),
        }
    });
}
