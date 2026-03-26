//! Scrapio MCP Server
//!
//! This crate provides an MCP (Model Context Protocol) server implementation
//! for the Scrapio web scraping toolkit.

mod error;
mod server;
mod tools;

pub use error::ScrapioMcpError;
pub use server::{run_mcp_http_server, run_mcp_server, ScrapioMcpServer};
