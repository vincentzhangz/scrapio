//! HTTP API server

use std::sync::Arc;

use axum::{
    Router,
    routing::{get, post},
};
use tracing::info;

use scrapio_storage::Storage;

pub use crate::swagger::{create_swagger_router, get_result, get_results, health, scrape};

#[tracing::instrument(fields(host, port))]
pub async fn serve_api_server(host: String, port: u16) -> Result<(), Box<dyn std::error::Error>> {
    info!("Starting API server");
    let storage = Arc::new(
        Storage::new(":memory:")
            .await
            .map_err(|e| format!("Failed to create storage: {}", e))?,
    );

    let app = Router::new()
        .merge(create_swagger_router())
        .route("/health", get(health))
        .route("/results", get(get_results))
        .route("/results/{id}", get(get_result))
        .route("/scrape", post(scrape))
        .with_state(storage);

    let addr = format!("{}:{}", host, port);
    info!("Starting API server on {}", addr);
    info!("Swagger UI: http://{}/swagger-ui", addr);
    info!("OpenAPI spec: http://{}/api-docs/openapi.json", addr);
    info!("Server running at http://{}", addr);

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .map_err(|e| format!("Failed to bind to {}: {}", addr, e))?;
    axum::serve(listener, app)
        .await
        .map_err(|e| format!("Server error: {}", e))?;

    Ok(())
}
