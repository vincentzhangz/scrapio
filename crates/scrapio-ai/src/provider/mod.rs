//! LLM Provider implementations (clean enum-based abstraction)

use scrapio_core::error::{ScrapioError, ScrapioResult};

use crate::config::AiConfig;
use crate::prompts;

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

// OpenAI Provider implementation
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

        let client = reqwest::Client::new();
        let api_key = self.config.api_key.as_deref().ok_or_else(|| {
            ScrapioError::Ai("OpenAI API key not set. Set OPENAI_API_KEY".to_string())
        })?;

        let response = client
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
}

// Anthropic Provider implementation
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
        let system = prompts::extraction_system_prompt();
        let user = prompts::extraction_user_prompt(content, schema);

        let body = serde_json::json!({
            "model": self.config.model,
            "messages": [{"role": "user", "content": user}],
            "system": system,
            "temperature": self.config.temperature,
            "max_tokens": self.config.max_tokens
        });

        let client = reqwest::Client::new();
        let api_key = self.config.api_key.as_deref().ok_or_else(|| {
            ScrapioError::Ai("Anthropic API key not set. Set ANTHROPIC_API_KEY".to_string())
        })?;

        let response = client
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
}

// Ollama Provider implementation
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

    fn get_model(&self) -> &str {
        match self.config.model.as_str() {
            "gpt-4o" | "gpt-4" | "claude-3" | "" => "llama3",
            m => m,
        }
    }

    pub async fn extract(&self, content: &str, schema: &str) -> ScrapioResult<String> {
        let url = format!(
            "{}/api/generate",
            self.config
                .ollama_url
                .as_deref()
                .unwrap_or("http://localhost:11434")
        );

        let prompt = format!(
            "{}\n\n{}",
            prompts::extraction_system_prompt(),
            prompts::extraction_user_prompt(content, schema)
        );

        let body = serde_json::json!({
            "model": self.get_model(),
            "prompt": prompt,
            "stream": false,
            "options": {
                "temperature": self.config.temperature,
                "num_predict": self.config.max_tokens
            }
        });

        let client = reqwest::Client::new();
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
}
