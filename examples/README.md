# Examples for Scrapio

This directory contains examples showing how to use Scrapio as a library.

## Running Examples

```bash
cargo run --example classic
cargo run --example ai
cargo run --example spider
cargo run --example browser
cargo run --example browser_complex
cargo run --example ralph
```

Or run from the examples directory:
```bash
cd examples
cargo run -p scrapio-examples --bin classic
cargo run -p scrapio-examples --bin ai
cargo run -p scrapio-examples --bin spider
cargo run -p scrapio-examples --bin browser
cargo run -p scrapio-examples --bin browser_complex
cargo run -p scrapio-examples --bin ralph
```

## Examples

### classic
Basic CSS selector scraping using the scrapio-classic crate.

### ai
AI-powered scraping with Ollama (local LLM).

### spider
Custom spider implementation for crawling websites.

### browser
Browser automation with stealth mode using WebDriver. Uses ChromeDriverManager to automatically download and manage ChromeDriver.

### browser_complex
More advanced browser automation example demonstrating click, scroll, and screenshot interactions.

### ralph
Ralph loop example - iterates through multiple extraction targets until all are completed. Inspired by the Ralph agent pattern. Uses AI to navigate and extract different elements from the page in sequence.