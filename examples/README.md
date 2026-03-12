# Examples for Scrapio

This directory contains examples showing how to use Scrapio as a library.

## Running Examples

```bash
cargo run --example classic
cargo run --example ai
cargo run --example spider
cargo run --example browser
cargo run --example browser_complex
```

Or run from the examples directory:
```bash
cd examples
cargo run -p scrapio-examples --bin classic
cargo run -p scrapio-examples --bin ai
cargo run -p scrapio-examples --bin spider
cargo run -p scrapio-examples --bin browser
cargo run -p scrapio-examples --bin browser_complex
```

## Examples

### classic
Basic CSS selector scraping using the scrapio-classic crate.

### ai
AI-powered scraping with Ollama (local LLM).

### spider
Custom spider implementation for crawling websites.

### browser
Browser automation with stealth mode using WebDriver.

**Note:** The browser example requires ChromeDriver to be installed:

Download from [ChromeDriver Downloads](https://googlechromelabs.github.io/chrome-for-testing/#stable):

```bash
# macOS (manual)
# 1. Download ChromeDriver from https://googlechromelabs.github.io/chrome-for-testing/#stable
# 2. Extract and add to PATH or place in project directory

# Linux (Ubuntu/Debian)
sudo apt-get update
sudo apt-get install chromium-chromedriver

# Windows
# Download and add to PATH
```

Start ChromeDriver before running:
```bash
chromedriver --port=9515
```