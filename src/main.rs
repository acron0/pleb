mod claude;
mod cli;
mod config;
mod github;
mod state;
mod templates;
mod tmux;
mod worktree;

use anyhow::{Context, Result};
use clap::Parser;
use std::path::Path;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use cli::{Cli, Commands, ConfigAction};
use config::Config;
use tmux::TmuxManager;
use github::GitHubClient;
use worktree::WorktreeManager;
use claude::ClaudeRunner;
use templates::{TemplateEngine, IssueContext};
use state::{IssueTracker, PlebState};

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

/// Orchestrator that manages the main daemon loop
struct Orchestrator {
    github: GitHubClient,
    worktree: WorktreeManager,
    tmux: TmuxManager,
    claude: ClaudeRunner,
    templates: TemplateEngine,
    tracker: IssueTracker,
    config: Config,
}

impl Orchestrator {
    async fn new(config: Config) -> Result<Self> {
        let github = GitHubClient::new(&config.github).await?;
        let worktree = WorktreeManager::new(&config.paths);
        let tmux = TmuxManager::new(&config.tmux);
        let claude = ClaudeRunner::new(&config.claude, &config.tmux);
        let templates = TemplateEngine::new(&config.prompts)?;
        let tracker = IssueTracker::new();

        Ok(Self {
            github,
            worktree,
            tmux,
            claude,
            templates,
            tracker,
            config,
        })
    }

    async fn run(&mut self) -> Result<()> {
        // Verify GitHub connection
        tracing::info!("Verifying GitHub connection...");
        self.github.verify_connection().await?;

        // Ensure repo is cloned
        tracing::info!("Ensuring repository is cloned...");
        self.worktree
            .ensure_repo(&self.config.github.owner, &self.config.github.repo)
            .await?;

        // Load the new_issue template
        tracing::info!("Loading templates...");
        self.templates
            .load_template(&self.config.prompts.new_issue)?;

        // Display startup banner
        println!(
            "Pleb daemon started - watching {}/{} for issues with label '{}'",
            self.config.github.owner, self.config.github.repo, self.config.labels.ready
        );

        // Enter polling loop
        let poll_interval = std::time::Duration::from_secs(self.config.watch.poll_interval_secs);

        loop {
            // Check for Ctrl+C signal
            tokio::select! {
                _ = tokio::signal::ctrl_c() => {
                    tracing::info!("Shutting down...");
                    break;
                }
                _ = self.poll_cycle() => {
                    // Continue to next cycle
                }
            }

            tokio::time::sleep(poll_interval).await;
        }

        Ok(())
    }

    async fn poll_cycle(&mut self) -> Result<()> {
        tracing::debug!("Polling for new issues...");

        // Fetch issues with pleb:ready label
        let issues = match self
            .github
            .get_issues_with_label(&self.config.labels.ready)
            .await
        {
            Ok(issues) => issues,
            Err(e) => {
                // Network errors shouldn't crash the daemon
                tracing::error!("Failed to fetch issues: {}. Will retry on next poll.", e);
                return Ok(());
            }
        };

        if issues.is_empty() {
            tracing::debug!(
                "No new issues with {} label",
                self.config.labels.ready
            );
            return Ok(());
        }

        // Process each new issue not in tracker
        let mut processed_count = 0;
        for issue in issues {
            if self.tracker.get(issue.number).is_some() {
                // Already being tracked, skip
                tracing::debug!("Skipping issue #{} (already being tracked)", issue.number);
                continue;
            }

            // Process this new issue
            if let Err(e) = self.process_issue(&issue).await {
                tracing::error!("Failed to process issue #{}: {}", issue.number, e);
                // Continue with other issues - don't crash the daemon
            } else {
                processed_count += 1;
            }
        }

        if processed_count > 0 {
            tracing::info!("Provisioned {} new issue(s)", processed_count);
        }

        Ok(())
    }

    async fn process_issue(&mut self, issue: &github::Issue) -> Result<()> {
        tracing::info!("Processing issue #{}: {}", issue.number, issue.title);

        // Transition label: ready -> provisioning
        self.github
            .transition_state(
                issue.number,
                PlebState::Ready,
                PlebState::Provisioning,
                &self.config.labels,
            )
            .await?;

        // Create worktree
        let worktree_path = self.worktree.create_worktree(issue.number).await?;

        // Create tmux window
        self.tmux.create_window(issue.number, &worktree_path).await?;

        // Render prompt with issue context
        let branch_name = format!("pleb/issue-{}", issue.number);
        let context = IssueContext::from_issue(issue, &branch_name, &worktree_path);
        let prompt = self
            .templates
            .render(&self.config.prompts.new_issue, &context)?;

        // Invoke Claude
        self.claude.invoke(issue.number, &prompt).await?;

        // Transition label: provisioning -> working
        self.github
            .transition_state(
                issue.number,
                PlebState::Provisioning,
                PlebState::Working,
                &self.config.labels,
            )
            .await?;

        // Track issue in IssueTracker with Working state
        self.tracker.track(issue.number, PlebState::Working);
        if let Err(e) = self.tracker.set_worktree_path(issue.number, worktree_path) {
            tracing::warn!("Failed to set worktree path for issue #{}: {}", issue.number, e);
        }

        tracing::info!(
            "Successfully provisioned issue #{}: {}",
            issue.number,
            issue.title
        );

        Ok(())
    }
}

async fn handle_command(command: Commands, config: Config) -> Result<()> {
    match command {
        Commands::Watch => {
            let mut orchestrator = Orchestrator::new(config).await?;
            orchestrator.run().await?;
        }
        Commands::List => {
            let tmux_manager = TmuxManager::new(&config.tmux);
            let issue_numbers = tmux_manager.list_windows().await.context("Failed to list issue windows")?;

            if issue_numbers.is_empty() {
                println!("No active issue windows in session '{}'", config.tmux.session_name);
            } else {
                println!("Active issue windows in session '{}':", config.tmux.session_name);
                for issue_number in issue_numbers {
                    println!("  - issue-{}", issue_number);
                }
            }
        }
        Commands::Attach => {
            let tmux_manager = TmuxManager::new(&config.tmux);

            // Ensure the session exists before attaching
            tmux_manager.ensure_session().await.context("Failed to ensure tmux session exists")?;

            // Get the attach command and execute it
            // This will replace the current process with tmux attach
            let status = tmux_manager.attach_command()
                .status()
                .context("Failed to attach to tmux session")?;

            if !status.success() {
                anyhow::bail!("Failed to attach to session '{}'", config.tmux.session_name);
            }
        }
        Commands::Config { .. } => {
            // Already handled above, shouldn't reach here
            unreachable!("Config command should be handled before this point");
        }
    }

    Ok(())
}
