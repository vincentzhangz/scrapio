//! Scrapio MCP Server Binary

use scrapio_mcp::run_mcp_server;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    run_mcp_server().await
}
