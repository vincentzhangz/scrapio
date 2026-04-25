//! LLM Provider implementations using rig-core
//!
//! Provides unified interface for OpenAI, Anthropic, OpenRouter, and Ollama.

use scrapio_core::error::{ScrapioError, ScrapioResult};

use crate::config::AiConfig;
use crate::prompts;

// Import CompletionClient trait for .agent() method
use rig::client::CompletionClient;
use rig::completion::Prompt;

/// Provider type enumeration - unified interface for different LLM providers
#[derive(Debug, Clone)]
pub enum LlmProvider {
    OpenAi(OpenAiProvider),
    Anthropic(AnthropicProvider),
    Ollama(OllamaProvider),
}

impl LlmProvider {
    /// Extract structured data from content using LLM
    pub async fn extract(&self, content: &str, schema: &str) -> ScrapioResult<String> {
        match self {
            LlmProvider::OpenAi(p) => p.extract(content, schema).await,
            LlmProvider::Anthropic(p) => p.extract(content, schema).await,
            LlmProvider::Ollama(p) => p.extract(content, schema).await,
        }
    }
}

/// Create a provider based on config
pub fn create_provider(config: &AiConfig) -> LlmProvider {
    match config.provider.as_str() {
        "openai" => LlmProvider::OpenAi(OpenAiProvider::new(config)),
        "anthropic" => LlmProvider::Anthropic(AnthropicProvider::new(config)),
        "ollama" => LlmProvider::Ollama(OllamaProvider::new(config)),
        _ => LlmProvider::OpenAi(OpenAiProvider::new(config)),
    }
}

// OpenAI/OpenRouter Provider implementation using rig
#[derive(Debug, Clone)]
pub struct OpenAiProvider {
    config: AiConfig,
}

impl OpenAiProvider {
    pub fn new(config: &AiConfig) -> Self {
        Self {
            config: config.clone(),
        }
    }

    pub async fn extract(&self, content: &str, schema: &str) -> ScrapioResult<String> {
        let api_key = self.config.api_key.as_deref().ok_or_else(|| {
            ScrapioError::Ai(
                "OpenAI API key not set. Set OPENAI_API_KEY or use --api-key".to_string(),
            )
        })?;

        let base_url = self.config.openai_url.as_deref();

        tracing::info!(provider = "openai", base_url = ?base_url, model = %self.config.model, "Connecting to AI service");

        // Build client with optional custom base URL (for OpenRouter, etc.)
        let openai_client = {
            let mut builder = rig::providers::openai::Client::builder().api_key(api_key);
            if let Some(url) = base_url {
                builder = builder.base_url(url);
            }
            builder
                .build()
                .map_err(|e| ScrapioError::Ai(format!("Failed to build OpenAI client: {}", e)))?
        };

        let agent = openai_client.agent(&self.config.model).build();

        let system = prompts::extraction_system_prompt();
        let user = prompts::extraction_user_prompt(content, schema);
        let full_prompt = format!("{}\n\n{}", system, user);

        agent
            .prompt(&full_prompt)
            .await
            .map_err(|e| ScrapioError::Ai(e.to_string()))
    }
}

// Anthropic Provider implementation using rig
#[derive(Debug, Clone)]
pub struct AnthropicProvider {
    config: AiConfig,
}

impl AnthropicProvider {
    pub fn new(config: &AiConfig) -> Self {
        Self {
            config: config.clone(),
        }
    }

    pub async fn extract(&self, content: &str, schema: &str) -> ScrapioResult<String> {
        let api_key = self.config.api_key.as_deref().ok_or_else(|| {
            ScrapioError::Ai(
                "Anthropic API key not set. Set ANTHROPIC_API_KEY or use --api-key".to_string(),
            )
        })?;

        tracing::info!(provider = "anthropic", model = %self.config.model, "Connecting to AI service");

        let client = rig::providers::anthropic::Client::builder()
            .api_key(api_key)
            .build()
            .map_err(|e| ScrapioError::Ai(format!("Failed to build Anthropic client: {}", e)))?
            .agent(&self.config.model)
            .build();

        let system = prompts::extraction_system_prompt();
        let user = prompts::extraction_user_prompt(content, schema);
        let full_prompt = format!("{}\n\n{}", system, user);

        client
            .prompt(&full_prompt)
            .await
            .map_err(|e| ScrapioError::Ai(e.to_string()))
    }
}

// Ollama Provider implementation using rig
#[derive(Debug, Clone)]
pub struct OllamaProvider {
    config: AiConfig,
}

impl OllamaProvider {
    pub fn new(config: &AiConfig) -> Self {
        Self {
            config: config.clone(),
        }
    }

    fn get_model(&self) -> String {
        match self.config.model.as_str() {
            "gpt-4o" | "gpt-4" | "claude-3" | "" => "llama3".to_string(),
            m => m.to_string(),
        }
    }

    pub async fn extract(&self, content: &str, schema: &str) -> ScrapioResult<String> {
        let base_url = self
            .config
            .ollama_url
            .as_deref()
            .unwrap_or("http://localhost:11434");

        tracing::info!(provider = "ollama", base_url = %base_url, model = %self.get_model(), "Connecting to AI service");

        // Ollama doesn't need an API key, use Nothing
        let ollama_client = rig::providers::ollama::Client::builder()
            .base_url(base_url)
            .api_key(rig::client::Nothing)
            .build()
            .map_err(|e| ScrapioError::Ai(format!("Failed to build Ollama client: {}", e)))?
            .agent(self.get_model())
            .build();

        let system = prompts::extraction_system_prompt();
        let user = prompts::extraction_user_prompt(content, schema);
        let full_prompt = format!("{}\n\n{}", system, user);

        ollama_client
            .prompt(&full_prompt)
            .await
            .map_err(|e| ScrapioError::Ai(e.to_string()))
    }
}
