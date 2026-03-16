use clap::{Parser, Subcommand};
use scrapio_runtime::Runtime;

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
    },
    Version,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
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
            );
        }
        Commands::Crawl { url, depth } => commands::handle_crawl(&url, depth),
        Commands::Save { url, database } => commands::handle_save(&url, &database),
        Commands::List { database, limit } => commands::handle_list(&database, limit),
        Commands::Serve { host, port } => {
            let runtime = scrapio_runtime::TokioRuntime::default();
            if let Err(e) = runtime.block_on(async {
                server::serve_api_server(host, port).await
            }) {
                eprintln!("Server error: {}", e);
            }
        }
        Commands::Browser {
            url,
            headless,
            stealth,
            script,
            driver_path,
        } => {
            commands::handle_browser(
                &url,
                headless,
                stealth.as_deref(),
                script.as_deref(),
                driver_path.as_deref(),
            );
        }
        Commands::Version => {
            println!("scrapio v{}", env!("CARGO_PKG_VERSION"));
            println!("Runtime: Tokio");
            println!("Features: classic, ai, storage, browser");
        }
    }

    Ok(())
}
