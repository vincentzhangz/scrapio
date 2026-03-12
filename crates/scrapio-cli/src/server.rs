//! HTTP API server

use std::sync::Arc;

use axum::{
    Router,
    extract::Path,
    response::Json as AxumJson,
    routing::{get, post},
};
use serde::Deserialize;

use scrapio_ai::AiScraper;
use scrapio_classic::Scraper;
use scrapio_core::error::ScrapioResult;
use scrapio_storage::Storage;

/// Extract data from a classic scrape response as owned, Send-safe types.
/// `scrapio_classic::Response` contains `Html` which is not `Send`, so we
/// eagerly copy out all needed fields here and let `Response` drop before
/// any subsequent `.await` in the caller.
async fn classic_scrape_data(
    url: &str,
) -> ScrapioResult<(String, u16, Option<String>, String, Vec<String>)> {
    let resp = Scraper::new().scrape(url).await?;
    let links = resp.links().iter().map(|s| s.to_string()).collect();
    let title = resp.title();
    let out_url = resp.url.clone();
    let status = resp.status;
    let html = resp.html.clone();
    Ok((out_url, status, title, html, links))
}

#[derive(Deserialize)]
struct ScrapeRequest {
    url: String,
    #[serde(default)]
    ai: bool,
}

async fn health() -> (axum::http::StatusCode, AxumJson<serde_json::Value>) {
    (
        axum::http::StatusCode::OK,
        AxumJson(serde_json::json!({ "status": "ok" })),
    )
}

async fn get_results(
    axum::extract::State(storage): axum::extract::State<Arc<Storage>>,
) -> (axum::http::StatusCode, AxumJson<serde_json::Value>) {
    match storage.get_all_results(100).await {
        Ok(results) => {
            let json = serde_json::json!({
                "results": results.iter().map(|r| serde_json::json!({
                    "id": r.id, "url": r.url, "status": r.status, "title": r.title
                })).collect::<Vec<_>>()
            });
            (axum::http::StatusCode::OK, AxumJson(json))
        }
        Err(e) => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            AxumJson(serde_json::json!({"error": e.to_string()})),
        ),
    }
}

async fn get_result(
    axum::extract::State(storage): axum::extract::State<Arc<Storage>>,
    Path(id): Path<i64>,
) -> (axum::http::StatusCode, AxumJson<serde_json::Value>) {
    match storage.get_result_by_id(id).await {
        Ok(Some(r)) => {
            let json = serde_json::json!({
                "id": r.id, "url": r.url, "status": r.status,
                "title": r.title, "content": r.content, "links": r.links
            });
            (axum::http::StatusCode::OK, AxumJson(json))
        }
        Ok(None) => (
            axum::http::StatusCode::NOT_FOUND,
            AxumJson(serde_json::json!({"error": "Not found"})),
        ),
        Err(e) => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            AxumJson(serde_json::json!({"error": e.to_string()})),
        ),
    }
}

async fn scrape(
    axum::extract::State(storage): axum::extract::State<Arc<Storage>>,
    body: axum::extract::Json<ScrapeRequest>,
) -> (axum::http::StatusCode, AxumJson<serde_json::Value>) {
    let payload = body.0;
    if payload.ai {
        let scraper = AiScraper::new();
        match scraper.scrape(&payload.url, "{}").await {
            Ok(result) => {
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
                    "url": result.url, "title": result.data.get("title"),
                    "description": result.data.get("description"), "links": result.links,
                    "model": result.model, "used_fallback": result.used_fallback
                });
                (axum::http::StatusCode::OK, AxumJson(json))
            }
            Err(e) => (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                AxumJson(serde_json::json!({"error": e.to_string()})),
            ),
        }
    } else {
        let scrape_result = classic_scrape_data(&payload.url).await;
        let (url, status, title, html, links) = match scrape_result {
            Ok(data) => data,
            Err(e) => {
                return (
                    axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                    AxumJson(serde_json::json!({"error": e.to_string()})),
                );
            }
        };
        let _ = storage
            .save_result(&url, status, title.as_deref(), &html, &links)
            .await;
        let json =
            serde_json::json!({"url": url, "status": status, "title": title, "links": links});
        (axum::http::StatusCode::OK, AxumJson(json))
    }
}

pub async fn serve_api_server(host: String, port: u16) {
    let storage = Arc::new(
        Storage::new(":memory:")
            .await
            .expect("Failed to create storage"),
    );

    let app = Router::new()
        .route("/health", get(health))
        .route("/results", get(get_results))
        .route("/results/{id}", get(get_result))
        .route("/scrape", post(scrape))
        .with_state(storage);

    let addr = format!("{}:{}", host, port);
    println!("Starting API server on {}", addr);
    println!("Endpoints:");
    println!(
        "  POST /scrape        - Scrape a URL (JSON: {{\"url\": \"...\", \"ai\": true/false}})"
    );
    println!("  GET  /results       - List saved results");
    println!("  GET  /results/{{id}} - Get specific result");
    println!("\nServer running at http://{}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
