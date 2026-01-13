use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "pleb")]
#[command(about = "Issue-driven Claude Code orchestrator", long_about = None)]
pub struct Cli {
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
}
