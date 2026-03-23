//! Tests for scrapio-ai lib module

use scrapio_ai::{AiConfig, AiScraper, ExtractionMode, FallbackReason};

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

#[test]
fn test_extraction_mode_default() {
    let mode = ExtractionMode::default();
    assert_eq!(mode, ExtractionMode::Ai);
}

#[test]
fn test_extraction_mode_serialization() {
    let ai = serde_json::to_string(&ExtractionMode::Ai).unwrap();
    assert!(ai.contains("ai"));

    let fallback = serde_json::to_string(&ExtractionMode::Fallback).unwrap();
    assert!(fallback.contains("fallback"));
}

#[test]
fn test_fallback_reason_default() {
    let reason = FallbackReason::default();
    assert_eq!(reason, FallbackReason::Unknown);
}

#[test]
fn test_fallback_reason_variants() {
    let no_api_key = serde_json::to_string(&FallbackReason::NoApiKey).unwrap();
    assert!(no_api_key.contains("no_api_key"));

    let provider_err = serde_json::to_string(&FallbackReason::ProviderError).unwrap();
    assert!(provider_err.contains("provider_error"));

    let schema_err = serde_json::to_string(&FallbackReason::SchemaParseError).unwrap();
    assert!(schema_err.contains("schema_parse_error"));
}

#[test]
fn test_ai_extraction_result_fields() {
    use scrapio_ai::AiExtractionResult;

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

    assert_eq!(result.url, "https://example.com");
    assert!(result.data.get("title").is_some());
    assert_eq!(result.mode, ExtractionMode::Ai);
    assert_eq!(result.confidence, Some(0.95));
}
