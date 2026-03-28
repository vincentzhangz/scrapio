//! Core types and utilities for Scrapio
//!
//! This crate provides the foundational types used across all Scrapio crates:
//! - Error types for unified error handling
//! - HTTP client utilities
//! - User agent management with browser profiles
//! - Proxy configuration and rotation
//! - URL validation utilities

pub mod error;
pub mod http;
pub mod proxy;
pub mod user_agent;
pub mod utils;

pub use error::{ScrapioError, ScrapioResult};
pub use http::{DEFAULT_TIMEOUT, DEFAULT_USER_AGENT};
pub use proxy::{
    AnonymityLevel, ProxyConfig, ProxyHealth, ProxyManager, ProxyRotationConfig,
    RotationStrategy as ProxyRotationStrategy,
};
pub use user_agent::{Browser, RotationStrategy, UserAgentManager, profiles};
