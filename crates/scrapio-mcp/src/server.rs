//! MCP server implementation for Scrapio.

use crate::error::ScrapioMcpError;
use crate::tools::*;
use rmcp::{
    ServerHandler, ServiceExt,
    model::{
        CallToolRequestParams, Content, InitializeRequestParams, InitializeResult, ListToolsResult,
        PaginatedRequestParams, ServerCapabilities, Tool,
    },
    service::RequestContext,
    transport::streamable_http_server::{StreamableHttpServerConfig, StreamableHttpService},
};
use std::sync::Arc;
use tokio::io::{stdin, stdout};
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

/// Initialize tracing for the MCP server
fn init_tracing() {
    tracing_subscriber::registry()
        .with(fmt::layer().with_target(true).with_line_number(true))
        .with(EnvFilter::from_default_env())
        .init();
}

/// Convert a serde_json::Value to Arc<JsonObject>
/// This is needed because Tool::new expects Arc<JsonObject> not serde_json::Value
fn value_to_json_object(value: serde_json::Value) -> Arc<rmcp::model::JsonObject> {
    match value {
        serde_json::Value::Object(map) => Arc::new(
            map.into_iter()
                .collect::<serde_json::Map<String, serde_json::Value>>(),
        ),
        _ => Arc::new(serde_json::Map::new()),
    }
}

/// The main Scrapio MCP server.
#[derive(Debug, Clone, Default)]
pub struct ScrapioMcpServer;

impl ScrapioMcpServer {
    /// Create a new Scrapio MCP server.
    pub fn new() -> Self {
        Self
    }

    /// Get the list of available tools.
    pub fn get_tools() -> Vec<Tool> {
        vec![
            Tool::new(
                "classic_scrape",
                "Scrape a URL using CSS selectors. Returns title, links, and HTML preview.",
                value_to_json_object(serde_json::json!({
                    "type": "object",
                    "properties": {
                        "url": { "type": "string" },
                        "selector": { "type": "string" }
                    },
                    "required": ["url"]
                })),
            ),
            Tool::new(
                "ai_scrape",
                "Scrape a URL using AI-powered extraction with structured output.",
                value_to_json_object(serde_json::json!({
                    "type": "object",
                    "properties": {
                        "url": { "type": "string" },
                        "schema": { "type": "string" },
                        "provider": { "type": "string", "default": "openai" },
                        "model": { "type": "string" }
                    },
                    "required": ["url"]
                })),
            ),
            Tool::new(
                "crawl_start",
                "Start a crawl operation to discover and scrape multiple pages.",
                value_to_json_object(serde_json::json!({
                    "type": "object",
                    "properties": {
                        "url": { "type": "string" },
                        "depth": { "type": "number" },
                        "max_pages": { "type": "number" },
                        "scope": { "type": "string", "default": "domain" }
                    },
                    "required": ["url"]
                })),
            ),
            Tool::new(
                "crawl_status",
                "Get the status of a crawl operation.",
                value_to_json_object(serde_json::json!({
                    "type": "object",
                    "properties": {
                        "crawl_id": { "type": "string" }
                    },
                    "required": ["crawl_id"]
                })),
            ),
            Tool::new(
                "browser_navigate",
                "Navigate to a URL using a headless browser and get rendered HTML.",
                value_to_json_object(serde_json::json!({
                    "type": "object",
                    "properties": {
                        "url": { "type": "string" },
                        "headless": { "type": "boolean", "default": true },
                        "browser": { "type": "string", "default": "chrome" }
                    },
                    "required": ["url"]
                })),
            ),
            Tool::new(
                "storage_save",
                "Save a scraping result to storage.",
                value_to_json_object(serde_json::json!({
                    "type": "object",
                    "properties": {
                        "url": { "type": "string" },
                        "content": { "type": "string" },
                        "database": { "type": "string", "default": ":memory:" }
                    },
                    "required": ["url", "content"]
                })),
            ),
            Tool::new(
                "storage_get",
                "Get a stored result by ID.",
                value_to_json_object(serde_json::json!({
                    "type": "object",
                    "properties": {
                        "id": { "type": "number" },
                        "database": { "type": "string", "default": ":memory:" }
                    },
                    "required": ["id"]
                })),
            ),
            Tool::new(
                "storage_list",
                "List all stored results.",
                value_to_json_object(serde_json::json!({
                    "type": "object",
                    "properties": {
                        "limit": { "type": "number", "default": 10 },
                        "database": { "type": "string", "default": ":memory:" }
                    }
                })),
            ),
        ]
    }

    /// Call a tool by name and return the result.
    async fn call_tool_by_name(
        &self,
        name: &str,
        args: serde_json::Value,
    ) -> Result<serde_json::Value, ScrapioMcpError> {
        match name {
            "classic_scrape" => {
                let input: ClassicScrapeInput = serde_json::from_value(args)
                    .map_err(|e| ScrapioMcpError::InvalidInput(e.to_string()))?;
                let output = classic_scrape_impl(input).await?;
                serde_json::to_value(output)
                    .map_err(|e| ScrapioMcpError::InvalidInput(e.to_string()))
            }
            "ai_scrape" => {
                let input: AiScrapeInput = serde_json::from_value(args)
                    .map_err(|e| ScrapioMcpError::InvalidInput(e.to_string()))?;
                let output = ai_scrape_impl(input).await?;
                serde_json::to_value(output)
                    .map_err(|e| ScrapioMcpError::InvalidInput(e.to_string()))
            }
            "crawl_start" => {
                let input: CrawlStartInput = serde_json::from_value(args)
                    .map_err(|e| ScrapioMcpError::InvalidInput(e.to_string()))?;
                let output = crawl_start_impl(input).await?;
                serde_json::to_value(output)
                    .map_err(|e| ScrapioMcpError::InvalidInput(e.to_string()))
            }
            "crawl_status" => {
                let input: CrawlStatusInput = serde_json::from_value(args)
                    .map_err(|e| ScrapioMcpError::InvalidInput(e.to_string()))?;
                let output = crawl_status_impl(input).await?;
                serde_json::to_value(output)
                    .map_err(|e| ScrapioMcpError::InvalidInput(e.to_string()))
            }
            "browser_navigate" => {
                let input: BrowserNavigateInput = serde_json::from_value(args)
                    .map_err(|e| ScrapioMcpError::InvalidInput(e.to_string()))?;
                let output = browser_navigate_impl(input).await?;
                serde_json::to_value(output)
                    .map_err(|e| ScrapioMcpError::InvalidInput(e.to_string()))
            }
            "storage_save" => {
                let input: StorageSaveInput = serde_json::from_value(args)
                    .map_err(|e| ScrapioMcpError::InvalidInput(e.to_string()))?;
                let output = storage_save_impl(input).await?;
                serde_json::to_value(output)
                    .map_err(|e| ScrapioMcpError::InvalidInput(e.to_string()))
            }
            "storage_get" => {
                let input: StorageGetInput = serde_json::from_value(args)
                    .map_err(|e| ScrapioMcpError::InvalidInput(e.to_string()))?;
                let output = storage_get_impl(input).await?;
                serde_json::to_value(output)
                    .map_err(|e| ScrapioMcpError::InvalidInput(e.to_string()))
            }
            "storage_list" => {
                let input: StorageListInput = serde_json::from_value(args)
                    .map_err(|e| ScrapioMcpError::InvalidInput(e.to_string()))?;
                let output = storage_list_impl(input).await?;
                serde_json::to_value(output)
                    .map_err(|e| ScrapioMcpError::InvalidInput(e.to_string()))
            }
            _ => Err(ScrapioMcpError::InvalidInput(format!(
                "Unknown tool: {}",
                name
            ))),
        }
    }
}

impl ServerHandler for ScrapioMcpServer {
    async fn initialize(
        &self,
        request: InitializeRequestParams,
        context: RequestContext<rmcp::RoleServer>,
    ) -> Result<InitializeResult, rmcp::ErrorData> {
        if context.peer.peer_info().is_none() {
            context.peer.set_peer_info(request);
        }
        Ok(self.get_info())
    }

    fn get_info(&self) -> InitializeResult {
        let capabilities = ServerCapabilities::builder()
            .enable_tools()
            .enable_resources()
            .enable_prompts()
            .build();

        InitializeResult::new(capabilities)
            .with_server_info(rmcp::model::Implementation::new(
                "scrapio",
                env!("CARGO_PKG_VERSION"),
            ))
            .with_instructions(
                "Scrapio MCP server - web scraping toolkit with AI extraction. \
                 Set OPENAI_API_KEY or ANTHROPIC_API_KEY for AI features.",
            )
    }

    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<rmcp::RoleServer>,
    ) -> Result<ListToolsResult, rmcp::ErrorData> {
        Ok(ListToolsResult {
            tools: Self::get_tools(),
            ..Default::default()
        })
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        _context: RequestContext<rmcp::RoleServer>,
    ) -> Result<rmcp::model::CallToolResult, rmcp::ErrorData> {
        // Convert JsonObject to serde_json::Value for deserialization
        let args = request.arguments.unwrap_or_default();
        let args_value = serde_json::Value::Object(args);

        // Get tool name as &str for matching
        let tool_name: &str = &request.name;

        match self.call_tool_by_name(tool_name, args_value).await {
            Ok(output) => {
                let text = serde_json::to_string(&output).unwrap_or_default();
                let content = Content::text(text);
                Ok(rmcp::model::CallToolResult::success(vec![content]))
            }
            Err(e) => {
                let content = Content::text(e.to_string());
                Ok(rmcp::model::CallToolResult::error(vec![content]))
            }
        }
    }
}

/// Run the MCP server with stdio transport.
pub async fn run_mcp_server() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    init_tracing();
    let server = ScrapioMcpServer::new();
    let transport = (stdin(), stdout());
    let service = server.serve(transport).await?;
    let _quit_reason = service.waiting().await?;
    Ok(())
}

/// Run the MCP server with HTTP transport.
pub async fn run_mcp_http_server(
    host: String,
    port: u16,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use axum::Router;
    use rmcp::transport::streamable_http_server::session::local::LocalSessionManager;
    use tower_http::cors::{Any, CorsLayer};
    use tracing::info;

    init_tracing();
    let server = ScrapioMcpServer::new();
    let config = StreamableHttpServerConfig::default();
    let mcp_service: StreamableHttpService<_, LocalSessionManager> =
        StreamableHttpService::new(move || Ok(server.clone()), Default::default(), config);

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new().nest_service("/mcp", mcp_service).layer(cors);

    let addr = format!("{}:{}", host, port);
    info!("Starting MCP HTTP server on http://{}/mcp", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
