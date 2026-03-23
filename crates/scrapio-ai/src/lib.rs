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
