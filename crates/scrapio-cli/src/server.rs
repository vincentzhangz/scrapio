//! HTTP API server

use std::sync::Arc;

use axum::{
    Router,
    routing::{get, post},
};

use scrapio_storage::Storage;

pub use crate::swagger::{create_swagger_router, get_result, get_results, health, scrape};

pub async fn serve_api_server(host: String, port: u16) {
    let storage = Arc::new(
        Storage::new(":memory:")
            .await
            .expect("Failed to create storage"),
    );

    let app = Router::new()
        .merge(create_swagger_router())
        .route("/health", get(health))
        .route("/results", get(get_results))
        .route("/results/{id}", get(get_result))
        .route("/scrape", post(scrape))
        .with_state(storage);

    let addr = format!("{}:{}", host, port);
    println!("Starting API server on {}", addr);
    println!();
    println!("Swagger UI: http://{}/swagger-ui", addr);
    println!("OpenAPI spec: http://{}/api-docs/openapi.json", addr);
    println!();
    println!("Server running at http://{}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
