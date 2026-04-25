//! Ralph Loop - Iterates through schema targets until all extracted
//!
//! This module provides the Ralph pattern implementation for browser automation.
//! It iterates through extraction targets until all are completed.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fmt;

use scrapio_browser::{StealthBrowser, StealthConfig, StealthLevel};
use scrapio_core::error::ScrapioError;

use crate::browser_agent::{ActionResult, AgentState, BrowserAction, BrowserAiScraper};

/// Maximum iterations for Ralph loop
pub const DEFAULT_MAX_ITERATIONS: usize = 50;

/// Maximum steps per iteration
pub const DEFAULT_MAX_STEPS: usize = 10;

/// Ralph input type - explicitly defines the schema input format
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum RalphInput {
    /// A natural language objective/prompt (legacy behavior)
    PromptObjective {
        /// The objective description
        objective: String,
    },
    /// A list of extraction targets
    TargetList {
        /// List of targets to extract
        targets: Vec<RalphTarget>,
    },
    /// A JSON Schema for extraction
    JsonExtractionSchema {
        /// The JSON schema
        schema: String,
    },
}

/// Error when parsing Ralph input
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RalphInputError {
    /// Schema is empty
    EmptySchema,
    /// Invalid JSON format
    InvalidJson(String),
    /// Unsupported schema format
    UnsupportedFormat(String),
    /// Cannot infer input type
    CannotInferType(String),
}

impl fmt::Display for RalphInputError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RalphInputError::EmptySchema => write!(f, "Schema is empty"),
            RalphInputError::InvalidJson(msg) => write!(f, "Invalid JSON: {}", msg),
            RalphInputError::UnsupportedFormat(msg) => write!(f, "Unsupported format: {}", msg),
            RalphInputError::CannotInferType(msg) => write!(f, "Cannot infer type: {}", msg),
        }
    }
}

/// Ralph loop configuration
pub struct RalphLoopOptions<'a> {
    pub url: &'a str,
    pub schema: &'a str,
    pub custom_prompt: &'a str,
    pub max_iterations: Option<usize>,
    pub max_steps_per_iteration: Option<usize>,
    pub stealth_level: Option<StealthLevel>,
    pub webdriver_url: Option<String>,
    pub browser_version: Option<String>,
    pub headless: bool,
    pub verbose: bool,
    /// Maximum characters of stripped HTML text to send to AI (None = no limit)
    pub text_limit: Option<usize>,
    /// Characters to skip from the beginning of stripped HTML (None = 0)
    pub text_offset: Option<usize>,
}

impl<'a> Default for RalphLoopOptions<'a> {
    fn default() -> Self {
        Self {
            url: "",
            schema: "[]",
            custom_prompt: "",
            max_iterations: None,
            max_steps_per_iteration: None,
            stealth_level: None,
            webdriver_url: None,
            browser_version: None,
            headless: true,
            verbose: false,
            text_limit: None,
            text_offset: None,
        }
    }
}

/// Extraction verification status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ExtractionStatus {
    /// Extraction not yet attempted
    #[default]
    Pending,
    /// Data was extracted but not verified
    PartialSuccess,
    /// Data was extracted and verified
    VerifiedSuccess,
    /// Extraction failed
    Failed,
}

/// A single extraction target in the Ralph loop
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RalphTarget {
    pub id: String,
    pub description: String,
    #[serde(default)]
    pub extracted: bool,
    pub data: Option<Value>,
    pub error: Option<String>,
    /// Verification status of the extraction
    #[serde(default)]
    pub status: ExtractionStatus,
    /// Optional validation rule for this target
    #[serde(default)]
    pub validation_rule: Option<String>,
}

impl RalphTarget {
    pub fn new(id: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            description: description.into(),
            extracted: false,
            data: None,
            error: None,
            status: ExtractionStatus::Pending,
            validation_rule: None,
        }
    }

    /// Validate the extracted data for this target
    pub fn validate(&self) -> bool {
        // Check if data exists
        let data = match &self.data {
            Some(d) => d,
            None => return false,
        };

        // If there's a validation rule, use it
        if let Some(rule) = &self.validation_rule {
            return self.apply_validation_rule(data, rule);
        }

        // Default validation: check for non-empty data
        match data {
            Value::String(s) => !s.trim().is_empty(),
            Value::Array(arr) => !arr.is_empty(),
            Value::Object(obj) => !obj.is_empty(),
            Value::Number(n) => n.as_i64().map(|v| v != 0).unwrap_or(true),
            Value::Bool(b) => *b,
            Value::Null => false,
        }
    }

    /// Apply a custom validation rule to the data
    fn apply_validation_rule(&self, data: &Value, rule: &str) -> bool {
        match rule {
            "non_empty" => match data {
                Value::String(s) => !s.trim().is_empty(),
                Value::Array(arr) => !arr.is_empty(),
                Value::Object(obj) => !obj.is_empty(),
                Value::Null => false,
                _ => true,
            },
            "required" => !data.is_null(),
            "not_empty_string" => data.as_str().map(|s| !s.trim().is_empty()).unwrap_or(false),
            r => {
                // Unknown rule, be permissive
                tracing::warn!("Unknown validation rule: {}", r);
                true
            }
        }
    }

    /// Mark this target as verified successfully
    pub fn mark_verified(&mut self, data: Value) {
        self.extracted = true;
        self.data = Some(data);
        self.error = None;
        self.status = ExtractionStatus::VerifiedSuccess;
    }

    /// Mark this target as partially successful (extracted but not verified)
    pub fn mark_partial(&mut self, data: Value) {
        self.extracted = true;
        self.data = Some(data);
        self.error = None;
        self.status = ExtractionStatus::PartialSuccess;
    }
}

/// Ralph input parsing - explicitly parses different input formats
impl RalphInput {
    /// Parse input with explicit type detection
    pub fn parse(schema: &str, custom_prompt: &str) -> Result<Self, RalphInputError> {
        // If custom prompt is provided, treat it as objective
        if !custom_prompt.is_empty() {
            return Ok(RalphInput::PromptObjective {
                objective: custom_prompt.to_string(),
            });
        }

        // Empty schema check
        if schema.is_empty() || schema == "[]" {
            return Err(RalphInputError::EmptySchema);
        }

        // Try parsing as explicit RalphInput format first
        if let Ok(input) = serde_json::from_str::<RalphInput>(schema) {
            return Ok(input);
        }

        // Try parsing as JSON array of targets (TargetList format)
        if let Ok(targets) = serde_json::from_str::<Vec<RalphTarget>>(schema) {
            return Ok(RalphInput::TargetList { targets });
        }

        // Try parsing as JSON object with items or properties
        let obj: Value = serde_json::from_str(schema)
            .map_err(|e| RalphInputError::InvalidJson(e.to_string()))?;

        // Check for items array
        if let Some(items) = obj.get("items").and_then(|v| v.as_array()) {
            let targets: Vec<RalphTarget> = items
                .iter()
                .enumerate()
                .map(|(i, item)| {
                    let id = item
                        .get("id")
                        .or_else(|| item.get("name"))
                        .and_then(|v| v.as_str())
                        .unwrap_or(&format!("item_{}", i))
                        .to_string();
                    let description = item
                        .get("description")
                        .or_else(|| item.get("title"))
                        .or_else(|| item.get("type"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("Extract data")
                        .to_string();
                    RalphTarget::new(id, description)
                })
                .collect();
            return Ok(RalphInput::TargetList { targets });
        }

        // Check for properties object
        if let Some(props) = obj.get("properties").and_then(|v| v.as_object()) {
            let targets: Vec<RalphTarget> = props
                .iter()
                .map(|(name, prop)| {
                    let description = prop
                        .get("description")
                        .or_else(|| prop.get("title"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("Extract data")
                        .to_string();
                    RalphTarget::new(name.clone(), description)
                })
                .collect();
            return Ok(RalphInput::TargetList { targets });
        }

        // If none of the above, treat as JSON schema
        Ok(RalphInput::JsonExtractionSchema {
            schema: schema.to_string(),
        })
    }

    /// Extract targets from this input
    pub fn to_targets(&self) -> Vec<RalphTarget> {
        match self {
            RalphInput::PromptObjective { objective } => {
                vec![RalphTarget::new("objective", objective.clone())]
            }
            RalphInput::TargetList { targets } => targets.clone(),
            RalphInput::JsonExtractionSchema { schema } => {
                // Extract targets from JSON schema properties
                #[allow(clippy::collapsible_if)]
                if let Ok(obj) = serde_json::from_str::<Value>(schema) {
                    if let Some(props) = obj.get("properties").and_then(|v| v.as_object()) {
                        return props
                            .iter()
                            .map(|(name, prop)| {
                                let description = prop
                                    .get("description")
                                    .or_else(|| prop.get("title"))
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("Extract data")
                                    .to_string();
                                RalphTarget::new(name.clone(), description)
                            })
                            .collect();
                    }
                }
                vec![RalphTarget::new("default", schema.clone())]
            }
        }
    }
}

/// Ralph loop progress
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RalphProgress {
    #[serde(default)]
    pub targets: Vec<RalphTarget>,
    #[serde(default)]
    pub iterations_completed: usize,
    #[serde(default)]
    pub steps_taken: usize,
    #[serde(default)]
    pub current_target: Option<String>,
    #[serde(default)]
    pub is_complete: bool,
}

impl RalphProgress {
    /// Check if all targets have been extracted (either verified or partial)
    pub fn all_extracted(&self) -> bool {
        !self.targets.is_empty() && self.targets.iter().all(|t| t.extracted)
    }

    /// Check if all targets have been successfully verified
    pub fn all_verified(&self) -> bool {
        !self.targets.is_empty()
            && self
                .targets
                .iter()
                .all(|t| t.status == ExtractionStatus::VerifiedSuccess)
    }

    /// Check if all targets have failed
    pub fn all_failed(&self) -> bool {
        !self.targets.is_empty()
            && self
                .targets
                .iter()
                .all(|t| t.status == ExtractionStatus::Failed)
    }

    pub fn next_pending_target(&self) -> Option<&RalphTarget> {
        self.targets.iter().find(|t| !t.extracted)
    }

    /// Mark a target as extracted and validate it
    pub fn mark_extracted(&mut self, target_id: &str, data: Value) {
        if let Some(target) = self.targets.iter_mut().find(|t| t.id == target_id) {
            target.data = Some(data.clone());
            target.error = None;

            // Validate the extracted data
            if target.validation_rule.is_some() || !data.is_null() {
                // If there's a validation rule, use it
                if target.validate() {
                    target.extracted = true;
                    target.status = ExtractionStatus::VerifiedSuccess;
                } else {
                    // Data exists but validation failed
                    target.extracted = true;
                    target.status = ExtractionStatus::PartialSuccess;
                    target.error =
                        Some("Validation failed: data did not meet requirements".to_string());
                }
            } else {
                target.extracted = true;
                target.status = ExtractionStatus::PartialSuccess;
            }
        }
    }

    /// Mark a target as failed
    pub fn mark_failed(&mut self, target_id: &str, error: &str) {
        if let Some(target) = self.targets.iter_mut().find(|t| t.id == target_id) {
            target.error = Some(error.to_string());
            target.status = ExtractionStatus::Failed;
        }
    }

    pub fn from_schema(schema: &str, custom_prompt: &str) -> Result<Self, ScrapioError> {
        // Use the new explicit input parser
        let input = RalphInput::parse(schema, custom_prompt)
            .map_err(|e| ScrapioError::Parse(e.to_string()))?;

        let targets = input.to_targets();

        if targets.is_empty() {
            return Err(ScrapioError::Parse(
                "No targets could be extracted from schema".to_string(),
            ));
        }

        Ok(Self {
            targets,
            iterations_completed: 0,
            steps_taken: 0,
            current_target: None,
            is_complete: false,
        })
    }
}

/// Stop reason for Ralph loop
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RalphStopReason {
    AllTargetsExtracted,
    MaxIterationsReached,
    NoMoreTargets,
    Error,
    Cancelled,
}

impl fmt::Display for RalphStopReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RalphStopReason::AllTargetsExtracted => write!(f, "all_targets_extracted"),
            RalphStopReason::MaxIterationsReached => write!(f, "max_iterations_reached"),
            RalphStopReason::NoMoreTargets => write!(f, "no_more_targets"),
            RalphStopReason::Error => write!(f, "error"),
            RalphStopReason::Cancelled => write!(f, "cancelled"),
        }
    }
}

/// Ralph loop result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RalphResult {
    pub progress: RalphProgress,
    pub stop_reason: RalphStopReason,
    #[serde(default)]
    pub data: Value,
    pub message: String,
}

impl BrowserAiScraper {
    /// Run Ralph loop - iterates through schema targets until all extracted
    pub async fn ralph_loop(
        &self,
        options: RalphLoopOptions<'_>,
    ) -> Result<RalphResult, ScrapioError> {
        use scrapio_browser::{ChromeDriverManager, ChromeDriverSession, StealthBrowser};

        let driver = if let Some(ref version) = options.browser_version {
            ChromeDriverSession::start_with(ChromeDriverManager::new().with_version(version))
                .await
                .map_err(|e| {
                    ScrapioError::Browser(format!("Failed to start ChromeDriver: {}", e))
                })?
        } else {
            ChromeDriverSession::start().await.map_err(|e| {
                ScrapioError::Browser(format!("Failed to start ChromeDriver: {}", e))
            })?
        };

        let webdriver_url = driver.webdriver_url();

        let stealth_level = options.stealth_level.unwrap_or(StealthLevel::Basic);
        let stealth_config = StealthConfig::new(stealth_level);

        let builder = StealthBrowser::with_webdriver(webdriver_url)
            .headless(options.headless)
            .stealth(stealth_config);

        let mut browser = builder
            .init()
            .await
            .map_err(|e| ScrapioError::Browser(e.to_string()))?;

        let result = self.run_ralph_loop(&mut browser, options).await;

        let _ = browser.close().await;

        result
    }

    /// Internal: Run Ralph loop with existing browser
    pub async fn run_ralph_loop(
        &self,
        browser: &mut StealthBrowser,
        options: RalphLoopOptions<'_>,
    ) -> Result<RalphResult, ScrapioError> {
        // Let RalphInput::parse handle all the format detection
        let mut progress = RalphProgress::from_schema(options.schema, options.custom_prompt)?;

        let max_iterations = options.max_iterations.unwrap_or(DEFAULT_MAX_ITERATIONS);
        let max_steps = options.max_steps_per_iteration.unwrap_or(DEFAULT_MAX_STEPS);
        let verbose = options.verbose;

        if verbose {
            println!("\n=== Ralph Loop Started ===");
            println!("URL: {}", options.url);
            println!("Targets: {}", progress.targets.len());
            println!("Max iterations: {}", max_iterations);
            println!("Max steps per iteration: {}\n", max_steps);
        }

        tracing::info!(
            "Starting Ralph loop with {} targets",
            progress.targets.len()
        );

        browser.goto(options.url).await?;
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;

        let mut iteration = 0;
        let mut total_steps = 0;
        let mut stop_reason = RalphStopReason::AllTargetsExtracted;

        while iteration < max_iterations {
            if progress.all_extracted() {
                stop_reason = RalphStopReason::AllTargetsExtracted;
                tracing::info!("All targets extracted, stopping Ralph loop");
                break;
            }

            let (target_id, target_description) = match progress.next_pending_target() {
                Some(t) => (t.id.clone(), t.description.clone()),
                None => {
                    stop_reason = RalphStopReason::NoMoreTargets;
                    break;
                }
            };

            iteration += 1;
            progress.current_target = Some(target_id.clone());

            if verbose {
                println!(
                    "\n[Iteration {}/{}] Target: {} - {}",
                    iteration, max_iterations, target_id, target_description
                );
            } else {
                tracing::info!(
                    "Ralph iteration {}/{}: extracting target '{}'",
                    iteration,
                    max_iterations,
                    target_id
                );
            }

            let target_schema = serde_json::json!({
                "type": "object",
                "properties": {
                    target_id.clone(): {
                        "type": "object",
                        "description": target_description.clone()
                    }
                }
            })
            .to_string();

            let step_result = self
                .run_ralph_iteration(
                    browser,
                    &target_schema,
                    options.custom_prompt,
                    max_steps,
                    options.verbose,
                )
                .await;

            match step_result {
                Ok(result) => {
                    total_steps += result
                        .get("steps_taken")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0) as usize;

                    if let Some(data) = result.get("data").or_else(|| result.get("extracted_data"))
                    {
                        progress.mark_extracted(&target_id, data.clone());
                        tracing::info!("Successfully extracted target: {}", target_id);
                    } else if let Some(data) = result
                        .get("extracted_data")
                        .and_then(|v| v.as_array())
                        .and_then(|arr| arr.last())
                    {
                        progress.mark_extracted(&target_id, data.clone());
                    } else {
                        let error_msg = result
                            .get("message")
                            .and_then(|v| v.as_str())
                            .unwrap_or("Unknown error");
                        progress.mark_failed(&target_id, error_msg);
                        tracing::warn!("Failed to extract target '{}': {}", target_id, error_msg);
                    }
                }
                Err(e) => {
                    progress.mark_failed(&target_id, &e.to_string());
                    tracing::warn!("Error extracting target '{}': {}", target_id, e);
                }
            }

            progress.iterations_completed = iteration;
            progress.steps_taken = total_steps;

            tokio::time::sleep(std::time::Duration::from_millis(300)).await;
        }

        if iteration >= max_iterations {
            stop_reason = RalphStopReason::MaxIterationsReached;
        }

        progress.is_complete = progress.all_extracted();

        let data = serde_json::json!({
            "targets": progress.targets.iter().map(|t| {
                serde_json::json!({
                    "id": t.id,
                    "description": t.description,
                    "extracted": t.extracted,
                    "data": t.data,
                    "error": t.error
                })
            }).collect::<Vec<_>>()
        });

        let message = match stop_reason {
            RalphStopReason::AllTargetsExtracted => {
                format!(
                    "Successfully extracted all {} targets",
                    progress.targets.len()
                )
            }
            RalphStopReason::MaxIterationsReached => {
                let extracted = progress.targets.iter().filter(|t| t.extracted).count();
                format!(
                    "Max iterations reached. Extracted {}/{} targets",
                    extracted,
                    progress.targets.len()
                )
            }
            RalphStopReason::NoMoreTargets => "No more pending targets".to_string(),
            RalphStopReason::Error => "Error occurred during extraction".to_string(),
            RalphStopReason::Cancelled => "Ralph loop was cancelled".to_string(),
        };

        Ok(RalphResult {
            progress,
            stop_reason,
            data,
            message,
        })
    }

    /// Run a single Ralph iteration - extracts one target
    pub async fn run_ralph_iteration(
        &self,
        browser: &mut StealthBrowser,
        schema: &str,
        custom_prompt: &str,
        max_steps: usize,
        verbose: bool,
    ) -> Result<Value, ScrapioError> {
        use crate::browser_agent::StopReason;

        let mut state = AgentState::new();
        let mut step = 0;
        let mut stop_reason = StopReason::Unknown;

        // Refresh state
        self.refresh_state(browser, &mut state).await?;
        state
            .action_history
            .push(format!("goto: {}", state.current_url));

        if verbose {
            println!("  → Step 1: Navigated to {}", state.current_url);
        }

        while step < max_steps {
            step += 1;

            if state.is_stuck() {
                stop_reason = StopReason::Stuck;
                if verbose {
                    println!("  → Step {}: Agent appears stuck, stopping", step);
                }
                break;
            }

            let snapshot = self.get_page_snapshot(browser, &state).await?;

            let action = self
                .decide_action(&snapshot, schema, &state.action_history, custom_prompt)
                .await?;

            if verbose {
                let action_desc = match &action {
                    BrowserAction::Goto { url } => format!("goto({})", url),
                    BrowserAction::Click { selector } => format!("click({})", selector),
                    BrowserAction::ClickElement { element_id } => {
                        format!("click_element({})", element_id)
                    }
                    BrowserAction::TypeInto { element_id, text } => format!(
                        "type({}, \"{}\")",
                        element_id,
                        text.chars().take(20).collect::<String>()
                    ),
                    BrowserAction::Scroll { pixels } => format!("scroll({})", pixels),
                    BrowserAction::ScrollToBottom => "scroll_to_bottom()".to_string(),
                    BrowserAction::Wait { duration_ms } => format!("wait({}ms)", duration_ms),
                    BrowserAction::ExtractPartial => "extract_partial()".to_string(),
                    BrowserAction::Extract => "extract()".to_string(),
                    BrowserAction::Finish => "finish()".to_string(),
                    BrowserAction::Screenshot => "screenshot()".to_string(),
                    BrowserAction::FindElements { selector } => {
                        format!("find_elements({})", selector)
                    }
                    BrowserAction::ExecuteScript { script } => format!(
                        "execute_script({})",
                        script.chars().take(30).collect::<String>()
                    ),
                    BrowserAction::SetTextSlice { limit, offset } => {
                        format!("set_text_slice(limit={:?}, offset={:?})", limit, offset)
                    }
                };
                println!("  → Step {}: {}", step, action_desc);
            }

            let result = self
                .execute_action(browser, &action, &mut state, schema, custom_prompt)
                .await?;

            match result {
                ActionResult::Success { data: _, message } => {
                    state.record_success();
                    if verbose && let Some(msg) = message {
                        println!("      └─ {}", msg.chars().take(80).collect::<String>());
                    }
                    if action.needs_refresh() {
                        self.refresh_state(browser, &mut state).await?;
                    }
                }
                ActionResult::Error { message } => {
                    state.record_failure(&action.to_history_string(), "", &message);
                    if verbose {
                        println!(
                            "      └─ Error: {}",
                            message.chars().take(80).collect::<String>()
                        );
                    }
                }
                ActionResult::Done { data } => {
                    if verbose {
                        println!("  → Step {}: Extraction complete!", step);
                    }
                    return Ok(serde_json::json!({
                        "data": data,
                        "stop_reason": StopReason::ExtractionCompleted.to_string(),
                        "steps_taken": step,
                    }));
                }
            }

            if matches!(action, BrowserAction::Finish) {
                stop_reason = StopReason::ObjectiveCompleted;
                if verbose {
                    println!("  → Step {}: Finish action triggered", step);
                }
                break;
            }

            tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        }

        Ok(serde_json::json!({
            "steps_taken": step,
            "url": state.current_url,
            "stop_reason": stop_reason.to_string(),
            "extracted_data": state.extracted_data
        }))
    }
}
