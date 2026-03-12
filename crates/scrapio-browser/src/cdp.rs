//! Chrome DevTools Protocol (CDP) integration for stealth browser automation
//!
//! This module provides CDP commands for injecting stealth scripts and
//! manipulating browser state for anti-detection.

use serde::{Deserialize, Serialize};

/// CDP command for evaluating JavaScript in the browser context
#[derive(Debug, Serialize, Deserialize)]
pub struct EvaluateCommand {
    pub id: u64,
    pub method: String,
    pub params: EvaluateParams,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EvaluateParams {
    /// JavaScript expression to evaluate
    pub expression: String,
    /// Whether to return the value as a string
    #[serde(default)]
    pub return_by_value: bool,
    /// Whether to evaluate in the main world
    #[serde(default)]
    pub context_arena_id: Option<u64>,
}

/// CDP response from evaluation
#[derive(Debug, Deserialize)]
pub struct EvaluateResponse {
    pub id: u64,
    pub result: EvaluateResult,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum EvaluateResult {
    Success { result: CDPValue },
    Error { error: CDPError },
}

#[derive(Debug, Deserialize)]
pub struct CDPValue {
    #[serde(rename = "type")]
    pub value_type: String,
    pub value: Option<serde_json::Value>,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CDPError {
    pub code: i32,
    pub message: String,
}

/// CDP command for adding a script to evaluate on each document
#[derive(Debug, Serialize, Deserialize)]
pub struct AddScriptCommand {
    pub id: u64,
    pub method: String,
    pub params: AddScriptParams,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AddScriptParams {
    /// Source code of the script
    pub source: String,
    /// Whether the script should be injected into all frames
    #[serde(default)]
    pub world_name: Option<String>,
}

/// CDP command for configuring network conditions
#[derive(Debug, Serialize, Deserialize)]
pub struct NetworkConditionsCommand {
    pub id: u64,
    pub method: String,
    pub params: NetworkConditionsParams,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NetworkConditionsParams {
    /// Whether to disable network throttling
    #[serde(rename = "offline")]
    pub offline: bool,
    /// Download speed in bytes per second (-1 = unlimited)
    #[serde(rename = "downloadThroughput")]
    pub download_throughput: i64,
    /// Upload speed in bytes per second (-1 = unlimited)
    #[serde(rename = "uploadThroughput")]
    pub upload_throughput: i64,
    /// Minimum latency in milliseconds
    #[serde(rename = "latency")]
    pub latency: i64,
}

/// CDP command for setting user agent override
#[derive(Debug, Serialize, Deserialize)]
pub struct UserAgentOverrideCommand {
    pub id: u64,
    pub method: String,
    pub params: UserAgentOverrideParams,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserAgentOverrideParams {
    /// User agent string
    pub user_agent: String,
    /// Platform (e.g., "Win32", "MacIntel")
    #[serde(default)]
    pub platform: Option<String>,
    /// Accept language
    #[serde(default, rename = "acceptLanguage")]
    pub accept_language: Option<String>,
}

/// CDP command for viewport configuration
#[derive(Debug, Serialize, Deserialize)]
pub struct SetViewportCommand {
    pub id: u64,
    pub method: String,
    pub params: SetViewportParams,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SetViewportParams {
    /// Viewport width
    pub width: u32,
    /// Viewport height
    pub height: u32,
    /// Device scale factor
    #[serde(default = "default_dsf")]
    pub device_scale_factor: f64,
    /// Whether the viewport is mobile
    #[serde(default)]
    pub is_mobile: bool,
    /// Whether the viewport supports touch
    #[serde(default)]
    pub is_touch: bool,
    /// Whether the viewport is in landscape mode
    #[serde(default)]
    pub is_landscape: bool,
}

fn default_dsf() -> f64 {
    1.0
}

/// Generate CDP command JSON for evaluating a stealth script
pub fn create_evaluate_command(script: &str) -> String {
    let id = 1u64;
    let cmd = EvaluateCommand {
        id,
        method: "Runtime.evaluate".to_string(),
        params: EvaluateParams {
            expression: script.to_string(),
            return_by_value: true,
            context_arena_id: None,
        },
    };
    serde_json::to_string(&cmd).unwrap_or_default()
}

/// Generate CDP command JSON for setting user agent
pub fn create_user_agent_command(user_agent: &str) -> String {
    let id = 2u64;
    let cmd = UserAgentOverrideCommand {
        id,
        method: "Network.setUserAgentOverride".to_string(),
        params: UserAgentOverrideParams {
            user_agent: user_agent.to_string(),
            platform: None,
            accept_language: None,
        },
    };
    serde_json::to_string(&cmd).unwrap_or_default()
}

/// Generate CDP command JSON for setting viewport
pub fn create_viewport_command(width: u32, height: u32) -> String {
    let id = 3u64;
    let cmd = SetViewportCommand {
        id,
        method: "Emulation.setDeviceMetricsOverride".to_string(),
        params: SetViewportParams {
            width,
            height,
            device_scale_factor: 1.0,
            is_mobile: false,
            is_touch: false,
            is_landscape: false,
        },
    };
    serde_json::to_string(&cmd).unwrap_or_default()
}

/// Parse CDP response for evaluation result
pub fn parse_evaluate_response(response: &str) -> Option<String> {
    let resp: EvaluateResponse = serde_json::from_str(response).ok()?;

    match resp.result {
        EvaluateResult::Success { result } => result
            .description
            .or_else(|| result.value.as_ref().map(|v| v.to_string())),
        EvaluateResult::Error { error } => Some(format!("Error: {}", error.message)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_evaluate_command() {
        let script = "console.log('test')";
        let cmd = create_evaluate_command(script);
        assert!(cmd.contains("Runtime.evaluate"));
        assert!(cmd.contains("test"));
    }

    #[test]
    fn test_create_user_agent_command() {
        let cmd = create_user_agent_command("Custom Agent/1.0");
        assert!(cmd.contains("Network.setUserAgentOverride"));
        assert!(cmd.contains("Custom Agent"));
    }

    #[test]
    fn test_create_viewport_command() {
        let cmd = create_viewport_command(1920, 1080);
        assert!(cmd.contains("Emulation.setDeviceMetricsOverride"));
        assert!(cmd.contains("1920"));
        assert!(cmd.contains("1080"));
    }
}
