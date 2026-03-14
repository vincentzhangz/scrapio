//! AI-Powered Scraping for Scrapio
//!
//! Provides intelligent content extraction using Large Language Models.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use scrapio_core::{
    DEFAULT_TIMEOUT, DEFAULT_USER_AGENT,
    error::{ScrapioError, ScrapioResult},
};

pub mod config;
pub mod extraction;
pub mod prompts;
pub mod provider;

#[cfg(feature = "browser")]
pub mod browser_agent;

pub use config::AiConfig;

#[cfg(feature = "browser")]
pub use browser_agent::{ActionResult, AgentState, BrowserAction, BrowserAiScraper};

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

/// AI Scraper for intelligent content extraction
pub struct AiScraper {
    config: AiConfig,
    client: reqwest::Client,
}

fn build_client() -> reqwest::Client {
    reqwest::Client::builder()
        .user_agent(DEFAULT_USER_AGENT)
        .timeout(DEFAULT_TIMEOUT)
        .build()
        .expect("Failed to create HTTP client")
}

impl AiScraper {
    pub fn new() -> Self {
        Self {
            config: AiConfig::new(),
            client: build_client(),
        }
    }

    pub fn with_config(config: AiConfig) -> Self {
        Self {
            client: build_client(),
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

        let html = self.client.get(url).send().await?.text().await?;
        let text_content = extraction::strip_html(&html);

        let result = self
            .extract_with_ai(&text_content, schema, include_markdown, url)
            .await
            .unwrap_or_else(|_| extraction::fallback_extraction(&html, url));

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
        // Use fallback if API key is not set (except for Ollama which doesn't require one)
        if self.config.provider != "ollama" && self.config.api_key.is_none() {
            return Ok(extraction::fallback_extraction(content, url));
        }

        // Create provider and extract
        let llm_provider = provider::create_provider(&self.config);
        let response = llm_provider.extract(content, schema).await?;

        let data: Value = serde_json::from_str(&response)
            .unwrap_or_else(|_| serde_json::json!({ "raw": response }));

        let links = extraction::extract_links(content);

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

    pub fn config(&self) -> &AiConfig {
        &self.config
    }
}

impl Default for AiScraper {
    fn default() -> Self {
        Self::new()
    }
}

/// Convenience function for quick AI scraping
pub async fn quick_scrape(url: &str, schema: &str) -> ScrapioResult<AiExtractionResult> {
    AiScraper::new().scrape(url, schema).await
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
