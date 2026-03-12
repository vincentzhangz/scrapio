//! Classic scraping example using CSS selectors
//!
//! Run with: cargo run --example classic
use scrapio_classic::Scraper;
use scrapio_runtime::{Runtime, TokioRuntime};

fn main() {
    let runtime = TokioRuntime::default();
    runtime.block_on(async {
        let scraper = Scraper::new();

        match scraper.scrape("https://www.rust-lang.org/").await {
            Ok(response) => {
                println!("Status: {}", response.status);
                println!("Title: {:?}", response.title());
                println!("Links found: {}", response.links().len());

                // Use CSS selectors to extract specific elements
                for element in response.select("h1") {
                    println!("H1: {}", element.inner_html());
                }

                for element in response.select("p") {
                    println!("Paragraph: {}", element.inner_html());
                }
            }
            Err(e) => eprintln!("Error: {}", e),
        }
    });
}
