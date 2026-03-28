//! API documentation with Swagger/OpenAPI

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tracing::{debug, error, info, instrument};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use scrapio_ai::AiScraper;
use scrapio_ai::BrowserAiScraper;
use scrapio_classic::Scraper;
use scrapio_core::error::ScrapioResult;
use scrapio_storage::Storage;

/// Request body for scraping
#[derive(Deserialize, Serialize, utoipa::ToSchema)]
pub struct ScrapeRequest {
    /// URL to scrape (required)
    pub url: String,
    /// Use AI for extraction (default: false)
    #[serde(default)]
    pub ai: bool,
    /// Use browser automation (default: false, requires ai=true)
    #[serde(default)]
    pub browser: bool,
    /// Maximum agent steps for browser mode (default: 10)
    #[serde(default)]
    pub max_steps: Option<usize>,
    /// JSON schema for extraction (default: "{}")
    #[serde(default = "default_schema")]
    pub schema: String,
    /// Custom prompt for AI extraction
    #[serde(default)]
    pub prompt: String,
}

fn default_schema() -> String {
    "{}".to_string()
}

/// Classic scrape response
#[derive(Serialize, utoipa::ToSchema)]
pub struct ClassicResponse {
    pub url: String,
    pub status: u16,
    pub title: Option<String>,
    pub links: Vec<String>,
}

/// AI scrape response
#[derive(Serialize, utoipa::ToSchema)]
pub struct AiResponse {
    pub url: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub links: Vec<String>,
    pub model: String,
    pub extraction_mode: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fallback_reason: Option<String>,
}

/// Result item from storage
#[derive(Serialize, utoipa::ToSchema)]
pub struct ResultItem {
    pub id: i64,
    pub url: String,
    pub status: u16,
    pub title: Option<String>,
    pub content: Option<String>,
    pub links: Vec<String>,
}

/// Extract data from a classic scrape response as owned, Send-safe types.
#[instrument(skip(url), fields(url))]
pub async fn classic_scrape_data(
    url: &str,
) -> ScrapioResult<(String, u16, Option<String>, String, Vec<String>)> {
    debug!("Performing classic scrape");
    let resp = Scraper::new().scrape(url).await?;
    let links = resp.links().iter().map(|s| s.to_string()).collect();
    let title = resp.title();
    let out_url = resp.url.clone();
    let status = resp.status;
    let html = resp.html.clone();
    Ok((out_url, status, title, html, links))
}

#[utoipa::path(
    get,
    path = "/health",
    tag = "Health",
    responses(
        (status = 200, description = "Service is healthy", body = serde_json::Value)
    )
)]
pub async fn health() -> (axum::http::StatusCode, axum::Json<serde_json::Value>) {
    (
        axum::http::StatusCode::OK,
        axum::Json(serde_json::json!({ "status": "ok" })),
    )
}

#[instrument(skip(storage))]
#[utoipa::path(
    get,
    path = "/results",
    tag = "Results",
    responses(
        (status = 200, description = "List of saved results", body = serde_json::Value),
        (status = 500, description = "Internal server error", body = serde_json::Value)
    )
)]
pub async fn get_results(
    axum::extract::State(storage): axum::extract::State<Arc<Storage>>,
) -> (axum::http::StatusCode, axum::Json<serde_json::Value>) {
    debug!("Fetching all results");
    match storage.get_all_results(100).await {
        Ok(results) => {
            let json = serde_json::json!({
                "results": results.iter().map(|r| serde_json::json!({
                    "id": r.id, "url": r.url, "status": r.status, "title": r.title
                })).collect::<Vec<_>>()
            });
            (axum::http::StatusCode::OK, axum::Json(json))
        }
        Err(e) => {
            error!("Failed to get results: {}", e);
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                axum::Json(serde_json::json!({"error": e.to_string()})),
            )
        }
    }
}

#[instrument(skip(storage), fields(id))]
#[utoipa::path(
    get,
    path = "/results/{id}",
    tag = "Results",
    responses(
        (status = 200, description = "Get result by ID", body = serde_json::Value),
        (status = 404, description = "Result not found", body = serde_json::Value),
        (status = 500, description = "Internal server error", body = serde_json::Value)
    )
)]
pub async fn get_result(
    axum::extract::State(storage): axum::extract::State<Arc<Storage>>,
    axum::extract::Path(id): axum::extract::Path<i64>,
) -> (axum::http::StatusCode, axum::Json<serde_json::Value>) {
    debug!("Fetching result by ID: {}", id);
    match storage.get_result_by_id(id).await {
        Ok(Some(r)) => {
            let json = serde_json::json!({
                "id": r.id, "url": r.url, "status": r.status,
                "title": r.title, "content": r.content, "links": r.links
            });
            (axum::http::StatusCode::OK, axum::Json(json))
        }
        Ok(None) => {
            debug!("Result not found: {}", id);
            (
                axum::http::StatusCode::NOT_FOUND,
                axum::Json(serde_json::json!({"error": "Not found"})),
            )
        }
        Err(e) => {
            error!("Failed to get result: {}", e);
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                axum::Json(serde_json::json!({"error": e.to_string()})),
            )
        }
    }
}

#[instrument(skip(storage, body), fields(url = %body.url, ai = body.ai, browser = body.browser))]
#[utoipa::path(
    post,
    path = "/scrape",
    tag = "Scraping",
    request_body = ScrapeRequest,
    responses(
        (status = 200, description = "Scraping successful", body = serde_json::Value),
        (status = 400, description = "Bad request", body = serde_json::Value),
        (status = 500, description = "Internal server error", body = serde_json::Value)
    )
)]
pub async fn scrape(
    axum::extract::State(storage): axum::extract::State<Arc<Storage>>,
    axum::extract::Json(body): axum::extract::Json<ScrapeRequest>,
) -> (axum::http::StatusCode, axum::Json<serde_json::Value>) {
    info!(
        "Scrape request: url={}, ai={}, browser={}",
        body.url, body.ai, body.browser
    );
    let payload = body;

    // Browser-based AI scraping
    if payload.ai && payload.browser {
        let max_steps = payload.max_steps.unwrap_or(10);
        let scraper = BrowserAiScraper::new().with_max_steps(max_steps);
        let schema = if payload.schema.is_empty() {
            "{}"
        } else {
            &payload.schema
        };
        let prompt = if payload.prompt.is_empty() {
            ""
        } else {
            &payload.prompt
        };
        match scraper
            .scrape_with_managed_browser(&payload.url, schema, prompt, None, true)
            .await
        {
            Ok(result) => {
                info!("Browser AI scrape completed successfully");
                let _ = storage
                    .save_result(
                        &result.url,
                        200,
                        result.data.get("title").and_then(|t| t.as_str()),
                        &serde_json::to_string(&result.data).unwrap_or_default(),
                        &result.links,
                    )
                    .await;
                let json = serde_json::json!({
                    "url": result.url,
                    "data": result.data,
                    "model": result.model,
                    "stop_reason": result.data.get("stop_reason"),
                    "steps_taken": result.data.get("steps_taken")
                });
                (axum::http::StatusCode::OK, axum::Json(json))
            }
            Err(e) => {
                error!("Browser AI scrape failed: {}", e);
                (
                    axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                    axum::Json(serde_json::json!({"error": e.to_string()})),
                )
            }
        }
    }
    // HTTP-based AI scraping
    else if payload.ai {
        let scraper = AiScraper::new();
        let schema = if payload.schema.is_empty() {
            "{}"
        } else {
            &payload.schema
        };
        match scraper.scrape(&payload.url, schema).await {
            Ok(result) => {
                info!("HTTP AI scrape completed successfully");
                let _ = storage
                    .save_result(
                        &result.url,
                        200,
                        result.data.get("title").and_then(|t| t.as_str()),
                        &serde_json::to_string(&result.data).unwrap_or_default(),
                        &result.links,
                    )
                    .await;
                let json = serde_json::json!({
                    "url": result.url,
                    "title": result.data.get("title"),
                    "description": result.data.get("description"),
                    "links": result.links,
                    "model": result.model,
                    "extraction_mode": format!("{:?}", result.mode),
                    "fallback_reason": result.fallback_reason.as_ref().map(|r| format!("{:?}", r)),
                    "provider_error": result.provider_error
                });
                (axum::http::StatusCode::OK, axum::Json(json))
            }
            Err(e) => {
                error!("HTTP AI scrape failed: {}", e);
                (
                    axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                    axum::Json(serde_json::json!({"error": e.to_string()})),
                )
            }
        }
    } else {
        let scrape_result = classic_scrape_data(&payload.url).await;
        let (url, status, title, html, links) = match scrape_result {
            Ok(data) => data,
            Err(e) => {
                error!("Classic scrape failed: {}", e);
                return (
                    axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                    axum::Json(serde_json::json!({"error": e.to_string()})),
                );
            }
        };
        info!("Classic scrape completed successfully");
        let _ = storage
            .save_result(&url, status, title.as_deref(), &html, &links)
            .await;
        let json = serde_json::json!({
            "url": url,
            "status": status,
            "title": title,
            "links": links
        });
        (axum::http::StatusCode::OK, axum::Json(json))
    }
}

/// OpenAPI documentation struct
#[derive(OpenApi)]
#[openapi(
    paths(health, get_results, get_result, scrape),
    components(
        schemas(ScrapeRequest, ClassicResponse, AiResponse, ResultItem)
    ),
    tags(
        (name = "Health", description = "Health check endpoints"),
        (name = "Scraping", description = "Web scraping endpoints"),
        (name = "Results", description = "Result storage endpoints")
    )
)]
pub struct ApiDoc;

/// Create the Swagger UI router
pub fn create_swagger_router() -> SwaggerUi {
    SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi())
}
