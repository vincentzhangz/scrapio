# Scrapio

All-in-one web scraping toolkit with AI and non-AI capabilities.

## Features

- **Classic Scraping** - Rule-based scraping with CSS selectors
- **AI-Powered Scraping** - Intelligent content extraction using LLMs
- **Spider System** - Spider framework for defining crawl behavior
- **Item Pipelines** - Process and export scraped data (JSON, CSV)
- **SQLite Storage** - Persist crawl results
- **Multiple LLM Providers** - OpenAI, Anthropic, Ollama (local)

## Installation

### From Source

```bash
# Clone the repository
git clone https://github.com/vincentzhangz/scrapio.git
cd scrapio

# Build the project
cargo build --release

# Run
cargo run -- --help
```

## Quick Start

### Classic Scraping

```bash
# Basic scraping
scrapio classic https://example.com

# Output:
# Status: 200
# Title: Example Domain
# Links: 1
```

### AI-Powered Scraping

```bash
# Set your API key
export OPENAI_API_KEY=your-api-key

# AI scraping (uses fallback if no key)
scrapio ai https://example.com

# With custom schema
scrapio ai https://example.com --schema '{"title": "string", "links": "array"}'

# Use specific provider (openai, anthropic, ollama)
scrapio ai https://example.com --provider ollama
```

### Using AI with Ollama (Local Models)

Ollama allows you to run LLM models locally. This is useful for:
- Free usage without API costs
- Privacy (data stays local)
- Custom/private deployments

```bash
# 1. Install Ollama first (https://ollama.com)
# Then pull a model:
ollama pull llama3
# or
ollama pull mistral
# or
ollama pull phi3

# 2. Start Ollama server (runs on port 11434 by default)
ollama serve

# 3. Use Scrapio with Ollama
scrapio ai https://example.com --provider ollama
```

### Selecting Models

You can specify which model to use with the `--model` flag:

```bash
# OpenAI models
scrapio ai https://example.com --provider openai --model gpt-4o
scrapio ai https://example.com --provider openai --model gpt-4
scrapio ai https://example.com --provider openai --model gpt-3.5-turbo

# Anthropic models
scrapio ai https://example.com --provider anthropic --model claude-sonnet-4-20250514
scrapio ai https://example.com --provider anthropic --model claude-3-opus-20240229
scrapio ai https://example.com --provider anthropic --model claude-3-haiku-20240307

# Ollama models (must be installed locally)
scrapio ai https://example.com --provider ollama --model llama3
scrapio ai https://example.com --provider ollama --model mistral
scrapio ai https://example.com --provider ollama --model phi3
scrapio ai https://example.com --provider ollama --model codellama
```

#### Environment Variables

```bash
# OpenAI
export OPENAI_API_KEY=your-openai-key

# Anthropic
export ANTHROPIC_API_KEY=your-anthropic-key

# Ollama (optional - defaults to http://localhost:11434)
export OLLAMA_BASE_URL=http://localhost:11434
```

### Website Crawling

```bash
# Crawl a website with depth limit
scrapio crawl https://example.com --depth 3
```

### Save to Database

```bash
# Save result to SQLite
scrapio save https://example.com --database scrapio.db

# List saved results
scrapio list --database scrapio.db
```

### Start API Server

```bash
# Start server
scrapio serve --host 127.0.0.1 --port 8080
```

### API Server Usage

Once the server is running, you can use the following curl commands:

```bash
# Health check
curl http://127.0.0.1:8080/health

# Classic scraping (non-AI)
curl -X POST http://127.0.0.1:8080/scrape \
  -H "Content-Type: application/json" \
  -d '{"url": "https://rust-lang.org", "ai": false}'

# AI-powered scraping
curl -X POST http://127.0.0.1:8080/scrape \
  -H "Content-Type: application/json" \
  -d '{"url": "https://rust-lang.org", "ai": true}'

# List all saved results
curl http://127.0.0.1:8080/results

# Get specific result by ID
curl http://127.0.0.1:8080/results/1
```

## Usage as a Library

### Add to your Cargo.toml

```toml
[dependencies]
scrapio-classic = { path = "path/to/scrapio/crates/scrapio-classic" }
scrapio-ai = { path = "path/to/scrapio/crates/scrapio-ai" }
scrapio-storage = { path = "path/to/scrapio/crates/scrapio-storage" }
scrapio-runtime = { path = "path/to/scrapio/crates/scrapio-runtime" }
```

### Classic Scraping

```rust
use scrapio_classic::Scraper;
use scrapio_runtime::{Runtime, TokioRuntime};

fn main() {
    let runtime = TokioRuntime::default();
    runtime.block_on(async {
        let scraper = Scraper::new();
        match scraper.scrape("https://example.com").await {
            Ok(response) => {
                println!("Title: {}", response.title().unwrap_or_default());
                println!("Links: {}", response.links().len());

                // Select elements with CSS
                for element in response.select("h1") {
                    println!("H1: {}", element.inner_html());
                }
            }
            Err(e) => eprintln!("Error: {}", e),
        }
    });
}
```

### AI-Powered Scraping

```rust
use scrapio_ai::{AiScraper, AiConfig};
use scrapio_runtime::{Runtime, TokioRuntime};

fn main() {
    let runtime = TokioRuntime::default();
    runtime.block_on(async {
        let config = AiConfig::new()
            .with_provider("openai")
            .with_api_key("your-api-key");

        let scraper = AiScraper::with_config(config);

        let schema = r#"{
            "title": "string",
            "description": "string",
            "links": "array"
        }"#;

        match scraper.scrape("https://example.com", schema).await {
            Ok(result) => {
                println!("Model: {}", result.model);
                println!("Data: {}", serde_json::to_string_pretty(&result.data).unwrap());
            }
            Err(e) => eprintln!("Error: {}", e),
        }
    });
}
```

### Spider System

```rust
use scrapio_classic::spider::{Spider, SpiderRunner, SpiderOutput, Request, Item};
use scrapio_classic::Scraper;

struct MySpider {
    scraper: Scraper,
}

impl Spider for MySpider {
    fn name(&self) -> &str {
        "my_spider"
    }

    fn start_urls(&self) -> Vec<String> {
        vec!["https://example.com".to_string()]
    }

    fn parse(&self, response: &scrapio_classic::Response) -> SpiderOutput {
        // Extract items
        let mut item = Item::new();
        item.insert("title".to_string(), serde_json::json!(response.title()));

        // Extract links to follow
        let requests: Vec<Request> = response.links()
            .iter()
            .filter(|l| l.starts_with("https://example.com"))
            .map(|l| Request::get(l))
            .collect();

        SpiderOutput::Both(vec![item], requests)
    }
}

fn main() {
    let runtime = scrapio_runtime::TokioRuntime::default();
    runtime.block_on(async {
        let runner = SpiderRunner::new();
        let spider = MySpider { scraper: Scraper::new() };
        let items = runner.run(&spider).await;
        println!("Extracted {} items", items.len());
    });
}
```

### Storage

```rust
use scrapio_storage::Storage;
use scrapio_runtime::{Runtime, TokioRuntime};

fn main() {
    let runtime = TokioRuntime::default();
    runtime.block_on(async {
        let storage = Storage::new("scrapio.db").await.unwrap();

        // Save a result
        storage.save_result(
            "https://example.com",
            200,
            Some("Example Domain"),
            "<html>...</html>",
            &["https://example.com/link1".to_string()],
        ).await.unwrap();

        // List results
        let results = storage.get_all_results(10).await.unwrap();
        for r in results {
            println!("{} - {}", r.id, r.url);
        }
    });
}
```

## CLI Reference

```
scrapio --help

Commands:
  classic  Classic scraping using CSS selectors
  ai       AI-powered scraping with LLM extraction
  crawl    Crawl a website with configurable depth
  save     Save result to database
  list     List saved results
  serve    Start API server
  version  Show version info
```

### Options

- `--log-level` - Set logging level (default: info)

### Classic Command

```
scrapio classic <URL>
```

### AI Command

```
scrapio ai <URL> [OPTIONS]

Options:
  --schema <SCHEMA>      JSON schema for extraction
  --provider <PROVIDER>   LLM provider: openai, anthropic, ollama (default: openai)
```

### Crawl Command

```
scrapio crawl <URL> [OPTIONS]

Options:
  --depth <DEPTH>    Crawl depth (default: 2)
```

### Save Command

```
scrapio save <URL> [OPTIONS]

Options:
  --database <PATH>  SQLite database path (default: scrapio.db)
```

### List Command

```
scrapio list [OPTIONS]

Options:
  --database <PATH>  SQLite database path (default: scrapio.db)
  --limit <LIMIT>   Number of results to show (default: 10)
```

### Serve Command

```
scrapio serve [OPTIONS]

Options:
  --host <HOST>  Host to bind (default: 127.0.0.1)
  --port <PORT>  Port to bind (default: 8080)
```

## Environment Variables

- `OPENAI_API_KEY` - OpenAI API key for AI scraping
- `ANTHROPIC_API_KEY` - Anthropic API key for AI scraping

## Project Structure

```
scrapio/
├── crates/
│   ├── scrapio-core/       # Core types, error handling, HTTP
│   ├── scrapio-runtime/    # Runtime abstraction (tokio)
│   ├── scrapio-classic/    # Classic scraping, Spider, Pipeline
│   ├── scrapio-ai/         # AI-powered scraping
│   ├── scrapio-storage/    # SQLite storage
│   └── scrapio-cli/        # CLI binary
├── examples/               # Example programs
└── Cargo.toml
```

## Examples

See the [examples](examples/README.md) directory for usage examples:
- `classic` - CSS selector scraping
- `ai` - AI-powered scraping with Ollama
- `spider` - Custom spider implementation

## License

MIT - See [LICENSE](LICENSE) file for details.