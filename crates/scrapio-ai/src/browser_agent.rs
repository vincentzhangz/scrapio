//! Browser AI Scraper - Agentic browser-based AI scraping
//!
//! This module provides AI-powered scraping that uses a real browser to navigate
//! and interact with pages. The AI analyzes page content and decides what actions
//! to take to achieve the user's extraction goal.

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[cfg(feature = "browser")]
use scrapio_browser::{StealthBrowser, StealthConfig, StealthLevel};

#[cfg(feature = "browser")]
use scrapio_core::error::ScrapioError;

use super::config::AiConfig;
use super::extraction as ext;
use super::provider;

/// Maximum number of agentic loops before stopping
const DEFAULT_MAX_STEPS: usize = 10;

/// Browser action that the AI can request
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum BrowserAction {
    /// Navigate to a URL
    Goto { url: String },
    /// Click an element by CSS selector
    Click { selector: String },
    /// Scroll the page
    Scroll { pixels: i32 },
    /// Scroll to bottom of page
    ScrollToBottom,
    /// Wait for some time (ms)
    Wait { duration_ms: u64 },
    /// Extract content from current page
    Extract,
    /// Take a screenshot (returns as base64)
    Screenshot,
    /// Find elements matching selector
    FindElements { selector: String },
    /// Execute custom JavaScript
    ExecuteScript { script: String },
}

/// Result of executing a browser action
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum ActionResult {
    Success {
        #[serde(skip_serializing_if = "Option::is_none")]
        data: Option<Value>,
        #[serde(skip_serializing_if = "Option::is_none")]
        message: Option<String>,
    },
    Error {
        message: String,
    },
    Done {
        data: Value,
    },
}

/// State of the browser agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentState {
    pub current_url: String,
    pub page_title: String,
    pub html_content: String,
    pub action_history: Vec<String>,
    pub extracted_data: Vec<Value>,
}

impl AgentState {
    pub fn new() -> Self {
        Self {
            current_url: String::new(),
            page_title: String::new(),
            html_content: String::new(),
            action_history: Vec::new(),
            extracted_data: Vec::new(),
        }
    }
}

impl Default for AgentState {
    fn default() -> Self {
        Self::new()
    }
}

/// Browser AI Scraper that uses an agentic loop with browser automation
#[cfg(feature = "browser")]
pub struct BrowserAiScraper {
    config: AiConfig,
    max_steps: usize,
}

#[cfg(feature = "browser")]
impl BrowserAiScraper {
    /// Create a new BrowserAiScraper
    pub fn new() -> Self {
        Self {
            config: AiConfig::new(),
            max_steps: DEFAULT_MAX_STEPS,
        }
    }

    /// Create with custom config
    pub fn with_config(config: AiConfig) -> Self {
        Self {
            config,
            max_steps: DEFAULT_MAX_STEPS,
        }
    }

    /// Set maximum number of agent steps
    pub fn with_max_steps(mut self, steps: usize) -> Self {
        self.max_steps = steps;
        self
    }

    /// Scrape using browser automation with AI-guided navigation
    pub async fn scrape(
        &self,
        url: &str,
        schema: &str,
    ) -> Result<super::AiExtractionResult, ScrapioError> {
        self.scrape_with_prompt(url, schema, "").await
    }

    /// Scrape with a custom prompt
    pub async fn scrape_with_prompt(
        &self,
        url: &str,
        schema: &str,
        prompt: &str,
    ) -> Result<super::AiExtractionResult, ScrapioError> {
        self.scrape_with_options(url, schema, false, None, prompt)
            .await
    }

    /// Scrape with additional options
    pub async fn scrape_with_options(
        &self,
        url: &str,
        schema: &str,
        include_markdown: bool,
        stealth_level: Option<StealthLevel>,
        custom_prompt: &str,
    ) -> Result<super::AiExtractionResult, ScrapioError> {
        let mut browser = self.create_browser(stealth_level);

        let result = self
            .run_agent_loop(&mut browser, url, schema, custom_prompt)
            .await;

        let _ = browser.close().await;

        result.map(|data| super::AiExtractionResult {
            url: url.to_string(),
            data,
            markdown: if include_markdown {
                Some(String::new())
            } else {
                None
            },
            links: Vec::new(),
            used_fallback: false,
            model: self.config.model.clone(),
        })
    }

    fn create_browser(&self, stealth_level: Option<StealthLevel>) -> StealthBrowser {
        let level = stealth_level.unwrap_or(StealthLevel::Basic);
        let config = StealthConfig::new(level);

        StealthBrowser::new()
            .headless(true)
            .stealth(config)
            .window_size(1920, 1080)
    }

    /// Run the agent loop - AI decides browser actions until goal is achieved
    async fn run_agent_loop(
        &self,
        browser: &mut StealthBrowser,
        initial_url: &str,
        schema: &str,
        custom_prompt: &str,
    ) -> Result<Value, ScrapioError> {
        let mut state = AgentState::new();
        let mut step = 0;

        // Navigate to initial URL
        browser.goto(initial_url).await?;
        state.current_url = initial_url.to_string();
        state.page_title = browser.title().await.unwrap_or_default();
        state.html_content = browser.html().await.unwrap_or_default();
        state.action_history.push(format!("goto: {}", initial_url));

        while step < self.max_steps {
            step += 1;
            tracing::info!("Agent step {}/{}", step, self.max_steps);

            // Get current page state
            let page_state = self.get_page_state(browser).await?;

            // Ask AI what action to take
            let action = self
                .decide_action(&page_state, schema, &state.action_history, custom_prompt)
                .await?;

            // Execute the action
            let result = self
                .execute_action(browser, &action, &mut state, schema, custom_prompt)
                .await?;

            // Check if done
            if let ActionResult::Done { data } = result {
                return Ok(data);
            }

            // Small delay between actions
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        }

        // Max steps reached, extract what we have
        Ok(serde_json::json!({
            "steps_taken": step,
            "url": state.current_url,
            "message": "Max steps reached"
        }))
    }

    async fn get_page_state(
        &self,
        browser: &mut StealthBrowser,
    ) -> Result<PageState, ScrapioError> {
        let url = browser.url().await.unwrap_or_default();
        let title = browser.title().await.unwrap_or_default();
        let html = browser.html().await.unwrap_or_default();

        Ok(PageState { url, title, html })
    }

    /// Ask AI to decide the next action based on current state
    async fn decide_action(
        &self,
        page_state: &PageState,
        schema: &str,
        action_history: &[String],
        custom_prompt: &str,
    ) -> Result<BrowserAction, ScrapioError> {
        // Build the objective/instruction part
        let custom_instruction = if custom_prompt.is_empty() {
            format!(
                "Your goal is to extract data from the webpage according to this schema: {}",
                schema
            )
        } else {
            format!(
                "Your goal is: {}\n\nAdditionally, extract data according to this schema: {}",
                custom_prompt, schema
            )
        };

        let prompt = format!(
            r#"You are a web scraping agent. {}

Current page state:
- URL: {}
- Title: {}

HTML preview:
{}

Action history:
{}

IMPORTANT: You MUST respond with ONLY a raw JSON object, no markdown code blocks, no explanations.

Valid actions (use these exact JSON formats):
{{"type": "goto", "url": "..."}}
{{"type": "click", "selector": "..."}}
{{"type": "scroll", "pixels": 500}}
{{"type": "scroll_to_bottom"}}
{{"type": "wait", "duration_ms": 1000}}
{{"type": "extract"}}
{{"type": "find_elements", "selector": "..."}}

Decide what to do next and respond with ONLY the JSON object."#,
            custom_instruction,
            page_state.url,
            page_state.title,
            &page_state.html[..page_state.html.len().min(3000)],
            action_history
                .iter()
                .rev()
                .take(5)
                .map(|s| s.as_str())
                .collect::<Vec<_>>()
                .join("\n")
        );

        let response = self.call_ai(&prompt).await?;

        // Clean up the response - remove markdown code blocks
        let cleaned = response
            .trim()
            .trim_start_matches("```json")
            .trim_start_matches("```")
            .trim_end_matches("```")
            .trim();

        // Parse the response as a BrowserAction
        let action: BrowserAction = serde_json::from_str(cleaned).unwrap_or_else(|_| {
            // Try to find JSON in the response
            if let Some(start) = cleaned.find('{') {
                if let Some(end) = cleaned.rfind('}') {
                    serde_json::from_str(&cleaned[start..=end]).unwrap_or(BrowserAction::Extract)
                } else {
                    BrowserAction::Extract
                }
            } else {
                // If no valid JSON found, default to extract
                tracing::warn!(
                    "Could not parse AI response as action, defaulting to extract: {}",
                    cleaned
                );
                BrowserAction::Extract
            }
        });

        Ok(action)
    }

    /// Execute an action and return the result
    async fn execute_action(
        &self,
        browser: &mut StealthBrowser,
        action: &BrowserAction,
        state: &mut AgentState,
        schema: &str,
        custom_prompt: &str,
    ) -> Result<ActionResult, ScrapioError> {
        match action {
            BrowserAction::Goto { url } => {
                browser.goto(url).await?;
                state.current_url = url.clone();
                state.page_title = browser.title().await.unwrap_or_default();
                state.html_content = browser.html().await.unwrap_or_default();
                state.action_history.push(format!("goto: {}", url));
                Ok(ActionResult::Success {
                    data: None,
                    message: Some(format!("Navigated to {}", url)),
                })
            }

            BrowserAction::Click { selector } => match browser.click(selector).await {
                Ok(_) => {
                    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                    state.current_url = browser.url().await.unwrap_or_default();
                    state.page_title = browser.title().await.unwrap_or_default();
                    state.html_content = browser.html().await.unwrap_or_default();
                    state.action_history.push(format!("click: {}", selector));
                    Ok(ActionResult::Success {
                        data: None,
                        message: Some(format!("Clicked {}", selector)),
                    })
                }
                Err(e) => Ok(ActionResult::Error {
                    message: format!("Click failed: {}", e),
                }),
            },

            BrowserAction::Scroll { pixels } => {
                browser.scroll(*pixels).await?;
                state.action_history.push(format!("scroll: {}", pixels));
                Ok(ActionResult::Success {
                    data: None,
                    message: Some(format!("Scrolled {} pixels", pixels)),
                })
            }

            BrowserAction::ScrollToBottom => {
                browser.scroll_to_bottom().await?;
                tokio::time::sleep(std::time::Duration::from_millis(800)).await;
                state.html_content = browser.html().await.unwrap_or_default();
                state.action_history.push("scroll_to_bottom".to_string());
                Ok(ActionResult::Success {
                    data: None,
                    message: Some("Scrolled to bottom".to_string()),
                })
            }

            BrowserAction::Wait { duration_ms } => {
                tokio::time::sleep(std::time::Duration::from_millis(*duration_ms)).await;
                state.action_history.push(format!("wait: {}", duration_ms));
                Ok(ActionResult::Success {
                    data: None,
                    message: Some(format!("Waited {}ms", duration_ms)),
                })
            }

            BrowserAction::Extract => {
                let html = &state.html_content;
                let text_content = ext::strip_html(html);

                // Build extraction prompt - use prompt as guidance if schema is empty
                let extraction_goal = if schema == "{}" || schema.is_empty() {
                    custom_prompt.to_string()
                } else {
                    format!("Extract data according to this schema: {}", schema)
                };

                let prompt = format!(
                    r#"[INST]You are a data extraction assistant. Your task is to extract information from the provided web page content.

TASK: {}

Page URL: {}
Page Title: {}

Page content (just text extracted from HTML):
{}

IMPORTANT: Respond ONLY with a JSON object containing the extracted data. Do NOT include any other text, explanations, or action commands. Just return the JSON.

Example of correct response: {{"title": "Rust Programming Language", "description": "A language empowering everyone..."}}

Now extract the data:[/INST]"#,
                    extraction_goal,
                    state.current_url,
                    state.page_title,
                    &text_content[..text_content.len().min(5000)]
                );

                let result = self.call_ai(&prompt).await?;
                tracing::info!("Extraction result: {}", result);

                // If the response contains action format, extract actual content
                let final_result = if result.contains("\"action\"") {
                    tracing::warn!("AI returned action format instead of data, using fallback");
                    serde_json::json!({
                        "title": state.page_title.clone(),
                        "url": state.current_url.clone(),
                        "note": "Fallback - AI did not return extracted data"
                    })
                } else {
                    // Clean up response
                    let cleaned = result
                        .trim()
                        .trim_start_matches("```json")
                        .trim_start_matches("```")
                        .trim_end_matches("```")
                        .trim();

                    serde_json::from_str(cleaned).unwrap_or_else(|_| {
                        // Try to find JSON in the response
                        if let Some(start) = cleaned.find('{') {
                            if let Some(end) = cleaned.rfind('}') {
                                serde_json::from_str(&cleaned[start..=end])
                                    .unwrap_or_else(|_| serde_json::json!({ "raw": cleaned }))
                            } else {
                                serde_json::json!({ "raw": cleaned })
                            }
                        } else {
                            serde_json::json!({ "raw": cleaned })
                        }
                    })
                };

                state.extracted_data.push(final_result.clone());
                Ok(ActionResult::Done { data: final_result })
            }

            BrowserAction::Screenshot => {
                let screenshot = browser.screenshot().await?;
                let base64 = base64_encode(&screenshot);
                state.action_history.push("screenshot".to_string());
                Ok(ActionResult::Success {
                    data: Some(serde_json::json!({ "screenshot": base64 })),
                    message: Some("Screenshot taken".to_string()),
                })
            }

            BrowserAction::FindElements { selector } => {
                let elements = browser.find_elements(selector).await?;
                let count = elements.len();
                state
                    .action_history
                    .push(format!("find_elements: {} ({} found)", selector, count));
                Ok(ActionResult::Success {
                    data: Some(serde_json::json!({ "count": count })),
                    message: Some(format!("Found {} elements", count)),
                })
            }

            BrowserAction::ExecuteScript { script } => {
                let result = browser.execute_script(script).await?;
                state.action_history.push("execute_script".to_string());
                Ok(ActionResult::Success {
                    data: Some(result),
                    message: Some("Script executed".to_string()),
                })
            }
        }
    }

    /// Call the AI provider
    async fn call_ai(&self, prompt: &str) -> Result<String, ScrapioError> {
        let client = reqwest::Client::new();

        match self.config.provider.as_str() {
            "openai" => {
                let api_key = self
                    .config
                    .api_key
                    .as_deref()
                    .ok_or_else(|| ScrapioError::Ai("API key not set".to_string()))?;
                provider::call_openai(
                    &client,
                    &self.config,
                    api_key,
                    prompt,
                    r#"{"action": {"type": "string"}}"#,
                )
                .await
            }
            "anthropic" => {
                let api_key = self
                    .config
                    .api_key
                    .as_deref()
                    .ok_or_else(|| ScrapioError::Ai("API key not set".to_string()))?;
                provider::call_anthropic(
                    &client,
                    &self.config,
                    api_key,
                    prompt,
                    r#"{"action": {"type": "string"}}"#,
                )
                .await
            }
            "ollama" => {
                provider::call_ollama(
                    &client,
                    &self.config,
                    prompt,
                    r#"{"action": {"type": "string"}}"#,
                )
                .await
            }
            _ => Err(ScrapioError::Ai(format!(
                "Unknown provider: {}",
                self.config.provider
            ))),
        }
    }
}

#[cfg(feature = "browser")]
impl Default for BrowserAiScraper {
    fn default() -> Self {
        Self::new()
    }
}

/// Current page state for AI analysis
#[derive(Debug, Clone)]
struct PageState {
    url: String,
    title: String,
    html: String,
}

/// Simple base64 encoding
fn base64_encode(data: &[u8]) -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::new();

    for chunk in data.chunks(3) {
        let b0 = chunk[0] as usize;
        let b1 = chunk.get(1).copied().unwrap_or(0) as usize;
        let b2 = chunk.get(2).copied().unwrap_or(0) as usize;

        result.push(CHARS[b0 >> 2] as char);
        result.push(CHARS[((b0 & 0x03) << 4) | (b1 >> 4)] as char);

        if chunk.len() > 1 {
            result.push(CHARS[((b1 & 0x0f) << 2) | (b2 >> 6)] as char);
        } else {
            result.push('=');
        }

        if chunk.len() > 2 {
            result.push(CHARS[b2 & 0x3f] as char);
        } else {
            result.push('=');
        }
    }

    result
}

/// Quick browser AI scraping function
#[cfg(feature = "browser")]
pub async fn quick_browser_scrape(
    url: &str,
    schema: &str,
) -> Result<super::AiExtractionResult, ScrapioError> {
    BrowserAiScraper::new().scrape(url, schema).await
}
