use clap::{Parser, Subcommand};
use scrapio_ai::AiScraper;
use scrapio_classic::Scraper;
use scrapio_runtime::Runtime;
use scrapio_storage::Storage;
use serde::Deserialize;
use std::sync::Arc;

#[derive(Parser)]
#[command(name = "scrapio")]
#[command(about = "All-in-one web scraping toolkit", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    #[arg(long, default_value = "info")]
    log_level: String,
}

#[derive(Subcommand)]
enum Commands {
    Classic {
        url: String,
    },
    Ai {
        url: String,
        #[arg(long)]
        schema: Option<String>,
        #[arg(long, default_value = "openai")]
        provider: String,
        #[arg(long, default_value = "")]
        model: String,
    },
    Crawl {
        url: String,
        #[arg(long, default_value = "2")]
        depth: usize,
    },
    Save {
        url: String,
        #[arg(long, default_value = "scrapio.db")]
        database: String,
    },
    List {
        #[arg(long, default_value = "scrapio.db")]
        database: String,
        #[arg(long, default_value = "10")]
        limit: usize,
    },
    Serve {
        #[arg(long, default_value = "127.0.0.1")]
        host: String,
        #[arg(long, default_value = "8080")]
        port: u16,
    },
    Version,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Classic { url } => {
            let runtime = scrapio_runtime::TokioRuntime::default();
            runtime.block_on(async {
                let scraper = Scraper::new();
                match scraper.scrape(&url).await {
                    Ok(resp) => {
                        println!("Status: {}", resp.status);
                        if let Some(title) = resp.title() {
                            println!("Title: {}", title);
                        }
                        println!("Links: {}", resp.links().len());
                    }
                    Err(e) => eprintln!("Error: {}", e),
                }
            });
        }
        Commands::Ai {
            url,
            schema,
            provider,
            model,
        } => {
            let runtime = scrapio_runtime::TokioRuntime::default();
            runtime.block_on(async {
                let mut config = scrapio_ai::AiConfig::new().with_provider(&provider);
                if !model.is_empty() {
                    config = config.with_model(&model);
                }
                let scraper = AiScraper::with_config(config);
                let schema = schema.unwrap_or_else(|| "{}".to_string());
                match scraper.scrape(&url, &schema).await {
                    Ok(result) => {
                        println!("URL: {}", result.url);
                        println!("Model: {}", result.model);
                        println!("Used fallback: {}", result.used_fallback);
                        println!("Links found: {}", result.links.len());
                        println!(
                            "Data: {}",
                            serde_json::to_string_pretty(&result.data).unwrap_or_default()
                        );
                    }
                    Err(e) => eprintln!("Error: {}", e),
                }
            });
        }
        Commands::Crawl { url, depth } => {
            let runtime = scrapio_runtime::TokioRuntime::default();
            runtime.block_on(async {
                let scraper = Scraper::new();
                let mut visited = std::collections::HashSet::new();
                let mut queue = vec![url.clone()];
                let mut current_depth = 0;

                while current_depth < depth && !queue.is_empty() {
                    #[allow(clippy::drain_collect)]
                    let urls: Vec<String> = queue.drain(..).collect();
                    println!("Depth {}: processing {} URLs", current_depth, urls.len());

                    for url in urls {
                        if visited.contains(&url) {
                            continue;
                        }
                        visited.insert(url.clone());
                        match scraper.scrape(&url).await {
                            Ok(resp) => {
                                println!("  - {} (status: {})", url, resp.status);
                                if let Some(title) = resp.title() {
                                    println!("    Title: {}", title.trim());
                                }
                                if current_depth + 1 < depth {
                                    for link in resp.links() {
                                        if !visited.contains(&link) && link.starts_with("http") {
                                            queue.push(link);
                                        }
                                    }
                                }
                            }
                            Err(e) => eprintln!("  - {} Error: {}", url, e),
                        }
                        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                    }
                    current_depth += 1;
                }
                println!("\nCrawl complete! Visited {} pages", visited.len());
            });
        }
        Commands::Save { url, database } => {
            let runtime = scrapio_runtime::TokioRuntime::default();
            runtime.block_on(async {
                let scraper = Scraper::new();
                match Storage::new(&database).await {
                    Ok(storage) => match scraper.scrape(&url).await {
                        Ok(resp) => {
                            let title = resp.title();
                            let links = resp.links();
                            match storage
                                .save_result(
                                    &url,
                                    resp.status,
                                    title.as_deref(),
                                    &resp.html,
                                    &links,
                                )
                                .await
                            {
                                Ok(id) => println!("Saved result with ID: {}", id),
                                Err(e) => eprintln!("Save error: {}", e),
                            }
                        }
                        Err(e) => eprintln!("Scrape error: {}", e),
                    },
                    Err(e) => eprintln!("Database error: {}", e),
                }
            });
        }
        Commands::List { database, limit } => {
            let runtime = scrapio_runtime::TokioRuntime::default();
            runtime.block_on(async {
                match Storage::new(&database).await {
                    Ok(storage) => match storage.get_all_results(limit).await {
                        Ok(results) => {
                            println!("Found {} results:\n", results.len());
                            for r in results {
                                println!("  {} - {} - {}", r.id, r.status, r.url);
                                if let Some(title) = &r.title {
                                    println!("    Title: {}", title);
                                }
                            }
                        }
                        Err(e) => eprintln!("Error: {}", e),
                    },
                    Err(e) => eprintln!("Database error: {}", e),
                }
            });
        }
        Commands::Serve { host, port } => {
            let runtime = scrapio_runtime::TokioRuntime::default();
            runtime.block_on(async {
                serve_api_server(host, port).await;
            });
        }
        Commands::Version => {
            println!("scrapio v{}", env!("CARGO_PKG_VERSION"));
            println!("Runtime: Tokio");
            println!("Features: classic, ai, storage");
        }
    }

    Ok(())
}

async fn serve_api_server(host: String, port: u16) {
    use axum::{
        Router,
        extract::{Json, Path},
        response::Json as AxumJson,
        routing::{get, post},
    };

    #[derive(Deserialize)]
    struct ScrapeRequest {
        url: String,
        #[serde(default)]
        ai: bool,
    }

    // Use in-memory SQLite for the API server - create once and share
    let db_path = ":memory:".to_string();
    let storage = Arc::new(
        Storage::new(&db_path)
            .await
            .expect("Failed to create storage"),
    );

    // Health endpoint
    let health_handler = || async {
        (
            axum::http::StatusCode::OK,
            AxumJson(serde_json::json!({ "status": "ok" })),
        )
    };

    // Results list handler
    let get_results_handler = {
        let storage = storage.clone();
        move || async move {
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
    };

    // Single result handler
    let get_result_handler = {
        let storage = storage.clone();
        move |Path(id): Path<i64>| async move {
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
    };

    // Scrape handler
    let scrape_handler = {
        let storage = storage.clone();
        move |Json(payload): Json<ScrapeRequest>| async move {
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
                let (url, status, title, html, links) = {
                    let scraper = Scraper::new();
                    match scraper.scrape(&payload.url).await {
                        Ok(resp) => {
                            let links: Vec<String> =
                                resp.links().iter().map(|s| s.to_string()).collect();
                            (
                                resp.url.clone(),
                                resp.status,
                                resp.title(),
                                resp.html.clone(),
                                links,
                            )
                        }
                        Err(e) => {
                            return (
                                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                                AxumJson(serde_json::json!({"error": e.to_string()})),
                            );
                        }
                    }
                };

                let _ = storage
                    .save_result(&url, status, title.as_deref(), &html, &links)
                    .await;

                let json = serde_json::json!({"url": url, "status": status, "title": title, "links": links});
                (axum::http::StatusCode::OK, AxumJson(json))
            }
        }
    };

    let app = Router::new()
        .route("/health", get(health_handler))
        .route("/results", get(get_results_handler))
        .route("/results/{id}", get(get_result_handler))
        .route("/scrape", post(scrape_handler));

    let addr = format!("{}:{}", host, port);
    println!("Starting API server on {}", addr);
    println!("Endpoints:");
    println!("  POST /scrape     - Scrape a URL (JSON: {{\"url\": \"...\", \"ai\": true/false}}");
    println!("  GET  /results    - List saved results");
    println!("  GET  /results/{{id}} - Get specific result");
    println!("\nServer running at http://{}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
