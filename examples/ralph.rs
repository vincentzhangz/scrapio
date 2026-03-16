//! Ralph loop example - iterates through schema targets until all extracted
//!
//! Run with: cargo run --example ralph
//!
//! This example demonstrates the Ralph loop pattern which iterates through
//! extraction targets until all are completed, similar to the Ralph agent pattern.

use scrapio_ai::BrowserAiScraper;
use scrapio_ai::RalphLoopOptions;
use scrapio_browser::StealthLevel;

#[tokio::main]
async fn main() {
    println!("=== Ralph Loop Example ===\n");
    println!("This example extracts multiple targets from rust-lang.org using the Ralph loop.\n");

    // Define the schema with multiple extraction targets
    let schema = r#"[
        {"id": "title", "description": "Extract the main page title"},
        {"id": "navigation_links", "description": "Extract all navigation menu links"},
        {"id": "hero_heading", "description": "Extract the main hero section heading"},
        {"id": "install_button", "description": "Find and extract the install button text/link"},
        {"id": "footer_links", "description": "Extract all links in the footer"}
    ]"#;

    let scraper = BrowserAiScraper::new();

    let options = RalphLoopOptions {
        url: "https://www.rust-lang.org",
        schema,
        custom_prompt: "Focus on accurate extraction of the requested elements. Navigate the page if needed to find the information.",
        max_iterations: Some(20),
        max_steps_per_iteration: Some(10),
        stealth_level: Some(StealthLevel::Basic),
        webdriver_url: None,
        headless: true,
        verbose: false,
    };

    println!("Starting Ralph loop...");
    println!("URL: {}", options.url);
    println!("Targets: 5");
    println!("Max iterations: {:?}\n", options.max_iterations);

    match scraper.ralph_loop(options).await {
        Ok(result) => {
            println!("=== Ralph Loop Complete ===\n");
            println!("Stop reason: {:?}", result.stop_reason);
            println!("Iterations: {}", result.progress.iterations_completed);
            println!("Total steps: {}", result.progress.steps_taken);
            println!("\nExtraction results:");

            for target in &result.progress.targets {
                let status = if target.extracted { "✓" } else { "✗" };
                println!("\n  {} {}", status, target.id);
                println!("      {}", target.description);

                if target.extracted {
                    if let Some(data) = &target.data {
                        let data_str = serde_json::to_string_pretty(data).unwrap_or_default();
                        // Truncate long output
                        if data_str.len() > 200 {
                            println!("      Data: {}...", &data_str[..200]);
                        } else {
                            println!("      Data: {}", data_str);
                        }
                    }
                } else if let Some(error) = &target.error {
                    println!("      Error: {}", error);
                }
            }

            println!("\n=== Summary ===");
            let extracted = result
                .progress
                .targets
                .iter()
                .filter(|t| t.extracted)
                .count();
            let total = result.progress.targets.len();
            println!("Extracted: {}/{} targets", extracted, total);

            if result.progress.is_complete {
                println!("Status: All targets extracted successfully!");
            } else {
                println!("Status: Loop completed but some targets were not extracted.");
                println!("Try increasing --max-iterations or --max-steps");
            }
        }
        Err(e) => {
            eprintln!("Ralph loop error: {}", e);
            eprintln!("\nNote: Make sure you have an AI API key set:");
            eprintln!("  - OPENAI_API_KEY for OpenAI");
            eprintln!("  - ANTHROPIC_API_KEY for Anthropic");
        }
    }
}
