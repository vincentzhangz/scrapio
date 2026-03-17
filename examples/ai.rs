//! AI-powered scraping example
//!
//! Run with: cargo run --example ai
//!
//! Uses Ollama with glm-5:cloud model (requires local Ollama server running)
use scrapio_ai::{AiConfig, AiScraper};
use scrapio_runtime::{Runtime, TokioRuntime};

fn main() {
    let runtime = TokioRuntime::default();
    runtime.block_on(async {
        // Configure AI scraper
        let config = AiConfig::new()
            .with_provider("ollama")
            .with_model("glm-5:cloud");

        let scraper = AiScraper::with_config(config);

        // Define extraction schema
        let schema = r#"{
            "title": "string",
            "description": "string",
            "links": "array"
        }"#;

        match scraper.scrape("https://www.rust-lang.org/", schema).await {
            Ok(result) => {
                println!("URL: {}", result.url);
                println!("Model: {}", result.model);
                println!("Extraction mode: {:?}", result.mode);
                if let Some(ref reason) = result.fallback_reason {
                    println!("Fallback reason: {:?}", reason);
                }
                if let Some(ref error) = result.provider_error {
                    println!("Provider error: {}", error);
                }
                println!("Links found: {}", result.links.len());
                println!(
                    "Data: {}",
                    serde_json::to_string_pretty(&result.data).unwrap()
                );
            }
            Err(e) => eprintln!("Error: {}", e),
        }
    });
}
