use clap::{Parser, Subcommand};
use scrapio_runtime::Runtime;

mod commands;
mod server;

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
        Commands::Classic { url } => commands::handle_classic(&url),
        Commands::Ai { url, schema, provider, model } => {
            commands::handle_ai(&url, schema, &provider, &model);
        }
        Commands::Crawl { url, depth } => commands::handle_crawl(&url, depth),
        Commands::Save { url, database } => commands::handle_save(&url, &database),
        Commands::List { database, limit } => commands::handle_list(&database, limit),
        Commands::Serve { host, port } => {
            let runtime = scrapio_runtime::TokioRuntime::default();
            runtime.block_on(async {
                server::serve_api_server(host, port).await;
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

