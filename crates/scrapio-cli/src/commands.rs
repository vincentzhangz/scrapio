use scrapio_classic::Scraper;
use scrapio_runtime::Runtime;

/// Output format options
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    Json,
    Csv,
    Text,
}

impl OutputFormat {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "json" => OutputFormat::Json,
            "csv" => OutputFormat::Csv,
            _ => OutputFormat::Text,
        }
    }
}

fn write_output(content: &str, output_file: Option<&str>) {
    match output_file {
        Some(path) => match std::fs::write(path, content) {
            Ok(_) => println!("Output saved to: {}", path),
            Err(e) => eprintln!("Error writing to file: {}", e),
        },
        None => println!("{}", content),
    }
}

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

pub struct AiScrapeOptions {
    pub url: String,
    pub schema: Option<String>,
    pub provider: String,
    pub model: String,
    pub use_browser: bool,
    pub prompt: String,
    pub max_steps: usize,
    pub headless: bool,
    pub verbose: bool,
    pub output: String,
    pub output_file: Option<String>,
}

#[allow(clippy::too_many_arguments)]
fn parse_stealth_level(stealth: Option<&str>) -> scrapio_browser::StealthLevel {
    match stealth {
        Some("basic") => scrapio_browser::StealthLevel::Basic,
        Some("advanced") => scrapio_browser::StealthLevel::Advanced,
        Some("full") | Some(_) => scrapio_browser::StealthLevel::Full,
        None => scrapio_browser::StealthLevel::None,
    }
}

async fn start_driver(
    driver_path: Option<&str>,
    browser_type: scrapio_browser::BrowserType,
) -> Option<scrapio_browser::WebDriverSession> {
    let mut manager = scrapio_browser::DriverManager::with_driver_type(
        scrapio_browser::DriverType::parse(&browser_type.to_string())
            .unwrap_or(scrapio_browser::DriverType::Chrome),
    );

    if let Some(path) = driver_path {
        manager = manager.with_path(path.into());
    }

    let result = scrapio_browser::WebDriverSession::start_with(manager).await;

    match result {
        Ok(driver) => Some(driver),
        Err(e) => {
            eprintln!("Failed to start WebDriver: {}", e);
            None
        }
    }
}

async fn init_browser(
    driver_url: &str,
    headless: bool,
    stealth_level: scrapio_browser::StealthLevel,
    browser_type: scrapio_browser::BrowserType,
) -> Option<scrapio_browser::StealthBrowser> {
    let mut builder =
        scrapio_browser::StealthBrowser::with_webdriver_and_type(driver_url, browser_type)
            .headless(headless);

    if stealth_level != scrapio_browser::StealthLevel::None {
        let config = scrapio_browser::StealthConfig::new(stealth_level);
        builder = builder.stealth(config);
    }

    match builder.init().await {
        Ok(browser) => Some(browser),
        Err(e) => {
            eprintln!("Failed to initialize browser: {}", e);
            None
        }
    }
}

async fn print_page_info(browser: &mut scrapio_browser::StealthBrowser) {
    match browser.title().await {
        Ok(title) => println!("Page title: {}", title),
        Err(e) => eprintln!("Failed to get title: {}", e),
    }

    match browser.url().await {
        Ok(current_url) => println!("Current URL: {}", current_url),
        Err(e) => eprintln!("Failed to get URL: {}", e),
    }

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
}

#[allow(dead_code)]
#[allow(clippy::too_many_arguments)]
pub fn handle_ai(
    url: &str,
    schema: Option<String>,
    provider: &str,
    model: &str,
    use_browser: bool,
    prompt: &str,
    max_steps: usize,
    _driver_path: Option<&str>,
    headless: bool,
    verbose: bool,
    output: &str,
    output_file: Option<&str>,
) {
    let options = AiScrapeOptions {
        url: url.to_string(),
        schema,
        provider: provider.to_string(),
        model: model.to_string(),
        use_browser,
        prompt: prompt.to_string(),
        max_steps,
        headless,
        verbose,
        output: output.to_string(),
        output_file: output_file.map(|s| s.to_string()),
    };

    if options.use_browser {
        handle_ai_browser(options);
    } else {
        handle_ai_http(
            &options.url,
            options.schema,
            &options.provider,
            &options.model,
            &options.output,
            options.output_file.as_deref(),
        );
    }
}

fn handle_ai_http(
    url: &str,
    schema: Option<String>,
    provider: &str,
    model: &str,
    output: &str,
    output_file: Option<&str>,
) {
    run_async(async {
        let mut config = scrapio_ai::AiConfig::new().with_provider(provider);
        if !model.is_empty() {
            config = config.with_model(model);
        }
        let scraper = scrapio_ai::AiScraper::with_config(config);
        let schema = schema.unwrap_or_else(|| "{}".to_string());
        match scraper.scrape(url, &schema).await {
            Ok(result) => {
                let format = OutputFormat::from_str(output);
                let output_content = match format {
                    OutputFormat::Json => serde_json::to_string_pretty(&result).unwrap_or_default(),
                    OutputFormat::Csv => {
                        let mut csv_output = String::new();
                        csv_output.push_str("url,model,extraction_mode,links_count\n");
                        csv_output.push_str(&format!(
                            "{},{},{:?},{}\n",
                            result.url,
                            result.model,
                            result.mode,
                            result.links.len()
                        ));
                        if let Ok(data_json) = serde_json::to_string(&result.data) {
                            csv_output.push_str("data\n");
                            csv_output.push_str(&data_json);
                        }
                        csv_output
                    }
                    OutputFormat::Text => {
                        let mut text_output = String::new();
                        text_output.push_str(&format!("URL: {}\n", result.url));
                        text_output.push_str(&format!("Model: {}\n", result.model));
                        text_output.push_str(&format!("Extraction mode: {:?}\n", result.mode));
                        if let Some(ref reason) = result.fallback_reason {
                            text_output.push_str(&format!("Fallback reason: {:?}\n", reason));
                        }
                        if let Some(ref error) = result.provider_error {
                            text_output.push_str(&format!("Provider error: {:?}\n", error));
                        }
                        text_output.push_str(&format!("Links found: {}\n", result.links.len()));
                        text_output.push_str(&format!(
                            "Data: {}",
                            serde_json::to_string_pretty(&result.data).unwrap_or_default()
                        ));
                        text_output
                    }
                };
                write_output(&output_content, output_file);
            }
            Err(e) => eprintln!("Error: {}", e),
        }
    });
}

fn handle_ai_browser(options: AiScrapeOptions) {
    run_async(async {
        let mut config = scrapio_ai::AiConfig::new().with_provider(&options.provider);
        if !options.model.is_empty() {
            config = config.with_model(&options.model);
        }

        let scraper =
            scrapio_ai::BrowserAiScraper::with_config(config).with_max_steps(options.max_steps);

        let ralph_options = scrapio_ai::RalphLoopOptions {
            url: &options.url,
            schema: options.schema.as_deref().unwrap_or("[]"),
            custom_prompt: &options.prompt,
            max_iterations: None,
            max_steps_per_iteration: Some(options.max_steps),
            stealth_level: Some(scrapio_browser::StealthLevel::Basic),
            webdriver_url: None,
            headless: options.headless,
            verbose: options.verbose,
        };

        match scraper.ralph_loop(ralph_options).await {
            Ok(result) => {
                let format = OutputFormat::from_str(&options.output);
                let output_content = match format {
                    OutputFormat::Json => serde_json::to_string_pretty(&result).unwrap_or_default(),
                    OutputFormat::Csv => {
                        let mut wtr = csv::Writer::from_writer(vec![]);
                        for target in &result.progress.targets {
                            if target.extracted {
                                wtr.serialize((
                                    target.id.clone(),
                                    target.description.clone(),
                                    target
                                        .data
                                        .as_ref()
                                        .map(|d| serde_json::to_string(d).unwrap_or_default()),
                                    target.error.clone().unwrap_or_default(),
                                ))
                                .unwrap_or_default();
                            }
                        }
                        let csv_data = wtr.into_inner().unwrap_or_default();
                        String::from_utf8(csv_data).unwrap_or_default()
                    }
                    OutputFormat::Text => {
                        let mut text_output = String::new();
                        text_output.push_str("\nUsing Ralph loop for AI browser scraping...\n");
                        text_output.push_str(&format!("URL: {}\n", options.url));
                        if !options.prompt.is_empty() {
                            text_output.push_str(&format!("Objective: {}\n", options.prompt));
                        }
                        if let Some(ref schema) = options.schema {
                            text_output.push_str(&format!("Schema: {}\n", schema));
                        }
                        text_output.push_str(&format!("Max steps: {}\n", options.max_steps));
                        text_output.push_str(&format!("Headless: {}\n", options.headless));
                        text_output.push_str(&format!("Verbose: {}\n", options.verbose));

                        text_output.push_str("\n=== Ralph Loop Complete ===\n");
                        text_output.push_str(&format!("Stop reason: {:?}\n", result.stop_reason));
                        text_output.push_str(&format!(
                            "Iterations: {}\n",
                            result.progress.iterations_completed
                        ));
                        text_output
                            .push_str(&format!("Total steps: {}\n", result.progress.steps_taken));
                        text_output.push_str("\nExtraction results:\n");

                        for target in &result.progress.targets {
                            let status = if target.extracted { "✓" } else { "✗" };
                            text_output.push_str(&format!(
                                "  {} {}: {}\n",
                                status, target.id, target.description
                            ));
                            if target.extracted {
                                if let Some(data) = &target.data {
                                    text_output.push_str(&format!(
                                        "      Data: {}\n",
                                        serde_json::to_string_pretty(data).unwrap_or_default()
                                    ));
                                }
                            } else if let Some(error) = &target.error {
                                text_output.push_str(&format!("      Error: {}\n", error));
                            }
                        }

                        let extracted = result
                            .progress
                            .targets
                            .iter()
                            .filter(|t| t.extracted)
                            .count();
                        text_output.push_str(&format!(
                            "\nExtracted {}/{} targets",
                            extracted,
                            result.progress.targets.len()
                        ));
                        text_output
                    }
                };

                write_output(&output_content, options.output_file.as_deref());
            }
            Err(e) => eprintln!("Error: {}", e),
        }
    });
}

#[allow(clippy::too_many_arguments)]
pub fn handle_crawl(
    url: &str,
    depth: usize,
    max_pages: Option<usize>,
    scope: Option<&str>,
    extract: bool,
    schema: Option<&str>,
    provider: &str,
    model: &str,
    browser_escalation: &str,
    discover_sitemap: bool,
    discover_robots: bool,
    respect_robotstxt: bool,
    unsafe_mode: bool,
    store_path: &str,
    no_store: bool,
    capture_network: bool,
) {
    use ctrlc::set_handler;
    use scrapio_classic::crawler::{BrowserEscalation, CrawlOptions, Crawler, Scope, ScopeMode};
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};

    // Use atomic flag for shutdown signaling
    let should_shutdown = Arc::new(AtomicBool::new(false));

    // Set up Ctrl+C handler
    let _shutdown_flag = should_shutdown.clone();
    set_handler(move || {
        // Print message first, then exit
        eprintln!("\n\n→ Ctrl+C received! Exiting now...");
        // Small delay to ensure message is printed
        std::thread::sleep(std::time::Duration::from_millis(100));
        std::process::exit(0);
    })
    .expect("Error setting Ctrl+C handler");

    run_async(async {
        // Check shutdown flag periodically during async operations
        if should_shutdown.load(Ordering::SeqCst) {
            eprintln!("→ Already shutting down...");
            return;
        }

        let scope_mode = match scope {
            Some("host") => ScopeMode::Host,
            Some("subdomain") => ScopeMode::Subdomain,
            Some("custom") => ScopeMode::Custom,
            _ => ScopeMode::Domain,
        };
        let scope = Scope::new(scope_mode);

        // Build browser escalation
        let escalation = match browser_escalation {
            "never" => BrowserEscalation::Never,
            "always" => BrowserEscalation::Always,
            _ => BrowserEscalation::Auto,
        };

        // Build options
        let mut options = CrawlOptions::new()
            .with_max_depth(depth)
            .with_max_pages(max_pages.unwrap_or(100))
            .with_scope(scope)
            .with_rate_limit(10)
            .with_browser_escalation(escalation)
            .with_capture_network(capture_network);

        // Apply unsafe or robots.txt settings
        if unsafe_mode {
            options = options.aggressive();
        } else if !respect_robotstxt {
            options = options.ignore_robots_txt();
        }

        // Create channel for incremental saving (if storing)
        let result_tx = if !no_store {
            let (tx, _rx) = tokio::sync::mpsc::channel(100);
            Some(tx)
        } else {
            None
        };

        // Set up result sender for incremental saving
        if let Some(ref tx) = result_tx {
            options = options.with_result_sender(tx.clone());
        }

        // Add AI options if extract is enabled
        if extract {
            options = options
                .with_ai_provider(provider)
                .with_ai_model(model)
                .with_extract_data(true);
            if let Some(s) = schema {
                options = options.with_ai_schema(s);
            }
        }

        let mut crawler = match Crawler::new(url, options) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("Failed to create crawler: {}", e);
                return;
            }
        };

        // Initialize crawler with optional sitemap/robots discovery
        if discover_sitemap || discover_robots {
            println!("Initializing with discovery...");
            crawler
                .init_with_discovery(discover_sitemap, discover_robots)
                .await;
        } else {
            crawler.init().await;
        }

        // Handle AI extraction vs basic crawl
        if extract {
            // AI-assisted crawl
            handle_crawl_with_ai(&mut crawler, schema.unwrap_or("[]"), provider, model).await;
        } else {
            // Basic crawl
            match crawler.crawl().await {
                Ok(results) => {
                    println!("\n=== Crawl Results ===");
                    println!("Total pages crawled: {}", results.len());

                    // Save to database incrementally (each page as it's processed)
                    if !no_store {
                        match scrapio_storage::Storage::new(store_path).await {
                            Ok(storage) => {
                                for result in &results {
                                    // Save immediately after each page is processed
                                    let _ = storage
                                        .save_result(
                                            &result.url,
                                            result.status,
                                            result.title.as_deref(),
                                            "",
                                            &[],
                                        )
                                        .await;

                                    // Print after saving so we can see progress
                                    println!("\n- {} (status: {})", result.url, result.status);
                                    if let Some(ref title) = result.title {
                                        println!("  Title: {}", title);
                                    }
                                    println!("  Depth: {}", result.depth);
                                    println!("  Links found: {}", result.links_found);
                                    if let Some(ref error) = result.error {
                                        println!("  Error: {}", error);
                                    }
                                    println!("  ✓ Saved");
                                }
                                println!(
                                    "\n→ Total saved: {} results to {}",
                                    results.len(),
                                    store_path
                                );
                            }
                            Err(e) => {
                                // If storage fails, still print results
                                for result in &results {
                                    println!("\n- {} (status: {})", result.url, result.status);
                                    if let Some(ref title) = result.title {
                                        println!("  Title: {}", title);
                                    }
                                    println!("  Depth: {}", result.depth);
                                    println!("  Links found: {}", result.links_found);
                                }
                                eprintln!("Failed to save to database: {}", e);
                            }
                        }
                    } else {
                        // No-store mode - just print results
                        for result in &results {
                            println!("\n- {} (status: {})", result.url, result.status);
                            if let Some(ref title) = result.title {
                                println!("  Title: {}", title);
                            }
                            println!("  Depth: {}", result.depth);
                            println!("  Links found: {}", result.links_found);
                        }
                    }

                    println!("\nCrawl complete! Visited {} pages", results.len());
                }
                Err(e) => eprintln!("Crawl error: {}", e),
            }
        }
    });
}

async fn handle_crawl_with_ai(
    crawler: &mut scrapio_classic::crawler::Crawler,
    schema: &str,
    provider: &str,
    model: &str,
) {
    use scrapio_ai::AiConfig;
    use scrapio_ai::AiScraper;

    println!("AI-assisted crawl with extraction...");

    // First, do basic crawl to discover URLs
    match crawler.crawl().await {
        Ok(results) => {
            if results.is_empty() {
                println!("No pages found to extract from.");
                return;
            }

            println!(
                "Discovered {} pages, starting AI extraction...",
                results.len()
            );

            // Set up AI scraper
            let mut config = AiConfig::new().with_provider(provider);
            if !model.is_empty() {
                config = config.with_model(model);
            }
            let ai_scraper = AiScraper::with_config(config);

            // Extract from each page
            for (i, result) in results.iter().enumerate() {
                println!(
                    "\n[{}/{}] Extracting from: {}",
                    i + 1,
                    results.len(),
                    result.url
                );

                match ai_scraper.scrape(&result.url, schema).await {
                    Ok(extraction) => {
                        println!("  Status: {}", extraction.data);
                        if let Some(ref error) = extraction.provider_error {
                            println!("  Warning: {}", error);
                        }
                    }
                    Err(e) => {
                        println!("  Error: {}", e);
                    }
                }

                // Rate limit to avoid overwhelming the API
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            }

            println!("\nAI extraction complete!");
        }
        Err(e) => eprintln!("Crawl error: {}", e),
    }
}

pub fn handle_save(url: &str, database: &str) {
    run_async(async {
        let scraper = Scraper::new();
        match scrapio_storage::Storage::new(database).await {
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

pub fn handle_list(database: &str, limit: usize, output: &str) {
    run_async(async {
        match scrapio_storage::Storage::new(database).await {
            Ok(storage) => match storage.get_all_results(limit).await {
                Ok(results) => {
                    let format = OutputFormat::from_str(output);
                    let output_content = match format {
                        OutputFormat::Json => {
                            serde_json::to_string_pretty(&results).unwrap_or_default()
                        }
                        OutputFormat::Csv => {
                            let mut wtr = csv::Writer::from_writer(vec![]);
                            for r in &results {
                                wtr.serialize(r).unwrap_or_default();
                            }
                            let data = wtr.into_inner().unwrap_or_default();
                            String::from_utf8(data).unwrap_or_default()
                        }
                        OutputFormat::Text => {
                            let mut text = format!("Found {} results:\n", results.len());
                            for r in results {
                                text.push_str(&format!("  {} - {} - {}\n", r.id, r.status, r.url));
                                if let Some(title) = &r.title {
                                    text.push_str(&format!("    Title: {}\n", title));
                                }
                            }
                            text
                        }
                    };
                    write_output(&output_content, None);
                }
                Err(e) => eprintln!("Error: {}", e),
            },
            Err(e) => eprintln!("Database error: {}", e),
        }
    });
}

pub fn handle_browser(
    url: &str,
    headless: bool,
    stealth: Option<&str>,
    script: Option<&str>,
    driver_path: Option<&str>,
    browser: &str,
) {
    let browser_type = scrapio_browser::BrowserType::parse(browser)
        .unwrap_or(scrapio_browser::BrowserType::Chrome);

    run_async(async {
        let stealth_level = parse_stealth_level(stealth);

        let driver = match start_driver(driver_path, browser_type).await {
            Some(d) => d,
            None => return,
        };

        let mut browser = match init_browser(
            &driver.webdriver_url(),
            headless,
            stealth_level,
            browser_type,
        )
        .await
        {
            Some(b) => b,
            None => return,
        };

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

        if let Some(script_path) = script
            && let Ok(script_content) = std::fs::read_to_string(script_path)
        {
            match browser.execute_script(&script_content).await {
                Ok(result) => println!("Script result: {}", result),
                Err(e) => eprintln!("Script execution failed: {}", e),
            }
        }

        print_page_info(&mut browser).await;

        if let Err(e) = browser.close().await {
            eprintln!("Warning: Failed to close browser cleanly: {}", e);
        }

        println!("Browser session complete.");
    });
}
