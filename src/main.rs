mod cli;

use anyhow::Result;
use clap::Parser;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use cli::{Cli, Commands};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "pleb=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Watch => {
            println!("Not yet implemented: watch");
        }
        Commands::List => {
            println!("Not yet implemented: list");
        }
        Commands::Attach { session } => {
            println!("Not yet implemented: attach {}", session);
        }
    }

    Ok(())
}
