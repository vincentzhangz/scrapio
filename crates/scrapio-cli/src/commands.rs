//! CLI command handlers

use scrapio_ai::AiScraper;
use scrapio_ai::BrowserAiScraper;
use scrapio_browser::{StealthBrowser, StealthConfig, StealthLevel};
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

pub fn handle_ai(url: &str, schema: Option<String>, provider: &str, model: &str, use_browser: bool, prompt: &str) {
    if use_browser {
        handle_ai_browser(url, schema, provider, model, prompt);
    } else {
        handle_ai_http(url, schema, provider, model);
    }
}

fn handle_ai_http(url: &str, schema: Option<String>, provider: &str, model: &str) {
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

fn handle_ai_browser(url: &str, schema: Option<String>, provider: &str, model: &str, prompt: &str) {
    run_async(async {
        // Kill any existing ChromeDriver on port 9515
        scrapio_browser::ChromeDriverManager::kill_existing(9515);
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;

        // Auto-download and start ChromeDriver
        println!("Setting up ChromeDriver...");
        let mut driver_manager = scrapio_browser::ChromeDriverManager::new();

        let child = match driver_manager.download_and_start(9515).await {
            Ok(c) => {
                println!("ChromeDriver started on port 9515");
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                c
            }
            Err(e) => {
                eprintln!("Failed to start ChromeDriver: {}", e);
                return;
            }
        };

        // Use a scope to ensure ChromeDriver is stopped even on error
        let result = async {
            let mut config = scrapio_ai::AiConfig::new().with_provider(provider);
            if !model.is_empty() {
                config = config.with_model(model);
            }

            let scraper = BrowserAiScraper::with_config(config);
            let schema = schema.unwrap_or_else(|| "{}".to_string());

            println!("\nUsing browser automation for AI scraping...");
            println!("URL: {}", url);
            if !prompt.is_empty() {
                println!("Prompt: {}", prompt);
            }
            println!("Schema: {}", schema);

            scraper.scrape_with_prompt(url, &schema, prompt).await
        }.await;

        // Always stop ChromeDriver when done
        scrapio_browser::ChromeDriverManager::stop(child);

        match result {
            Ok(result) => {
                println!("\n--- Result ---");
                println!("URL: {}", result.url);
                println!("Model: {}", result.model);
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

pub fn handle_browser(url: &str, headless: bool, stealth: Option<&str>, script: Option<&str>) {
    run_async(async {
        // Determine stealth level
        let stealth_level = match stealth {
            Some("basic") => StealthLevel::Basic,
            Some("advanced") => StealthLevel::Advanced,
            Some("full") | Some(_) => StealthLevel::Full,
            None => StealthLevel::None,
        };

        // Build browser config
        let mut builder = StealthBrowser::new().headless(headless);

        if stealth_level != StealthLevel::None {
            let config = StealthConfig::new(stealth_level);
            builder = builder.stealth(config);
        }

        // Execute custom script if provided
        let custom_script = script.and_then(|s| std::fs::read_to_string(s).ok());

        let mut browser = match builder.init().await {
            Ok(b) => b,
            Err(e) => {
                eprintln!("Failed to initialize browser: {}", e);
                return;
            }
        };

        // Navigate to URL
        match browser.goto(url).await {
            Ok(_) => {
                println!("Successfully navigated to: {}", url);
            }
            Err(e) => {
                eprintln!("Failed to navigate: {}", e);
                let _ = browser.close().await;
                return;
            }
        }

        // Execute custom script if provided
        if let Some(ref script_content) = custom_script {
            match browser.execute_script(script_content).await {
                Ok(result) => {
                    println!("Script result: {}", result);
                }
                Err(e) => {
                    eprintln!("Script execution failed: {}", e);
                }
            }
        }

        // Get page info
        match browser.title().await {
            Ok(title) => println!("Page title: {}", title),
            Err(e) => eprintln!("Failed to get title: {}", e),
        }

        match browser.url().await {
            Ok(current_url) => println!("Current URL: {}", current_url),
            Err(e) => eprintln!("Failed to get URL: {}", e),
        }

        // Get page source
        match browser.html().await {
            Ok(html) => {
                let preview = if html.len() > 500 {
                    format!("{}...", &html[..500])
                } else {
                    html
                };
                println!("Page HTML preview:\n{}", preview);
            }
            Err(e) => eprintln!("Failed to get HTML: {}", e),
        }

        // Close browser
        if let Err(e) = browser.close().await {
            eprintln!("Warning: Failed to close browser cleanly: {}", e);
        }

        println!("Browser session complete.");
    });
}
