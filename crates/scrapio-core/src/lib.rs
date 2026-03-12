//! Core types and utilities for Scrapio

pub mod error;
pub mod http;
pub mod user_agent;
pub mod utils;

pub use error::{ScrapioError, ScrapioResult};
pub use http::{DEFAULT_TIMEOUT, DEFAULT_USER_AGENT};
pub use user_agent::{Browser, RotationStrategy, UserAgentManager, profiles};
