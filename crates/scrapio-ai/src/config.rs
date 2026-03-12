//! Configuration for AI scrapers

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

    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }

    pub fn with_provider(mut self, provider: impl Into<String>) -> Self {
        self.provider = provider.into();
        self
    }

    pub fn with_api_key(mut self, key: impl Into<String>) -> Self {
        self.api_key = Some(key.into());
        self
    }

    pub fn with_ollama_url(mut self, url: impl Into<String>) -> Self {
        self.ollama_url = Some(url.into());
        self
    }
}
