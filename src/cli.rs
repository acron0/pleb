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
    Watch {
        /// Run as a daemon in the background
        #[arg(long, short)]
        daemon: bool,
    },

    #[command(about = "List active sessions")]
    List,

    #[command(about = "Tail the pleb log file")]
    Log {
        /// Follow the log file (like tail -f)
        #[arg(long, short, default_value = "true")]
        follow: bool,

        /// Number of lines to show
        #[arg(long, short, default_value = "50")]
        lines: usize,
    },

    #[command(about = "Attach to the pleb tmux session")]
    Attach,

    #[command(about = "Transition issue to a new state")]
    Transition {
        /// Issue number
        issue_number: u64,
        /// Target state (ready, provisioning, waiting, working, done, none)
        state: String,
    },

    #[command(about = "Show pleb state for an issue")]
    Status {
        /// Issue number
        issue_number: u64,
    },

    #[command(about = "Hook invoked by Claude Code on events")]
    CcRunHook {
        /// Hook event (stop, user-prompt)
        event: String,
    },

    #[command(about = "Manage Claude Code hooks")]
    Hooks {
        #[command(subcommand)]
        action: HooksAction,
    },

    #[command(about = "Manage configuration")]
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },
}

#[derive(Subcommand, Clone)]
pub enum HooksAction {
    #[command(about = "Generate hooks JSON")]
    Generate,

    #[command(about = "Install hooks to current directory")]
    Install,
}

#[derive(Subcommand)]
pub enum ConfigAction {
    #[command(about = "Show current configuration")]
    Show,

    #[command(about = "Initialize config file from example")]
    Init,
}
