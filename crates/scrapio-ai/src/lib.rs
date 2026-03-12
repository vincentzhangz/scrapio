//! AI-Powered Scraping for Scrapio
//!
//! Provides intelligent content extraction using Large Language Models.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use scrapio_core::{ScrapioResult, error::*};

/// Configuration for AI scraping
#[derive(Debug, Clone)]
pub struct AiConfig {
    /// Model to use (e.g., "gpt-4o", "claude-3-sonnet", "llama3")
    pub model: String,
    /// LLM provider: "openai", "anthropic", "ollama"
    pub provider: String,
    /// API key
    pub api_key: Option<String>,
    /// Ollama base URL (for local models)
    pub ollama_url: Option<String>,
    /// Temperature for LLM generation
    pub temperature: f32,
    /// Maximum tokens to generate
    pub max_tokens: usize,
}

impl Default for AiConfig {
    fn default() -> Self {
        Self {
            model: "gpt-4o".to_string(),
            provider: "openai".to_string(),
            api_key: std::env::var("OPENAI_API_KEY").ok(),
            ollama_url: Some("http://localhost:11434".to_string()),
            temperature: 0.3,
            max_tokens: 4096,
        }
    }
}

impl AiConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_model(mut self, model: &str) -> Self {
        self.model = model.to_string();
        self
    }

    pub fn with_provider(mut self, provider: &str) -> Self {
        self.provider = provider.to_string();
        self
    }

    pub fn with_api_key(mut self, key: &str) -> Self {
        self.api_key = Some(key.to_string());
        self
    }

    pub fn with_ollama_url(mut self, url: &str) -> Self {
        self.ollama_url = Some(url.to_string());
        self
    }
}

/// Result from AI extraction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiExtractionResult {
    /// The original URL
    pub url: String,
    /// Extracted content as JSON
    pub data: Value,
    /// Raw markdown if requested
    pub markdown: Option<String>,
    /// Links discovered by AI
    pub links: Vec<String>,
    /// Whether fallback was used
    pub used_fallback: bool,
    /// Model used for extraction
    pub model: String,
}

/// Prompt templates for AI extraction
pub mod prompts {
    pub fn extraction_system_prompt() -> &'static str {
        r#"You are an expert web scraper. Your task is to extract structured data from web page content.
        Analyze the HTML/text content and extract meaningful information based on the user's schema.
        Return valid JSON that matches the schema exactly."#
    }

    pub fn extraction_user_prompt(content: &str, schema: &str) -> String {
        format!(
            "Extract data from the following content using this JSON schema:\n\nSchema:\n{}\n\nContent:\n{}\n\nReturn ONLY valid JSON.",
            schema, content
        )
    }
}

/// AI Scraper for intelligent content extraction
pub struct AiScraper {
    config: AiConfig,
    client: reqwest::Client,
}

impl AiScraper {
    pub fn new() -> Self {
        Self {
            config: AiConfig::new(),
            client: reqwest::Client::builder()
                .user_agent("Mozilla/5.0 (compatible; Scrapio/0.1)")
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .expect("Failed to create HTTP client"),
        }
    }

    pub fn with_config(config: AiConfig) -> Self {
        Self {
            client: reqwest::Client::builder()
                .user_agent("Mozilla/5.0 (compatible; Scrapio/0.1)")
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .expect("Failed to create HTTP client"),
            config,
        }
    }

    /// Scrape a URL and extract content using AI
    pub async fn scrape(&self, url: &str, schema: &str) -> ScrapioResult<AiExtractionResult> {
        self.scrape_with_options(url, schema, false).await
    }

    /// Scrape with additional options
    pub async fn scrape_with_options(
        &self,
        url: &str,
        schema: &str,
        include_markdown: bool,
    ) -> ScrapioResult<AiExtractionResult> {
        if !scrapio_core::utils::url::is_valid(url) {
            return Err(ScrapioError::Parse(format!("Invalid URL: {}", url)));
        }

        let response = self.client.get(url).send().await?;
        let html = response.text().await?;

        let text_content = strip_html(&html);

        let result = self
            .extract_with_ai(&text_content, schema, include_markdown, url)
            .await
            .unwrap_or_else(|_| self.fallback_extraction(&html, url));

        Ok(result)
    }

    /// Extract content using LLM
    async fn extract_with_ai(
        &self,
        content: &str,
        schema: &str,
        include_markdown: bool,
        url: &str,
    ) -> ScrapioResult<AiExtractionResult> {
        // For Ollama, no API key is needed
        // For OpenAI and Anthropic, API key is required
        if self.config.provider != "ollama" && self.config.api_key.is_none() {
            // Return fallback if no API key for non-Ollama providers
            return Ok(self.fallback_extraction(content, url));
        }

        let response = match self.config.provider.as_str() {
            "openai" => {
                let api_key = self
                    .config
                    .api_key
                    .clone()
                    .ok_or_else(|| ScrapioError::Ai("API key not set".to_string()))?;
                self.call_openai(&api_key, content, schema).await
            }
            "anthropic" => {
                let api_key = self
                    .config
                    .api_key
                    .clone()
                    .ok_or_else(|| ScrapioError::Ai("API key not set".to_string()))?;
                self.call_anthropic(&api_key, content, schema).await
            }
            "ollama" => self.call_ollama(content, schema).await,
            _ => Err(ScrapioError::Ai(format!(
                "Unknown provider: {}",
                self.config.provider
            ))),
        }?;

        let data: Value = serde_json::from_str(&response)
            .unwrap_or_else(|_| serde_json::json!({ "raw": response }));

        let links = extract_links(content);

        Ok(AiExtractionResult {
            url: url.to_string(),
            data,
            markdown: if include_markdown {
                Some(content.to_string())
            } else {
                None
            },
            links,
            used_fallback: false,
            model: self.config.model.clone(),
        })
    }

    /// Call OpenAI API
    async fn call_openai(
        &self,
        api_key: &str,
        content: &str,
        schema: &str,
    ) -> ScrapioResult<String> {
        let system = prompts::extraction_system_prompt();
        let user = prompts::extraction_user_prompt(content, schema);

        let body = serde_json::json!({
            "model": self.config.model,
            "messages": [
                {"role": "system", "content": system},
                {"role": "user", "content": user}
            ],
            "temperature": self.config.temperature,
            "max_tokens": self.config.max_tokens
        });

        let response = self
            .client
            .post("https://api.openai.com/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?
            .json::<serde_json::Value>()
            .await?;

        let text = response["choices"][0]["message"]["content"]
            .as_str()
            .ok_or_else(|| ScrapioError::Ai("Invalid response from OpenAI".to_string()))?
            .to_string();

        Ok(text)
    }

    /// Call Anthropic API
    async fn call_anthropic(
        &self,
        api_key: &str,
        content: &str,
        schema: &str,
    ) -> ScrapioResult<String> {
        let system = prompts::extraction_system_prompt();
        let user = prompts::extraction_user_prompt(content, schema);

        let body = serde_json::json!({
            "model": self.config.model,
            "messages": [{"role": "user", "content": user}],
            "system": system,
            "temperature": self.config.temperature,
            "max_tokens": self.config.max_tokens
        });

        let response = self
            .client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", api_key)
            .header("anthropic-version", "2023-06-01")
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?
            .json::<serde_json::Value>()
            .await?;

        let text = response["content"][0]["text"]
            .as_str()
            .ok_or_else(|| ScrapioError::Ai("Invalid response from Anthropic".to_string()))?
            .to_string();

        Ok(text)
    }

    /// Call Ollama API (local models)
    /// Call Ollama API (local models)
    async fn call_ollama(&self, content: &str, schema: &str) -> ScrapioResult<String> {
        let url = format!(
            "{}/api/generate",
            self.config
                .ollama_url
                .as_deref()
                .unwrap_or("http://localhost:11434")
        );

        let ollama_model = match self.config.model.as_str() {
            "gpt-4o" | "gpt-4" | "claude-3" | "" => "llama3",
            m => m,
        };

        let prompt = format!(
            "{}\n\n{}",
            prompts::extraction_system_prompt(),
            prompts::extraction_user_prompt(content, schema)
        );

        let body = serde_json::json!({
            "model": ollama_model,
            "prompt": prompt,
            "stream": false,
            "options": {
                "temperature": self.config.temperature,
                "num_predict": self.config.max_tokens
            }
        });

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(60))
            .build()
            .map_err(|e| ScrapioError::Ai(format!("Client error: {}", e)))?;

        let response = client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| ScrapioError::Ai(format!("Connection error: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(ScrapioError::Ai(format!(
                "Ollama error {}: {}",
                status, text
            )));
        }

        let value: serde_json::Value = response
            .json()
            .await
            .map_err(|e| ScrapioError::Ai(format!("Parse error: {}", e)))?;

        let text = value["response"]
            .as_str()
            .ok_or_else(|| ScrapioError::Ai("Invalid response from Ollama".to_string()))?
            .to_string();

        Ok(text)
    }

    /// Fallback extraction without AI
    fn fallback_extraction(&self, content: &str, url: &str) -> AiExtractionResult {
        use scraper::{Html, Selector};

        // Use parse_fragment which works with both full HTML and partial content
        let document = Html::parse_fragment(content);

        // Extract title
        let title = Selector::parse("title")
            .ok()
            .and_then(|s| document.select(&s).next())
            .map(|el| el.inner_html().trim().to_string());

        // Extract headings
        let headings: Vec<String> = Selector::parse("h1, h2, h3")
            .ok()
            .map(|s| {
                document
                    .select(&s)
                    .map(|el| el.inner_html().trim().to_string())
                    .collect()
            })
            .unwrap_or_default();

        // Extract links
        let links: Vec<String> = Selector::parse("a[href]")
            .ok()
            .map(|s| {
                document
                    .select(&s)
                    .filter_map(|el| el.value().attr("href").map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();

        // Extract meta description
        let description = Selector::parse("meta[name=description]")
            .ok()
            .and_then(|s| document.select(&s).next())
            .and_then(|el| el.value().attr("content"))
            .map(|s| s.to_string());

        // Extract main content
        let content = Selector::parse("main, article, .content, #content")
            .ok()
            .and_then(|s| document.select(&s).next())
            .map(|el| el.inner_html().trim().to_string());

        let data = serde_json::json!({
            "title": title,
            "description": description,
            "headings": headings,
            "content": content,
            "url": url,
        });

        AiExtractionResult {
            url: url.to_string(),
            data,
            markdown: None,
            links,
            used_fallback: true,
            model: "fallback".to_string(),
        }
    }

    pub fn config(&self) -> &AiConfig {
        &self.config
    }
}

impl Default for AiScraper {
    fn default() -> Self {
        Self::new()
    }
}

/// Strip HTML tags to get plain text
fn strip_html(html: &str) -> String {
    use scraper::{Html, Selector};
    let document = Html::parse_fragment(html);

    let main_content = Selector::parse("main, article, .content, #content, body")
        .ok()
        .and_then(|s| document.select(&s).next())
        .map(|el| el.inner_html());

    let content = main_content.unwrap_or_else(|| html.to_string());

    content
        .replace(&['<', '>'][..], " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Extract URLs from text using regex
fn extract_links(text: &str) -> Vec<String> {
    let url_regex = regex::Regex::new(r"https?://[^\s]+").unwrap();

    url_regex
        .find_iter(text)
        .map(|m| {
            m.as_str()
                .trim_end_matches(|c: char| {
                    !c.is_alphanumeric()
                        && c != ':'
                        && c != '/'
                        && c != '.'
                        && c != '-'
                        && c != '_'
                        && c != '~'
                })
                .to_string()
        })
        .collect()
}

/// Convenience function for quick AI scraping
pub async fn quick_scrape(url: &str, schema: &str) -> ScrapioResult<AiExtractionResult> {
    let scraper = AiScraper::new();
    scraper.scrape(url, schema).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ai_config_default() {
        let config = AiConfig::new();
        assert_eq!(config.model, "gpt-4o");
        assert_eq!(config.provider, "openai");
        assert_eq!(config.temperature, 0.3);
        assert_eq!(config.max_tokens, 4096);
    }

    #[test]
    fn test_ai_config_with_model() {
        let config = AiConfig::new().with_model("claude-3-opus");
        assert_eq!(config.model, "claude-3-opus");
    }

    #[test]
    fn test_ai_config_with_provider() {
        let config = AiConfig::new().with_provider("ollama");
        assert_eq!(config.provider, "ollama");
    }

    #[test]
    fn test_ai_config_with_api_key() {
        let config = AiConfig::new().with_api_key("test-key");
        assert_eq!(config.api_key, Some("test-key".to_string()));
    }

    #[test]
    fn test_ai_scraper_new() {
        let scraper = AiScraper::new();
        assert_eq!(scraper.config().model, "gpt-4o");
        assert_eq!(scraper.config().provider, "openai");
    }

    #[test]
    fn test_ai_scraper_with_config() {
        let config = AiConfig::new()
            .with_provider("anthropic")
            .with_model("claude-sonnet");
        let scraper = AiScraper::with_config(config);
        assert_eq!(scraper.config().provider, "anthropic");
        assert_eq!(scraper.config().model, "claude-sonnet");
    }
}
