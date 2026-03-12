//! LLM provider call functions — OpenAI, Anthropic, and Ollama

use scrapio_core::error::{ScrapioError, ScrapioResult};

use crate::config::AiConfig;
use crate::prompts;

/// Call OpenAI API
pub async fn call_openai(
    client: &reqwest::Client,
    config: &AiConfig,
    api_key: &str,
    content: &str,
    schema: &str,
) -> ScrapioResult<String> {
    let system = prompts::extraction_system_prompt();
    let user = prompts::extraction_user_prompt(content, schema);

    let body = serde_json::json!({
        "model": config.model,
        "messages": [
            {"role": "system", "content": system},
            {"role": "user", "content": user}
        ],
        "temperature": config.temperature,
        "max_tokens": config.max_tokens
    });

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

/// Call Anthropic API
pub async fn call_anthropic(
    client: &reqwest::Client,
    config: &AiConfig,
    api_key: &str,
    content: &str,
    schema: &str,
) -> ScrapioResult<String> {
    let system = prompts::extraction_system_prompt();
    let user = prompts::extraction_user_prompt(content, schema);

    let body = serde_json::json!({
        "model": config.model,
        "messages": [{"role": "user", "content": user}],
        "system": system,
        "temperature": config.temperature,
        "max_tokens": config.max_tokens
    });

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

/// Call Ollama API (local models)
pub async fn call_ollama(
    client: &reqwest::Client,
    config: &AiConfig,
    content: &str,
    schema: &str,
) -> ScrapioResult<String> {
    let url = format!(
        "{}/api/generate",
        config
            .ollama_url
            .as_deref()
            .unwrap_or("http://localhost:11434")
    );

    let ollama_model = match config.model.as_str() {
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
            "temperature": config.temperature,
            "num_predict": config.max_tokens
        }
    });

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
        return Err(ScrapioError::Ai(format!("Ollama error {}: {}", status, text)));
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
