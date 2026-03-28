use clap::{Parser, Subcommand};
use scrapio_runtime::Runtime;
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

mod commands;
mod server;
mod swagger;

#[derive(Parser)]
#[command(name = "scrapio")]
#[command(about = "All-in-one web scraping toolkit", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    #[arg(long, default_value = "info")]
    log_level: String,
}

fn init_tracing() {
    tracing_subscriber::registry()
        .with(fmt::layer().with_target(true).with_line_number(true))
        .with(EnvFilter::from_default_env())
        .init();
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
        #[arg(long)]
        browser: bool,
        #[arg(long, default_value = "")]
        prompt: String,
        #[arg(long, default_value = "10")]
        max_steps: usize,
        #[arg(long)]
        driver_path: Option<String>,
        #[arg(long, default_value_t = true, action = clap::ArgAction::Set)]
        headless: bool,
        #[arg(long, short)]
        verbose: bool,
        #[arg(long, default_value = "text")]
        output: String,
        #[arg(long)]
        output_file: Option<String>,
    },
    Crawl {
        url: String,
        #[arg(long, default_value = "2")]
        depth: usize,
        #[arg(long)]
        max_pages: Option<usize>,
        #[arg(long)]
        scope: Option<String>,
        #[arg(long)]
        extract: bool,
        #[arg(long)]
        schema: Option<String>,
        #[arg(long, default_value = "openai")]
        provider: String,
        #[arg(long, default_value = "")]
        model: String,
        #[arg(long, default_value = "auto")]
        browser_escalation: String,
        #[arg(long)]
        sitemap: bool,
        #[arg(long)]
        robots: bool,
        #[arg(long, default_value_t = true)]
        respect_robotstxt: bool,
        #[arg(long)]
        unsafe_mode: bool,
        #[arg(long, default_value = "scrapio.db")]
        store: String,
        #[arg(long)]
        no_store: bool,
        #[arg(long)]
        network: bool,
        /// Single proxy URL (e.g., http://user:pass@proxy:8080)
        #[arg(long)]
        proxy: Option<String>,
        /// Path to file with proxy list (one per line)
        #[arg(long)]
        proxy_list: Option<String>,
        /// Proxy rotation strategy (round-robin, random, per-domain)
        #[arg(long, default_value = "round-robin")]
        proxy_rotation: String,
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
        #[arg(long, default_value = "text")]
        output: String,
    },
    Serve {
        #[arg(long, default_value = "127.0.0.1")]
        host: String,
        #[arg(long, default_value = "8080")]
        port: u16,
    },
    Browser {
        url: String,
        #[arg(long, default_value_t = true, action = clap::ArgAction::Set)]
        headless: bool,
        #[arg(long)]
        stealth: Option<String>,
        #[arg(long)]
        script: Option<String>,
        #[arg(long)]
        driver_path: Option<String>,
        #[arg(long, default_value = "chrome")]
        browser: String,
        /// Proxy URL (e.g., http://user:pass@proxy:8080)
        #[arg(long)]
        proxy: Option<String>,
    },
    /// Start the MCP server for AI client integration
    Mcp {
        /// Use HTTP transport instead of stdio
        #[arg(long, default_value_t = false)]
        http: bool,
        /// Host to bind to (only for HTTP mode)
        #[arg(long, default_value = "127.0.0.1")]
        host: String,
        /// Port to bind to (only for HTTP mode)
        #[arg(long, default_value = "8080")]
        port: u16,
    },
    Version,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_tracing();
    let cli = Cli::parse();

    match cli.command {
        Commands::Classic { url } => commands::handle_classic(&url),
        Commands::Ai {
            url,
            schema,
            provider,
            model,
            browser,
            prompt,
            max_steps,
            driver_path,
            headless,
            verbose,
            output,
            output_file,
        } => {
            commands::handle_ai(
                &url,
                schema,
                &provider,
                &model,
                browser,
                &prompt,
                max_steps,
                driver_path.as_deref(),
                headless,
                verbose,
                &output,
                output_file.as_deref(),
            );
        }
        Commands::Crawl {
            url,
            depth,
            max_pages,
            scope,
            extract,
            schema,
            provider,
            model,
            browser_escalation,
            sitemap,
            robots,
            respect_robotstxt,
            unsafe_mode,
            store,
            no_store,
            network,
            proxy,
            proxy_list,
            proxy_rotation,
        } => commands::handle_crawl(
            &url,
            depth,
            max_pages,
            scope.as_deref(),
            extract,
            schema.as_deref(),
            &provider,
            &model,
            &browser_escalation,
            sitemap,
            robots,
            respect_robotstxt,
            unsafe_mode,
            &store,
            no_store,
            network,
            proxy.as_deref(),
            proxy_list.as_deref(),
            &proxy_rotation,
        ),
        Commands::Save { url, database } => commands::handle_save(&url, &database),
        Commands::List {
            database,
            limit,
            output,
        } => commands::handle_list(&database, limit, &output),
        Commands::Serve { host, port } => {
            let runtime = scrapio_runtime::TokioRuntime::default();
            if let Err(e) = runtime.block_on(async { server::serve_api_server(host, port).await }) {
                eprintln!("Server error: {}", e);
            }
        }
        Commands::Browser {
            url,
            headless,
            stealth,
            script,
            driver_path,
            browser,
            proxy,
        } => {
            commands::handle_browser(
                &url,
                headless,
                stealth.as_deref(),
                script.as_deref(),
                driver_path.as_deref(),
                &browser,
                proxy.as_deref(),
            );
        }
        Commands::Mcp { http, host, port } => {
            let runtime = scrapio_runtime::TokioRuntime::default();
            if http {
                if let Err(e) =
                    runtime.block_on(async { scrapio_mcp::run_mcp_http_server(host, port).await })
                {
                    eprintln!("MCP HTTP server error: {}", e);
                }
            } else {
                if let Err(e) = runtime.block_on(async { scrapio_mcp::run_mcp_server().await }) {
                    eprintln!("MCP server error: {}", e);
                }
            }
        }
        Commands::Version => {
            println!("scrapio v{}", env!("CARGO_PKG_VERSION"));
            println!("Runtime: Tokio");
            println!("Features: classic, ai, storage, browser");
        }
    }

    Ok(())
}
