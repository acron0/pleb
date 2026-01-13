mod cli;
mod config;
mod github;

use anyhow::{Context, Result};
use clap::Parser;
use std::path::Path;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use cli::{Cli, Commands, ConfigAction};
use config::Config;

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

    match &cli.command {
        Commands::Config { action } => {
            handle_config_command(action)?;
        }
        _ => {
            // For all other commands, load and validate config
            let config = load_config(&cli.config)?;
            handle_command(cli.command, config).await?;
        }
    }

    Ok(())
}

fn handle_config_command(action: &ConfigAction) -> Result<()> {
    match action {
        ConfigAction::Show => {
            let config = Config::load_default().context(
                "Failed to load config. Run 'pleb config init' to create pleb.toml from example.",
            )?;
            config.validate()?;

            // Pretty print the config
            let toml_str = toml::to_string_pretty(&config)
                .context("Failed to serialize config to TOML")?;
            println!("{}", toml_str);
        }
        ConfigAction::Init => {
            let target_path = Path::new("pleb.toml");

            if target_path.exists() {
                anyhow::bail!(
                    "pleb.toml already exists. Delete it first if you want to reinitialize."
                );
            }

            std::fs::copy("pleb.example.toml", target_path).context(
                "Failed to copy pleb.example.toml to pleb.toml. Make sure pleb.example.toml exists.",
            )?;

            println!("Created pleb.toml from pleb.example.toml");
            println!("Edit pleb.toml to configure for your repository.");
        }
    }

    Ok(())
}

fn load_config(path: &str) -> Result<Config> {
    let config_path = Path::new(path);

    let config = Config::load(config_path).with_context(|| {
        format!(
            "Failed to load config from {}. Run 'pleb config init' to create pleb.toml from example.",
            path
        )
    })?;

    config.validate().context("Config validation failed")?;

    Ok(config)
}

async fn handle_command(command: Commands, config: Config) -> Result<()> {
    match command {
        Commands::Watch => {
            tracing::info!("Starting watch mode with repo: {}/{}", config.github.owner, config.github.repo);
            println!("Not yet implemented: watch");
        }
        Commands::List => {
            println!("Not yet implemented: list");
        }
        Commands::Attach { session } => {
            println!("Not yet implemented: attach {}", session);
        }
        Commands::Config { .. } => {
            // Already handled above, shouldn't reach here
            unreachable!("Config command should be handled before this point");
        }
    }

    Ok(())
}
