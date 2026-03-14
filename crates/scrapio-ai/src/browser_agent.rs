//! Browser AI Scraper - Agentic browser-based AI scraping
//!
//! This module provides AI-powered scraping that uses a real browser to navigate
//! and interact with pages. The AI analyzes page content and decides what actions
//! to take to achieve the user's extraction goal.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fmt;

#[cfg(feature = "browser")]
use scrapio_browser::{
    ChromeDriverManager, ChromeDriverSession, StealthBrowser, StealthConfig, StealthLevel,
};

#[cfg(feature = "browser")]
use scrapio_core::error::ScrapioError;

use super::config::AiConfig;
use super::extraction as ext;
use super::provider;

/// Maximum number of agentic loops before stopping
const DEFAULT_MAX_STEPS: usize = 10;

/// Maximum number of retries for the same failed action
const MAX_ACTION_RETRIES: usize = 3;

/// Options for webdriver-based scraping
pub struct WebdriverScrapeOptions<'a> {
    pub url: &'a str,
    pub schema: &'a str,
    pub include_markdown: bool,
    pub stealth_level: Option<StealthLevel>,
    pub custom_prompt: &'a str,
    pub webdriver_url: String,
    pub headless: bool,
}

/// Browser action that the AI can request
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum BrowserAction {
    /// Navigate to a URL
    Goto { url: String },
    /// Click an element by CSS selector (legacy, prefer ClickElement)
    Click { selector: String },
    /// Click an element by grounded element ID
    ClickElement { element_id: String },
    /// Type text into an element by grounded element ID
    TypeInto { element_id: String, text: String },
    /// Scroll the page
    Scroll { pixels: i32 },
    /// Scroll to bottom of page
    ScrollToBottom,
    /// Wait for some time (ms)
    Wait { duration_ms: u64 },
    /// Extract partial data and continue (non-terminal)
    ExtractPartial,
    /// Extract and finish (terminal action)
    Extract,
    /// Finish and return current state
    Finish,
    /// Take a screenshot (returns as base64)
    Screenshot,
    /// Find elements matching selector
    FindElements { selector: String },
    /// Execute custom JavaScript
    ExecuteScript { script: String },
}

impl BrowserAction {
    /// Check if this action requires page state refresh
    pub fn needs_refresh(&self) -> bool {
        matches!(
            self,
            BrowserAction::Goto { .. }
                | BrowserAction::Click { .. }
                | BrowserAction::ClickElement { .. }
                | BrowserAction::TypeInto { .. }
                | BrowserAction::Scroll { .. }
                | BrowserAction::ScrollToBottom
                | BrowserAction::Wait { .. }
                | BrowserAction::ExecuteScript { .. }
        )
    }

    /// Check if this is a terminal action
    pub fn is_terminal(&self) -> bool {
        matches!(self, BrowserAction::Extract | BrowserAction::Finish)
    }

    /// Get a string representation for action history
    pub fn to_history_string(&self) -> String {
        match self {
            BrowserAction::Goto { url } => format!("goto: {}", url),
            BrowserAction::Click { selector } => format!("click: {}", selector),
            BrowserAction::ClickElement { element_id } => format!("click_element: {}", element_id),
            BrowserAction::TypeInto { element_id, text } => {
                format!("type_into: {} ({} chars)", element_id, text.len())
            }
            BrowserAction::Scroll { pixels } => format!("scroll: {}", pixels),
            BrowserAction::ScrollToBottom => "scroll_to_bottom".to_string(),
            BrowserAction::Wait { duration_ms } => format!("wait: {}ms", duration_ms),
            BrowserAction::ExtractPartial => "extract_partial".to_string(),
            BrowserAction::Extract => "extract".to_string(),
            BrowserAction::Finish => "finish".to_string(),
            BrowserAction::Screenshot => "screenshot".to_string(),
            BrowserAction::FindElements { selector } => format!("find_elements: {}", selector),
            BrowserAction::ExecuteScript { script } => {
                let preview = if script.len() > 50 {
                    format!("{}...", &script[..50])
                } else {
                    script.clone()
                };
                format!("execute_script: {}", preview)
            }
        }
    }
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

/// Structured failure tracking for action feedback
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionFailure {
    pub action_type: String,
    pub target: String,
    pub error: String,
}

/// Reason for stopping the agent loop
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum StopReason {
    ObjectiveCompleted,
    ExtractionCompleted,
    StepBudgetExceeded,
    Stuck,
    NavigationFailed,
    ValidationFailed,
    Unknown,
}

impl fmt::Display for StopReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            StopReason::ObjectiveCompleted => "objective_completed",
            StopReason::ExtractionCompleted => "extraction_completed",
            StopReason::StepBudgetExceeded => "step_budget_exceeded",
            StopReason::Stuck => "stuck",
            StopReason::NavigationFailed => "navigation_failed",
            StopReason::ValidationFailed => "validation_failed",
            StopReason::Unknown => "unknown",
        };
        write!(f, "{}", s)
    }
}

/// Interactable element for grounded actions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InteractableElement {
    pub id: String,
    pub element_type: String,
    pub text: String,
    pub selector_hint: Option<String>,
    pub href: Option<String>,
    pub placeholder: Option<String>,
}

/// Page snapshot for AI planning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageSnapshot {
    pub url: String,
    pub title: String,
    pub visible_text_summary: String,
    pub elements: Vec<InteractableElement>,
    pub recent_failures: Vec<ActionFailure>,
}

impl PageSnapshot {
    pub fn from_html(url: &str, title: &str, html: &str, failures: &[ActionFailure]) -> Self {
        // Extract interactable elements from HTML
        let elements = extract_interactable_elements(html);

        // Create a text summary (first 3000 chars of stripped HTML)
        let text_summary = ext::strip_html(html);
        let visible_text_summary = text_summary.chars().take(3000).collect();

        Self {
            url: url.to_string(),
            title: title.to_string(),
            visible_text_summary,
            elements,
            recent_failures: failures.to_vec(),
        }
    }
}

/// Extract interactable elements from HTML
fn extract_interactable_elements(html: &str) -> Vec<InteractableElement> {
    let mut elements = Vec::new();
    let mut element_counter = 0;

    // Use regex to find common interactable elements
    // This is a simplified approach - in production you might use a proper HTML parser

    // Find links
    let link_re = regex::Regex::new(r#"<a[^>]*href="([^"]*)"[^>]*>([^<]*)</a>"#).ok();
    if let Some(re) = link_re {
        for cap in re.captures_iter(html) {
            if element_counter >= 20 {
                break;
            }
            let href = cap.get(1).map(|m| m.as_str().to_string());
            let text = cap
                .get(2)
                .map(|m| m.as_str().trim().to_string())
                .unwrap_or_default();
            if !text.is_empty() {
                element_counter += 1;
                elements.push(InteractableElement {
                    id: format!("e{}", element_counter),
                    element_type: "link".to_string(),
                    text: text.clone(),
                    selector_hint: Some(format!(
                        "a[href='{}']",
                        href.as_ref().unwrap_or(&String::new())
                    )),
                    href,
                    placeholder: None,
                });
            }
        }
    }

    // Find buttons
    let button_re = regex::Regex::new(r#"<button[^>]*>([^<]*)</button>"#).ok();
    if let Some(re) = button_re {
        for cap in re.captures_iter(html) {
            if element_counter >= 20 {
                break;
            }
            let text = cap
                .get(1)
                .map(|m| m.as_str().trim().to_string())
                .unwrap_or_default();
            if !text.is_empty() {
                element_counter += 1;
                elements.push(InteractableElement {
                    id: format!("e{}", element_counter),
                    element_type: "button".to_string(),
                    text,
                    selector_hint: None,
                    href: None,
                    placeholder: None,
                });
            }
        }
    }

    // Find inputs with type submit or button
    let input_re =
        regex::Regex::new(r#"<input[^>]*type="(button|submit)"[^>]*value="([^"]*)"[^>]*>"#).ok();
    if let Some(re) = input_re {
        for cap in re.captures_iter(html) {
            if element_counter >= 20 {
                break;
            }
            let value = cap
                .get(2)
                .map(|m| m.as_str().trim().to_string())
                .unwrap_or_default();
            if !value.is_empty() {
                element_counter += 1;
                elements.push(InteractableElement {
                    id: format!("e{}", element_counter),
                    element_type: "button".to_string(),
                    text: value,
                    selector_hint: None,
                    href: None,
                    placeholder: None,
                });
            }
        }
    }

    // Find inputs with placeholders
    let input_placeholder_re = regex::Regex::new(r#"<input[^>]*placeholder="([^"]*)"[^>]*>"#).ok();
    if let Some(re) = input_placeholder_re {
        for cap in re.captures_iter(html) {
            if element_counter >= 20 {
                break;
            }
            let placeholder = cap
                .get(1)
                .map(|m| m.as_str().trim().to_string())
                .unwrap_or_default();
            if !placeholder.is_empty() {
                element_counter += 1;
                elements.push(InteractableElement {
                    id: format!("e{}", element_counter),
                    element_type: "input".to_string(),
                    text: String::new(),
                    selector_hint: None,
                    href: None,
                    placeholder: Some(placeholder),
                });
            }
        }
    }

    elements
}

/// State of the browser agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentState {
    pub current_url: String,
    pub page_title: String,
    pub html_content: String,
    pub action_history: Vec<String>,
    pub extracted_data: Vec<Value>,
    pub failures: Vec<ActionFailure>,
    pub last_action: Option<String>,
    pub consecutive_failures: usize,
}

impl AgentState {
    pub fn new() -> Self {
        Self {
            current_url: String::new(),
            page_title: String::new(),
            html_content: String::new(),
            action_history: Vec::new(),
            extracted_data: Vec::new(),
            failures: Vec::new(),
            last_action: None,
            consecutive_failures: 0,
        }
    }

    /// Record a failure
    pub fn record_failure(&mut self, action_type: &str, target: &str, error: &str) {
        self.failures.push(ActionFailure {
            action_type: action_type.to_string(),
            target: target.to_string(),
            error: error.to_string(),
        });
        // Keep only last 5 failures
        if self.failures.len() > 5 {
            self.failures.remove(0);
        }
        self.consecutive_failures += 1;
    }

    /// Record a success
    pub fn record_success(&mut self) {
        self.consecutive_failures = 0;
    }

    /// Check if stuck (too many consecutive failures)
    pub fn is_stuck(&self) -> bool {
        self.consecutive_failures >= MAX_ACTION_RETRIES
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

    /// Scrape with a managed ChromeDriver lifecycle.
    pub async fn scrape_with_managed_browser(
        &self,
        url: &str,
        schema: &str,
        prompt: &str,
        driver_path: Option<&str>,
        headless: bool,
    ) -> Result<super::AiExtractionResult, ScrapioError> {
        let driver = if let Some(path) = driver_path {
            ChromeDriverSession::start_with(ChromeDriverManager::new().with_path(path.into()))
                .await
                .map_err(|e| {
                    ScrapioError::Browser(format!("Failed to start ChromeDriver: {}", e))
                })?
        } else {
            ChromeDriverSession::start().await.map_err(|e| {
                ScrapioError::Browser(format!("Failed to start ChromeDriver: {}", e))
            })?
        };

        let options = WebdriverScrapeOptions {
            url,
            schema,
            include_markdown: false,
            stealth_level: None,
            custom_prompt: prompt,
            webdriver_url: driver.webdriver_url(),
            headless,
        };
        self.scrape_with_webdriver(options).await
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
        let options = WebdriverScrapeOptions {
            url,
            schema,
            include_markdown,
            stealth_level,
            custom_prompt,
            webdriver_url: "http://localhost:9515".to_string(),
            headless: true,
        };
        self.scrape_with_webdriver(options).await
    }

    async fn scrape_with_webdriver(
        &self,
        options: WebdriverScrapeOptions<'_>,
    ) -> Result<super::AiExtractionResult, ScrapioError> {
        let mut browser = self.create_browser(
            options.stealth_level,
            &options.webdriver_url,
            options.headless,
        );

        let result = self
            .run_agent_loop(
                &mut browser,
                options.url,
                options.schema,
                options.custom_prompt,
            )
            .await;

        let _ = browser.close().await;

        result.map(|data| super::AiExtractionResult {
            url: options.url.to_string(),
            data,
            markdown: if options.include_markdown {
                Some(String::new())
            } else {
                None
            },
            links: Vec::new(),
            used_fallback: false,
            model: self.config.model.clone(),
        })
    }

    fn create_browser(
        &self,
        stealth_level: Option<StealthLevel>,
        webdriver_url: &str,
        headless: bool,
    ) -> StealthBrowser {
        let level = stealth_level.unwrap_or(StealthLevel::Basic);
        let config = StealthConfig::new(level);

        StealthBrowser::with_webdriver(webdriver_url)
            .headless(headless)
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
        let mut stop_reason = StopReason::Unknown;

        // Navigate to initial URL
        browser.goto(initial_url).await?;
        self.refresh_state(browser, &mut state).await?;
        state.action_history.push(format!("goto: {}", initial_url));

        while step < self.max_steps {
            step += 1;
            tracing::info!("Agent step {}/{}", step, self.max_steps);

            // Check for stuck condition
            if state.is_stuck() {
                stop_reason = StopReason::Stuck;
                tracing::warn!("Agent stuck - too many consecutive failures");
                break;
            }

            // Get current page snapshot
            let snapshot = self.get_page_snapshot(browser, &state).await?;

            // Ask AI to decide the next action
            let action = self
                .decide_action(&snapshot, schema, &state.action_history, custom_prompt)
                .await?;

            // Validate action before execution
            if let Some(validation_error) =
                self.validate_action(&action, &snapshot, &state.action_history)
            {
                tracing::warn!("Action validation failed: {}", validation_error);
                state.record_failure(&format!("{:?}", action), "", &validation_error);
                // Try extract as fallback
                let extract_action = BrowserAction::Extract;
                let result = self
                    .execute_action(browser, &extract_action, &mut state, schema, custom_prompt)
                    .await?;
                if let ActionResult::Done { data } = result {
                    return Ok(serde_json::json!({
                        "data": data,
                        "stop_reason": StopReason::ValidationFailed.to_string(),
                        "steps_taken": step,
                    }));
                }
                continue;
            }

            // Execute the action
            let result = self
                .execute_action(browser, &action, &mut state, schema, custom_prompt)
                .await?;

            match result {
                ActionResult::Success { data: _, message } => {
                    state.record_success();
                    if let Some(msg) = message {
                        tracing::info!("Action result: {}", msg);
                    }
                    // If action needs refresh, update state
                    if action.needs_refresh() {
                        self.refresh_state(browser, &mut state).await?;
                    }
                }
                ActionResult::Error { message } => {
                    tracing::warn!("Action error: {}", message);
                    state.record_failure(&action.to_history_string(), "", &message);
                }
                ActionResult::Done { data } => {
                    // Extract completed (terminal)
                    return Ok(serde_json::json!({
                        "data": data,
                        "stop_reason": StopReason::ExtractionCompleted.to_string(),
                        "steps_taken": step,
                    }));
                }
            }

            // Check if we should stop based on action type
            if action.is_terminal() && matches!(action, BrowserAction::Finish) {
                stop_reason = StopReason::ObjectiveCompleted;
                break;
            }

            // Small delay between actions
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        }

        // Max steps reached or stopped
        if stop_reason == StopReason::Unknown {
            stop_reason = StopReason::StepBudgetExceeded;
        }

        Ok(serde_json::json!({
            "steps_taken": step,
            "url": state.current_url,
            "stop_reason": stop_reason.to_string(),
            "message": "Agent loop completed",
            "extracted_data": state.extracted_data
        }))
    }

    /// Refresh browser state
    async fn refresh_state(
        &self,
        browser: &mut StealthBrowser,
        state: &mut AgentState,
    ) -> Result<(), ScrapioError> {
        state.current_url = browser.url().await.unwrap_or_default();
        state.page_title = browser.title().await.unwrap_or_default();
        state.html_content = browser.html().await.unwrap_or_default();
        Ok(())
    }

    /// Get page snapshot with interactable elements
    async fn get_page_snapshot(
        &self,
        browser: &mut StealthBrowser,
        state: &AgentState,
    ) -> Result<PageSnapshot, ScrapioError> {
        let url = browser.url().await.unwrap_or_default();
        let title = browser.title().await.unwrap_or_default();
        let html = browser.html().await.unwrap_or_default();

        Ok(PageSnapshot::from_html(
            &url,
            &title,
            &html,
            &state.failures,
        ))
    }

    /// Validate action before execution
    fn validate_action(
        &self,
        action: &BrowserAction,
        snapshot: &PageSnapshot,
        action_history: &[String],
    ) -> Option<String> {
        // Check for element_id validity for ClickElement and TypeInto
        match action {
            BrowserAction::ClickElement { element_id } => {
                // Check if element exists in snapshot
                let exists = snapshot.elements.iter().any(|e| e.id == *element_id);
                if !exists {
                    return Some(format!(
                        "Element '{}' not found in page snapshot",
                        element_id
                    ));
                }
            }
            BrowserAction::TypeInto { element_id, text } => {
                let exists = snapshot
                    .elements
                    .iter()
                    .any(|e| e.id == *element_id && e.element_type == "input");
                if !exists {
                    return Some(format!("Input element '{}' not found", element_id));
                }
                if text.is_empty() {
                    return Some("Cannot type empty text".to_string());
                }
            }
            BrowserAction::Wait { duration_ms } => {
                if *duration_ms > 30000 {
                    return Some("Wait duration exceeds maximum of 30 seconds".to_string());
                }
            }
            BrowserAction::Goto { url } => {
                // Check for same-page navigation without reason
                if url == &snapshot.url && action_history.len() > 1 {
                    return Some("Redundant navigation to current page".to_string());
                }
            }
            _ => {}
        }
        None
    }

    /// Ask AI to decide the next action based on current snapshot
    async fn decide_action(
        &self,
        snapshot: &PageSnapshot,
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

        // Build elements list for grounded actions
        let elements_json = serde_json::to_string(&snapshot.elements).unwrap_or_default();

        // Build recent failures if any
        let mut failures_info = String::new();
        if !snapshot.recent_failures.is_empty() {
            let failures_str: Vec<String> = snapshot
                .recent_failures
                .iter()
                .map(|f| format!("- {} on '{}': {}", f.action_type, f.target, f.error))
                .collect();
            failures_info = format!(
                "\n\nRecent failures (avoid repeating these actions):\n{}\n\nIMPORTANT: Previous actions failed. Do NOT repeat the same failing actions.",
                failures_str.join("\n")
            );
        }

        let prompt = format!(
            r#"You are a web scraping agent. {}

Current page state:
- URL: {}
- Title: {}

Available interactable elements on this page:
{}

Visible text content (first 2000 chars):
{}

Action history (recent):
{}{}

IMPORTANT: You MUST respond with ONLY a raw JSON object, no markdown code blocks, no explanations.

Valid actions (use these exact JSON formats - prefer ClickElement over Click):
{{"type": "goto", "url": "https://..."}}
{{"type": "click_element", "element_id": "e1"}}
{{"type": "click", "selector": "..."}}
{{"type": "type_into", "element_id": "e1", "text": "..."}}
{{"type": "scroll", "pixels": 500}}
{{"type": "scroll_to_bottom"}}
{{"type": "wait", "duration_ms": 1000}}
{{"type": "extract_partial"}} - extract data so far and continue
{{"type": "extract"}} - final extraction and stop
{{"type": "finish"}} - finish and return current state
{{"type": "screenshot"}}

Decide what to do next and respond with ONLY the JSON object."#,
            custom_instruction,
            snapshot.url,
            snapshot.title,
            elements_json,
            &snapshot.visible_text_summary[..snapshot.visible_text_summary.len().min(2000)],
            action_history
                .iter()
                .rev()
                .take(5)
                .map(|s| s.as_str())
                .collect::<Vec<_>>()
                .join("\n"),
            failures_info
        );

        let response = self.call_ai_for_action(&prompt).await?;

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
        state.last_action = Some(action.to_history_string());
        state.action_history.push(action.to_history_string());

        match action {
            BrowserAction::Goto { url } => {
                browser.goto(url).await?;
                self.refresh_state(browser, state).await?;
                Ok(ActionResult::Success {
                    data: None,
                    message: Some(format!("Navigated to {}", url)),
                })
            }

            BrowserAction::Click { selector } => match browser.click(selector).await {
                Ok(_) => {
                    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                    self.refresh_state(browser, state).await?;
                    Ok(ActionResult::Success {
                        data: None,
                        message: Some(format!("Clicked {}", selector)),
                    })
                }
                Err(e) => Ok(ActionResult::Error {
                    message: format!("Click failed: {}", e),
                }),
            },

            BrowserAction::ClickElement { element_id } => {
                // Find the element by ID from snapshot
                let html = &state.html_content;
                let selector = find_selector_for_element(html, element_id);

                match selector {
                    Some(sel) => match browser.click(&sel).await {
                        Ok(_) => {
                            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                            self.refresh_state(browser, state).await?;
                            Ok(ActionResult::Success {
                                data: None,
                                message: Some(format!("Clicked element {}", element_id)),
                            })
                        }
                        Err(e) => Ok(ActionResult::Error {
                            message: format!("ClickElement failed: {}", e),
                        }),
                    },
                    None => Ok(ActionResult::Error {
                        message: format!("Could not find selector for element {}", element_id),
                    }),
                }
            }

            BrowserAction::TypeInto { element_id, text } => {
                let html = &state.html_content;
                let selector = find_selector_for_element(html, element_id);

                match selector {
                    Some(sel) => {
                        // Use JavaScript to type into the element
                        let script = format!(
                            "document.querySelector('{}').value = '{}';",
                            sel.replace('\'', "\\'"),
                            text.replace('\'', "\\'")
                        );
                        match browser.execute_script(&script).await {
                            Ok(_) => {
                                self.refresh_state(browser, state).await?;
                                Ok(ActionResult::Success {
                                    data: None,
                                    message: Some(format!("Typed into element {}", element_id)),
                                })
                            }
                            Err(e) => Ok(ActionResult::Error {
                                message: format!("TypeInto failed: {}", e),
                            }),
                        }
                    }
                    None => Ok(ActionResult::Error {
                        message: format!("Could not find selector for element {}", element_id),
                    }),
                }
            }

            BrowserAction::Scroll { pixels } => {
                browser.scroll(*pixels).await?;
                state.html_content = browser.html().await.unwrap_or_default();
                Ok(ActionResult::Success {
                    data: None,
                    message: Some(format!("Scrolled {} pixels", pixels)),
                })
            }

            BrowserAction::ScrollToBottom => {
                browser.scroll_to_bottom().await?;
                tokio::time::sleep(std::time::Duration::from_millis(800)).await;
                self.refresh_state(browser, state).await?;
                Ok(ActionResult::Success {
                    data: None,
                    message: Some("Scrolled to bottom".to_string()),
                })
            }

            BrowserAction::Wait { duration_ms } => {
                tokio::time::sleep(std::time::Duration::from_millis(*duration_ms)).await;
                Ok(ActionResult::Success {
                    data: None,
                    message: Some(format!("Waited {}ms", duration_ms)),
                })
            }

            BrowserAction::ExtractPartial | BrowserAction::Extract => {
                let html = &state.html_content;
                let text_content = ext::strip_html(html);

                // Build extraction prompt
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

Page content (text extracted from HTML):
{}

IMPORTANT: Respond ONLY with a JSON object containing the extracted data. Do NOT include any other text, explanations, or action commands. Just return the JSON.

Example of correct response: {{"title": "Rust Programming Language", "description": "A language empowering everyone..."}}

Now extract the data:[/INST]"#,
                    extraction_goal,
                    state.current_url,
                    state.page_title,
                    &text_content[..text_content.len().min(5000)]
                );

                let result = self.call_ai_for_extraction(&prompt, schema).await?;
                tracing::info!("Extraction result: {}", result);

                // Parse the result
                let final_result = parse_extraction_result(&result, state);

                state.extracted_data.push(final_result.clone());

                // Extract is terminal, ExtractPartial is not
                if matches!(action, BrowserAction::Extract) {
                    Ok(ActionResult::Done { data: final_result })
                } else {
                    Ok(ActionResult::Success {
                        data: Some(final_result),
                        message: Some("Partial extraction completed".to_string()),
                    })
                }
            }

            BrowserAction::Finish => {
                // Return current state as result
                let data = serde_json::json!({
                    "url": state.current_url,
                    "title": state.page_title,
                    "extracted_data": state.extracted_data,
                    "action_count": state.action_history.len()
                });
                Ok(ActionResult::Done { data })
            }

            BrowserAction::Screenshot => {
                let screenshot = browser.screenshot().await?;
                let base64 = base64_encode(&screenshot);
                Ok(ActionResult::Success {
                    data: Some(serde_json::json!({ "screenshot": base64 })),
                    message: Some("Screenshot taken".to_string()),
                })
            }

            BrowserAction::FindElements { selector } => {
                let elements = browser.find_elements(selector).await?;
                let count = elements.len();
                Ok(ActionResult::Success {
                    data: Some(serde_json::json!({ "count": count })),
                    message: Some(format!("Found {} elements", count)),
                })
            }

            BrowserAction::ExecuteScript { script } => {
                let result = browser.execute_script(script).await?;
                state.html_content = browser.html().await.unwrap_or_default();
                Ok(ActionResult::Success {
                    data: Some(result),
                    message: Some("Script executed".to_string()),
                })
            }
        }
    }

    /// Call AI for action planning
    async fn call_ai_for_action(&self, prompt: &str) -> Result<String, ScrapioError> {
        // Use fallback if API key is not set (except for Ollama which doesn't require one)
        if self.config.provider != "ollama" && self.config.api_key.is_none() {
            return Err(ScrapioError::Ai(
                "API key not set. Set OPENAI_API_KEY or ANTHROPIC_API_KEY".to_string(),
            ));
        }

        let provider = provider::create_provider(&self.config);
        provider.extract(prompt, r#"{"type": "string"}"#).await
    }

    /// Call AI for data extraction
    async fn call_ai_for_extraction(
        &self,
        prompt: &str,
        schema: &str,
    ) -> Result<String, ScrapioError> {
        // Use fallback if API key is not set (except for Ollama which doesn't require one)
        if self.config.provider != "ollama" && self.config.api_key.is_none() {
            return Err(ScrapioError::Ai(
                "API key not set. Set OPENAI_API_KEY or ANTHROPIC_API_KEY".to_string(),
            ));
        }

        // Use schema as the output format hint for extraction
        let schema_hint = if schema.is_empty() || schema == "{}" {
            r#"{"type": "object"}"#.to_string()
        } else {
            schema.to_string()
        };

        let provider = provider::create_provider(&self.config);
        provider.extract(prompt, &schema_hint).await
    }
}

#[cfg(feature = "browser")]
impl Default for BrowserAiScraper {
    fn default() -> Self {
        Self::new()
    }
}

/// Find CSS selector for an element by its ID
fn find_selector_for_element(html: &str, element_id: &str) -> Option<String> {
    // Parse the element ID to get the number
    let id_num = element_id.trim_start_matches('e');
    if let Ok(_num) = id_num.parse::<usize>() {
        // Re-extract elements and find matching selector
        let elements = extract_interactable_elements(html);
        for el in elements {
            if el.id == element_id {
                // Return selector_hint if available
                if let Some(hint) = el.selector_hint {
                    return Some(hint);
                }
                // Otherwise, construct a selector based on element type
                return Some(match el.element_type.as_str() {
                    "link" => format!("a:contains('{}')", el.text),
                    "button" => format!("button:contains('{}')", el.text),
                    "input" => {
                        if let Some(ph) = el.placeholder {
                            format!("input[placeholder='{}']", ph)
                        } else {
                            "input[type='text']".to_string()
                        }
                    }
                    _ => format!("{}:contains('{}')", el.element_type, el.text),
                });
            }
        }
    }
    None
}

/// Parse extraction result from AI response
fn parse_extraction_result(response: &str, state: &AgentState) -> Value {
    // If the response contains action format, extract actual content
    if response.contains("\"action\"") {
        tracing::warn!("AI returned action format instead of data, using fallback");
        serde_json::json!({
            "title": state.page_title.clone(),
            "url": state.current_url.clone(),
            "note": "Fallback - AI did not return extracted data"
        })
    } else {
        // Clean up response
        let cleaned = response
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
    }
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
