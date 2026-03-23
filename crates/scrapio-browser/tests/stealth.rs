//! Tests for stealth module

use scrapio_browser::{StealthConfig, StealthLevel};

#[test]
fn test_stealth_config_default() {
    let config = StealthConfig::default();
    assert_eq!(config.level, StealthLevel::Full);
}

#[test]
fn test_stealth_config_builder() {
    use scrapio_core::UserAgentManager;

    let config = StealthConfig::new(StealthLevel::Basic)
        .with_user_agent(UserAgentManager::new().with_custom("Custom Agent"))
        .with_canvas_seed(12345);

    assert_eq!(config.level, StealthLevel::Basic);
    assert!(config.user_agent.is_some());
    assert_eq!(config.canvas_seed, Some(12345));
}

#[test]
fn test_generate_script_basic() {
    let config = StealthConfig::new(StealthLevel::Basic);
    let script = config.generate_script();
    assert!(script.contains("Object.defineProperty"));
    assert!(script.contains("navigator"));
}

#[test]
fn test_generate_script_full() {
    let config = StealthConfig::new(StealthLevel::Full)
        .with_timezone("America/New_York")
        .with_locale("en-US");
    let script = config.generate_script();

    assert!(script.contains("Object.defineProperty"));
    assert!(script.contains("navigator"));
    assert!(script.contains("WebGLRenderingContext"));
    assert!(script.contains("America/New_York"));
    assert!(script.contains("en-US"));
}
