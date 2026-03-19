# Scrapio

All-in-one web scraping toolkit with AI and non-AI capabilities.

## Features

- **Classic Scraping** - Rule-based scraping with CSS selectors
- **AI-Powered Scraping** - Intelligent content extraction using LLMs
- **Spider System** - Spider framework for defining crawl behavior
- **Item Pipelines** - Process and export scraped data (JSON, CSV)
- **SQLite Storage** - Persist crawl results
- **Multiple LLM Providers** - OpenAI, Anthropic, Ollama (local)
- **Browser Automation** - Stealth browser for JavaScript-rendered content

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
scrapio classic https://rust-lang.org
```

### AI-Powered Scraping

```bash
# Set your API key
export OPENAI_API_KEY=your-api-key

# AI scraping (uses fallback if no key)
scrapio ai https://rust-lang.org

# With custom schema
scrapio ai https://rust-lang.org --schema '{"title": "string", "links": "array"}'

# Use specific provider (openai, anthropic, ollama)
scrapio ai https://rust-lang.org --provider ollama

# Use browser automation for JavaScript-rendered pages
scrapio ai https://rust-lang.org --browser

# With browser automation and custom model
scrapio ai https://rust-lang.org --browser --model gpt-4o

# With custom prompt to guide the AI
scrapio ai https://rust-lang.org --browser --prompt "Find and extract all the installation commands for different operating systems"

# With custom schema for structured extraction
scrapio ai https://rust-lang.org --browser --schema '[{"id":"title","description":"Extract page title"}]'
```

#### AI Browser Mode

When using `--browser`, the AI uses the Ralph loop pattern to navigate and interact with pages:

1. Opens a headless browser and navigates to the URL
2. Analyzes the page content using the LLM
3. Decides on next actions (click, scroll, navigate, extract)
4. Repeats until the objective is complete or max steps reached

This is useful for:
- JavaScript-rendered pages
- Sites requiring interaction (login, scroll, click)
- Single-page applications
- Dynamic content that requires user interaction

The browser runs in stealth mode to avoid detection.

The Ralph loop:
- **With prompt only**: The prompt becomes the extraction target. The loop continues until the objective is achieved or max iterations reached.
- **With schema**: Iterates through each schema target until all are extracted.

```bash
# Ralph loop with prompt (no schema needed)
scrapio ai https://rust-lang.org --browser --prompt "get the main heading"

# Ralph loop with schema for multiple targets
scrapio ai https://rust-lang.org --browser --schema '[{"id":"title","description":"Get title"},{"id":"links","description":"Get all links"}]'

# Ralph loop with custom iterations and steps
scrapio ai https://rust-lang.org --browser --prompt "extract install commands" --max-steps 20
```

The Ralph loop is useful for:
- Complex multi-step tasks that require iteration
- When the objective is not achieved in a single pass
- Extracting multiple items from a page

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
scrapio ai https://rust-lang.org --provider ollama
```

### Selecting Models

You can specify which model to use with the `--model` flag:

```bash
# OpenAI models
scrapio ai https://rust-lang.org --provider openai --model gpt-4o
scrapio ai https://rust-lang.org --provider openai --model gpt-4
scrapio ai https://rust-lang.org --provider openai --model gpt-3.5-turbo

# Anthropic models
scrapio ai https://rust-lang.org --provider anthropic --model claude-sonnet-4-20250514
scrapio ai https://rust-lang.org --provider anthropic --model claude-3-opus-20240229
scrapio ai https://rust-lang.org --provider anthropic --model claude-3-haiku-20240307

# Ollama models (must be installed locally)
scrapio ai https://rust-lang.org --provider ollama --model llama3
scrapio ai https://rust-lang.org --provider ollama --model mistral
scrapio ai https://rust-lang.org --provider ollama --model phi3
scrapio ai https://rust-lang.org --provider ollama --model codellama
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
# Basic crawl with depth limit
scrapio crawl https://rust-lang.org --depth 3

# With scope control (host, domain, subdomain)
scrapio crawl https://rust-lang.org --depth 2 --scope domain

# With browser escalation for JS-heavy pages (never, auto, always)
scrapio crawl https://rust-lang.org --browser-escalation auto

# Discover URLs from sitemap.xml
scrapio crawl https://rust-lang.org --sitemap

# Discover paths from robots.txt
scrapio crawl https://rust-lang.org --robots

# Capture network requests (XHR/fetch) in browser mode
scrapio crawl https://rust-lang.org --network

# With AI extraction
scrapio crawl https://rust-lang.org --extract --schema '[{"field":"title"}]'

# Save results to SQLite
scrapio crawl https://rust-lang.org --store results.db
```

### Save to Database

```bash
# Save result to SQLite
scrapio save https://rust-lang.org --database scrapio.db

# List saved results
scrapio list --database scrapio.db
```

### Browser Automation

```bash
# Basic browser automation (requires ChromeDriver)
scrapio browser https://rust-lang.org --headless

# With stealth mode
scrapio browser https://rust-lang.org --stealth full

# Run with visible browser
scrapio browser https://rust-lang.org --headless=false

# Execute custom JavaScript
scrapio browser https://rust-lang.org --script myscript.js
```

**Stealth Levels:**
- `basic` - Removes navigator.webdriver flag
- `advanced` - Canvas fingerprint randomization, WebGL spoofing
- `full` - Viewport randomization, timezone/locale settings

**Note:** ChromeDriver is automatically downloaded and managed by the `ChromeDriverManager`. The browser command will automatically download the correct ChromeDriver version for your Chrome browser.

For programmatic usage, you can also use ChromeDriverManager directly:

```rust
use scrapio_browser::{ChromeDriverManager, ChromeDriverChannel};
use scrapio_runtime::{Runtime, TokioRuntime};

fn main() {
    let runtime = TokioRuntime::default();
    runtime.block_on(async {
        let mut manager = ChromeDriverManager::new()
            .with_channel(ChromeDriverChannel::Stable);

        // Or specify a version manually
        // let mut manager = ChromeDriverManager::new()
        //     .with_version("146.0.7680.72");

        // Download and start ChromeDriver automatically
        match manager.download_and_start(9515).await {
            Ok(_child) => println!("ChromeDriver started on port 9515"),
            Err(e) => eprintln!("Failed: {}", e),
        }
    });
}
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
        match scraper.scrape("https://rust-lang.org").await {
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

        match scraper.scrape("https://rust-lang.org", schema).await {
            Ok(result) => {
                println!("Model: {}", result.model);
                println!("Data: {}", serde_json::to_string_pretty(&result.data).unwrap());
            }
            Err(e) => eprintln!("Error: {}", e),
        }
    });
}
```

#### AI Browser Scraping (for JavaScript-rendered pages)

```rust
use scrapio_ai::{BrowserAiScraper, AiConfig};
use scrapio_runtime::{Runtime, TokioRuntime};

fn main() {
    let runtime = TokioRuntime::default();
    runtime.block_on(async {
        let config = AiConfig::new()
            .with_provider("openai")
            .with_api_key("your-api-key");

        let scraper = BrowserAiScraper::with_config(config);

        let schema = r#"{
            "title": "string",
            "description": "string",
            "install_commands": "array"
        }"#;

        // AI will navigate, click, scroll as needed to extract data
        match scraper.scrape("https://rust-lang.org/install.html", schema).await {
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
        vec!["https://rust-lang.org".to_string()]
    }

    fn parse(&self, response: &scrapio_classic::Response) -> SpiderOutput {
        // Extract items
        let mut item = Item::new();
        item.insert("title".to_string(), serde_json::json!(response.title()));

        // Extract links to follow
        let requests: Vec<Request> = response.links()
            .iter()
            .filter(|l| l.starts_with("https://rust-lang.org"))
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
            "https://rust-lang.org",
            200,
            Some("Example Domain"),
            "<html>...</html>",
            &["https://rust-lang.org/link1".to_string()],
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
  browser  Browser automation with stealth mode
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
  --schema <SCHEMA>       JSON schema for extraction
  --provider <PROVIDER>   LLM provider: openai, anthropic, ollama (default: openai)
  --browser               Use browser automation for JavaScript-rendered pages (uses Ralph loop)
  --prompt <PROMPT>       Custom prompt/objective for the AI
  --max-steps <STEPS>     Max steps for browser automation (default: 10)
  --headless <HEADLESS>  Run browser headless (default: true)
  -v, --verbose           Show step-by-step progress during browser automation
```

### Crawl Command

```
scrapio crawl <URL> [OPTIONS]

Options:
  --depth <DEPTH>                   Crawl depth (default: 2)
  --max-pages <MAX_PAGES>           Maximum pages to crawl
  --scope <SCOPE>                    Scope mode: host, domain, subdomain
  --browser-escalation <MODE>       Browser escalation: never, auto, always (default: auto)
  --sitemap                         Discover URLs from sitemap.xml
  --robots                          Discover paths from robots.txt
  --network                         Capture network requests in browser mode
  --extract                         Enable AI extraction
  --schema <SCHEMA>                  JSON schema for extraction
  --store <PATH>                    Save results to SQLite database
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

### Browser Command

```
scrapio browser <URL> [OPTIONS]

Options:
  --headless         Run in headless mode (default: true)
  --stealth <LEVEL>  Stealth level: basic, advanced, full
  --script <PATH>    JavaScript file to execute
```

### User Agent Management

Scrapio provides a `UserAgentManager` for configuring browser user agents:

```rust
use scrapio_core::{UserAgentManager, Browser, profiles};

let ua = UserAgentManager::new()
    .with_browser(Browser::Chrome)
    .with_version("122.0.0.0");

// Or use predefined profiles
let ua = profiles::chrome_desktop();
let iphone_ua = profiles::iphone();
let android_ua = profiles::android();
```

## Environment Variables

- `OPENAI_API_KEY` - OpenAI API key for AI scraping
- `ANTHROPIC_API_KEY` - Anthropic API key for AI scraping

## Project Structure

```
scrapio/
├── crates/
│   ├── scrapio-core/       # Core types, error handling, HTTP, UserAgent
│   ├── scrapio-runtime/    # Runtime abstraction (tokio)
│   ├── scrapio-classic/    # Classic scraping, Spider, Pipeline
│   ├── scrapio-ai/         # AI-powered scraping
│   ├── scrapio-storage/    # SQLite storage
│   ├── scrapio-browser/    # Browser automation with stealth
│   └── scrapio-cli/        # CLI binary
├── examples/               # Example programs
└── Cargo.toml
```

## Examples

See the [examples](examples/README.md) directory for usage examples:
- `classic` - CSS selector scraping
- `ai` - AI-powered scraping with Ollama
- `spider` - Custom spider implementation
- `browser` - Browser automation with stealth mode

## License

MIT - See [LICENSE](LICENSE) file for details.