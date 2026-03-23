//! Tests for driver module

use scrapio_browser::{Arch, DriverManager, DriverType};

#[test]
fn test_driver_type_parse() {
    assert_eq!(DriverType::parse("chrome"), Some(DriverType::Chrome));
    assert_eq!(DriverType::parse("firefox"), Some(DriverType::Firefox));
    assert_eq!(DriverType::parse("geckodriver"), Some(DriverType::Firefox));
    assert_eq!(DriverType::parse("edge"), Some(DriverType::Edge));
    assert_eq!(DriverType::parse("invalid"), None);
}

#[test]
fn test_driver_type_default_port() {
    assert_eq!(DriverType::Chrome.default_port(), 9515);
    assert_eq!(DriverType::Firefox.default_port(), 4444);
    assert_eq!(DriverType::Edge.default_port(), 9516);
}

#[test]
fn test_os_detection() {
    use scrapio_browser::Os;
    let os = Os::current();
    match os {
        Os::Windows => assert_eq!(os.as_str(), "win32"),
        Os::Macos => assert_eq!(os.as_str(), "macos"),
        Os::Linux => assert_eq!(os.as_str(), "linux64"),
    }
}

#[test]
fn test_arch_detection() {
    let arch = Arch::current();
    match arch {
        Arch::Amd64 => assert_eq!(arch.as_str(), "amd64"),
        Arch::Arm64 => assert_eq!(arch.as_str(), "arm64"),
    }
}

#[test]
fn test_manager_default() {
    let manager = DriverManager::new();
    assert_eq!(manager.driver_type(), DriverType::Chrome);
}

#[test]
fn test_manager_with_driver_type() {
    let manager = DriverManager::with_driver_type(DriverType::Firefox);
    assert_eq!(manager.driver_type(), DriverType::Firefox);
    assert_eq!(manager.driver_type().default_port(), 4444);
}
