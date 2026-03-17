//! CLI command handlers

use scrapio_ai::AiScraper;
use scrapio_ai::BrowserAiScraper;
use scrapio_ai::RalphLoopOptions;
use scrapio_browser::{
    ChromeDriverManager, ChromeDriverSession, StealthBrowser, StealthConfig, StealthLevel,
};
use scrapio_classic::Scraper;
use scrapio_runtime::Runtime;
use scrapio_storage::Storage;

/// Output format options
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    Json,
    Csv,
    Text,
}

impl OutputFormat {
    /// Parse output format from string (case-insensitive)
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "json" => OutputFormat::Json,
            "csv" => OutputFormat::Csv,
            _ => OutputFormat::Text,
        }
    }
}

/// Write output content to file or stdout
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

/// Options for AI scraping
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
fn parse_stealth_level(stealth: Option<&str>) -> StealthLevel {
    match stealth {
        Some("basic") => StealthLevel::Basic,
        Some("advanced") => StealthLevel::Advanced,
        Some("full") | Some(_) => StealthLevel::Full,
        None => StealthLevel::None,
    }
}

async fn start_driver(driver_path: Option<&str>) -> Option<ChromeDriverSession> {
    let result = if let Some(path) = driver_path {
        ChromeDriverSession::start_with(ChromeDriverManager::new().with_path(path.into())).await
    } else {
        ChromeDriverSession::start().await
    };

    match result {
        Ok(driver) => Some(driver),
        Err(e) => {
            eprintln!("Failed to start ChromeDriver: {}", e);
            None
        }
    }
}

async fn init_browser(
    driver_url: &str,
    headless: bool,
    stealth_level: StealthLevel,
) -> Option<StealthBrowser> {
    let mut builder = StealthBrowser::with_webdriver(driver_url).headless(headless);

    if stealth_level != StealthLevel::None {
        let config = StealthConfig::new(stealth_level);
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

async fn print_page_info(browser: &mut StealthBrowser) {
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
        let scraper = AiScraper::with_config(config);
        let schema = schema.unwrap_or_else(|| "{}".to_string());
        match scraper.scrape(url, &schema).await {
            Ok(result) => {
                let format = OutputFormat::from_str(output);
                let output_content = match format {
                    OutputFormat::Json => serde_json::to_string_pretty(&result).unwrap_or_default(),
                    OutputFormat::Csv => {
                        // For CSV, output as key-value pairs
                        let mut csv_output = String::new();
                        csv_output.push_str("url,model,extraction_mode,links_count\n");
                        csv_output.push_str(&format!(
                            "{},{},{:?},{}\n",
                            result.url,
                            result.model,
                            result.mode,
                            result.links.len()
                        ));
                        // Also output data as separate rows
                        if let Ok(data_json) = serde_json::to_string(&result.data) {
                            csv_output.push_str("data\n");
                            csv_output.push_str(&data_json);
                        }
                        csv_output
                    }
                    OutputFormat::Text => {
                        // Default text output
                        let mut text_output = String::new();
                        text_output.push_str(&format!("URL: {}\n", result.url));
                        text_output.push_str(&format!("Model: {}\n", result.model));
                        text_output.push_str(&format!("Extraction mode: {:?}\n", result.mode));
                        if let Some(ref reason) = result.fallback_reason {
                            text_output.push_str(&format!("Fallback reason: {:?}\n", reason));
                        }
                        if let Some(ref error) = result.provider_error {
                            text_output.push_str(&format!("Provider error: {}\n", error));
                        }
                        text_output.push_str(&format!("Links found: {}\n", result.links.len()));
                        text_output.push_str(&format!(
                            "Data: {}",
                            serde_json::to_string_pretty(&result.data).unwrap_or_default()
                        ));
                        text_output
                    }
                };

                // Write to file or stdout
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

        let scraper = BrowserAiScraper::with_config(config).with_max_steps(options.max_steps);

        let ralph_options = RalphLoopOptions {
            url: &options.url,
            schema: options.schema.as_deref().unwrap_or("[]"),
            custom_prompt: &options.prompt,
            max_iterations: None,
            max_steps_per_iteration: Some(options.max_steps),
            stealth_level: Some(StealthLevel::Basic),
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
                        // CSV output: one row per extracted target
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
                        // Default text output
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

                // Write to file or stdout
                write_output(&output_content, options.output_file.as_deref());
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

pub fn handle_list(database: &str, limit: usize, output: &str) {
    run_async(async {
        match Storage::new(database).await {
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
) {
    run_async(async {
        let stealth_level = parse_stealth_level(stealth);

        let driver = match start_driver(driver_path).await {
            Some(d) => d,
            None => return,
        };

        let mut browser = match init_browser(&driver.webdriver_url(), headless, stealth_level).await
        {
            Some(b) => b,
            None => return,
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
        if let Some(script_path) = script
            && let Ok(script_content) = std::fs::read_to_string(script_path)
        {
            match browser.execute_script(&script_content).await {
                Ok(result) => println!("Script result: {}", result),
                Err(e) => eprintln!("Script execution failed: {}", e),
            }
        }

        // Print page info
        print_page_info(&mut browser).await;

        // Close browser
        if let Err(e) = browser.close().await {
            eprintln!("Warning: Failed to close browser cleanly: {}", e);
        }

        println!("Browser session complete.");
    });
}
