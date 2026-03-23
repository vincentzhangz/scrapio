//! Tests for robots module (public API)

use scrapio_classic::{PolitenessConfig, RobotsTxtManager};

#[test]
fn test_politeness_config_default() {
    let config = PolitenessConfig::default();
    assert!(config.respect_robots_txt);
    assert!(config.respect_crawl_delay);
}

#[test]
fn test_politeness_config_builder() {
    let config = PolitenessConfig::default()
        .with_requests_per_second(5.0)
        .with_min_crawl_delay(1000);

    assert_eq!(config.max_requests_per_second, 5.0);
    assert_eq!(config.min_crawl_delay_ms, 1000);
}