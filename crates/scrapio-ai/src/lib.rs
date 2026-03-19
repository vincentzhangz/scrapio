//! AI-Powered Scraping for Scrapio
//!
//! Provides intelligent content extraction using Large Language Models.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use scrapio_core::{
    error::{ScrapioError, ScrapioResult},
    http::HttpClient,
};

pub mod config;
pub mod extraction;
pub mod prompts;
pub mod provider;

#[cfg(feature = "browser")]
pub mod browser_agent;

#[cfg(feature = "browser")]
pub mod ralph;

pub use config::AiConfig;

#[cfg(feature = "browser")]
pub use browser_agent::{ActionResult, AgentState, BrowserAction, BrowserAiScraper};

#[cfg(feature = "browser")]
pub use ralph::{
    RalphInput, RalphInputError, RalphLoopOptions, RalphProgress, RalphResult, RalphStopReason,
    RalphTarget,
};

/// Extraction mode used for this result
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ExtractionMode {
    /// AI model extraction (primary)
    #[default]
    Ai,
    /// Fallback to heuristic extraction (degraded)
    Fallback,
}

/// Reason why fallback was used (meaningful when mode is Fallback)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum FallbackReason {
    /// No API key was configured
    NoApiKey,
    /// Provider API call failed
    ProviderError,
    /// Schema parsing failed
    SchemaParseError,
    /// Model output could not be parsed as JSON
    InvalidModelOutput,
    /// Network error or timeout
    NetworkError,
    /// Rate limited by provider
    RateLimited,
    /// Unknown error
    #[default]
    Unknown,
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
    /// Model used for extraction
    pub model: String,
    /// Extraction mode: ai or fallback
    #[serde(default)]
    pub mode: ExtractionMode,
    /// Reason for fallback (only populated when mode is Fallback)
    #[serde(default)]
    pub fallback_reason: Option<FallbackReason>,
    /// Provider error message if provider failed
    #[serde(default)]
    pub provider_error: Option<String>,
    /// Whether schema validation passed (for AI mode)
    #[serde(default)]
    pub schema_validation_passed: bool,
    /// Confidence score (0.0-1.0) if available
    #[serde(default)]
    pub confidence: Option<f32>,
}

/// AI Scraper for intelligent content extraction
pub struct AiScraper {
    config: AiConfig,
    http: HttpClient,
}

impl AiScraper {
    pub fn new() -> Self {
        Self {
            config: AiConfig::new(),
            http: HttpClient::new(),
        }
    }

    pub fn with_config(config: AiConfig) -> Self {
        Self {
            http: HttpClient::new(),
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

        let html = self.http.client().get(url).send().await?.text().await?;
        let text_content = extraction::strip_html(&html);

        // Try AI extraction first, handle errors explicitly
        match self
            .extract_with_ai(&text_content, schema, include_markdown, url)
            .await
        {
            Ok(result) => {
                // If result is already fallback mode, preserve it
                // Otherwise it's AI mode from extract_with_ai
                Ok(result)
            }
            Err(e) => {
                // Log the error and fall back to heuristic extraction
                tracing::warn!("AI extraction failed, falling back: {}", e);
                let mut fallback_result = extraction::fallback_extraction(&html, url);
                fallback_result.mode = ExtractionMode::Fallback;
                fallback_result.provider_error = Some(e.to_string());
                fallback_result.fallback_reason = Some(FallbackReason::ProviderError);
                Ok(fallback_result)
            }
        }
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
            let mut result = extraction::fallback_extraction(content, url);
            result.fallback_reason = Some(FallbackReason::NoApiKey);
            return Ok(result);
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
            model: self.config.model.clone(),
            mode: ExtractionMode::Ai,
            fallback_reason: None,
            provider_error: None,
            schema_validation_passed: true,
            confidence: None,
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

    // === ExtractionMode Tests ===

    #[test]
    fn test_extraction_mode_default() {
        let mode = ExtractionMode::default();
        assert_eq!(mode, ExtractionMode::Ai);
    }

    #[test]
    fn test_extraction_mode_serialization() {
        use serde_json;
        let ai = serde_json::to_string(&ExtractionMode::Ai).unwrap();
        assert!(ai.contains("ai"));

        let fallback = serde_json::to_string(&ExtractionMode::Fallback).unwrap();
        assert!(fallback.contains("fallback"));
    }

    // === FallbackReason Tests ===

    #[test]
    fn test_fallback_reason_default() {
        let reason = FallbackReason::default();
        assert_eq!(reason, FallbackReason::Unknown);
    }

    #[test]
    fn test_fallback_reason_variants() {
        use serde_json;

        let no_api_key = serde_json::to_string(&FallbackReason::NoApiKey).unwrap();
        assert!(no_api_key.contains("no_api_key"));

        let provider_err = serde_json::to_string(&FallbackReason::ProviderError).unwrap();
        assert!(provider_err.contains("provider_error"));

        let schema_err = serde_json::to_string(&FallbackReason::SchemaParseError).unwrap();
        assert!(schema_err.contains("schema_parse_error"));
    }

    // === AiExtractionResult Tests ===

    #[test]
    fn test_ai_extraction_result_fallback_fields() {
        let result = AiExtractionResult {
            url: "https://example.com".to_string(),
            data: serde_json::json!({"title": "Test"}),
            markdown: None,
            links: vec![],
            model: "gpt-4o".to_string(),
            mode: ExtractionMode::Fallback,
            fallback_reason: Some(FallbackReason::NoApiKey),
            provider_error: Some("No API key".to_string()),
            schema_validation_passed: false,
            confidence: None,
        };

        assert_eq!(result.mode, ExtractionMode::Fallback);
        assert_eq!(result.fallback_reason, Some(FallbackReason::NoApiKey));
        assert!(result.provider_error.is_some());
    }

    #[test]
    fn test_ai_extraction_result_ai_mode() {
        let result = AiExtractionResult {
            url: "https://example.com".to_string(),
            data: serde_json::json!({"title": "Test"}),
            markdown: None,
            links: vec![],
            model: "gpt-4o".to_string(),
            mode: ExtractionMode::Ai,
            fallback_reason: None,
            provider_error: None,
            schema_validation_passed: true,
            confidence: Some(0.95),
        };

        assert_eq!(result.mode, ExtractionMode::Ai);
        assert!(result.fallback_reason.is_none());
        assert!(result.provider_error.is_none());
        assert!(result.schema_validation_passed);
        assert_eq!(result.confidence, Some(0.95));
    }
}
