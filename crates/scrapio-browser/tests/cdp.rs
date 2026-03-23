//! Tests for CDP module

use scrapio_browser::cdp::{
    create_evaluate_command, create_user_agent_command, create_viewport_command,
};

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
