use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "pleb")]
#[command(about = "Issue-driven Claude Code orchestrator", long_about = None)]
pub struct Cli {
    /// Path to configuration file
    #[arg(long, default_value = "pleb.toml", global = true)]
    pub config: String,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    #[command(about = "Start watching for issues")]
    Watch,

    #[command(about = "List active sessions")]
    List,

    #[command(about = "Attach to a tmux session")]
    Attach {
        #[arg(help = "Session name to attach to")]
        session: String,
    },

    #[command(about = "Manage configuration")]
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },
}

#[derive(Subcommand)]
pub enum ConfigAction {
    #[command(about = "Show current configuration")]
    Show,

    #[command(about = "Initialize config file from example")]
    Init,
}
