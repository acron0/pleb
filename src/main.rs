mod claude;
mod cli;
mod commands;
mod config;
mod github;
mod hooks;
mod state;
mod templates;
mod tmux;
mod worktree;

use anyhow::{Context, Result};
use clap::Parser;
use std::collections::HashSet;
use std::path::Path;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use cli::{Cli, Commands, ConfigAction, HooksAction};
use config::Config;
use tmux::TmuxManager;
use github::GitHubClient;
use worktree::WorktreeManager;
use claude::ClaudeRunner;
use templates::{TemplateEngine, IssueContext};
use state::PlebState;

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Handle daemon mode specially - must fork BEFORE creating tokio runtime
    if let Commands::Watch { daemon: true } = &cli.command {
        let config = load_config(&cli.config)?;
        return run_daemon_mode(config);
    }

    // Initialize tracing for non-daemon modes
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "pleb=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Create tokio runtime for async operations
    let runtime = tokio::runtime::Runtime::new().context("Failed to create tokio runtime")?;

    match &cli.command {
        Commands::Config { action } => {
            handle_config_command(action)?;
        }
        Commands::Hooks { action } => {
            // Hooks commands don't need config
            handle_hooks_command(action.clone())?;
        }
        _ => {
            // For all other commands, load and validate config
            let config = load_config(&cli.config)?;
            runtime.block_on(handle_command(cli.command, config))?;
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
/// State is derived from GitHub labels - minimal in-memory tracking
struct Orchestrator {
    github: GitHubClient,
    worktree: WorktreeManager,
    tmux: TmuxManager,
    claude: ClaudeRunner,
    templates: TemplateEngine,
    config: Config,
    /// Track issues we've already logged as "skipping" to avoid log spam
    logged_skips: HashSet<u64>,
}

impl Orchestrator {
    async fn new(config: Config) -> Result<Self> {
        let github = GitHubClient::new(&config.github).await?;
        let worktree = WorktreeManager::new(&config.paths);
        let tmux = TmuxManager::new(&config.tmux);
        let claude = ClaudeRunner::new(&config.claude, &config.tmux);
        let templates = TemplateEngine::new(&config.prompts)?;

        Ok(Self {
            github,
            worktree,
            tmux,
            claude,
            templates,
            config,
            logged_skips: HashSet::new(),
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

        // Create ctrl_c future once, outside the loop
        let ctrl_c = tokio::signal::ctrl_c();
        tokio::pin!(ctrl_c);

        loop {
            tokio::select! {
                biased;

                _ = &mut ctrl_c => {
                    tracing::info!("Shutting down...");
                    break;
                }
                _ = async {
                    if let Err(e) = self.poll_cycle().await {
                        tracing::error!("Poll cycle error: {}", e);
                    }
                    tokio::time::sleep(poll_interval).await;
                } => {
                    // Continue to next cycle
                }
            }
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
            // Clear logged_skips since no issues are in ready state
            self.logged_skips.clear();
            return Ok(());
        }

        // Collect current issue numbers for cleanup
        let current_issue_numbers: HashSet<u64> = issues.iter().map(|i| i.number).collect();

        // Clean up logged_skips: remove issues no longer in ready state
        self.logged_skips.retain(|n| current_issue_numbers.contains(n));

        // Process each issue that doesn't already have a tmux window
        let mut processed_count = 0;
        for issue in issues {
            // Check if tmux window already exists (idempotent check)
            if self.tmux.window_exists(issue.number).await? {
                // Only log skip once per issue
                if !self.logged_skips.contains(&issue.number) {
                    tracing::info!("Issue #{} already has tmux window, skipping", issue.number);
                    self.logged_skips.insert(issue.number);
                }
                continue;
            }

            // Issue is being processed, remove from logged_skips if present
            self.logged_skips.remove(&issue.number);

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

        // Install Claude Code hooks in worktree
        if let Err(e) = hooks::install_hooks(&worktree_path) {
            tracing::warn!(
                "Failed to install hooks for issue #{}: {}",
                issue.number,
                e
            );
            // Continue anyway - hooks are nice to have but not critical
        } else {
            tracing::info!("Installed Claude Code hooks for issue #{}", issue.number);
        }

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

        tracing::info!(
            "Successfully provisioned issue #{}: {}",
            issue.number,
            issue.title
        );

        Ok(())
    }
}

async fn handle_transition_command(
    issue_number: u64,
    state_str: &str,
    config: Config,
) -> Result<()> {
    // Create GitHub client
    let github = GitHubClient::new(&config.github).await?;

    // Fetch the issue to determine current state
    let issue = github.get_issue(issue_number).await?;
    let current_state = github.get_pleb_state(&issue, &config.labels);

    // Handle "none" as a special case to remove all pleb labels
    if state_str.to_lowercase() == "none" {
        // Remove all pleb labels
        let all_labels = vec![
            &config.labels.ready,
            &config.labels.provisioning,
            &config.labels.waiting,
            &config.labels.working,
            &config.labels.done,
        ];

        for label in all_labels {
            github.remove_label(issue_number, label).await?;
        }

        println!("Issue #{} is no longer managed by pleb (all pleb labels removed)", issue_number);
        return Ok(());
    }

    // Parse state string
    let target_state = parse_state(state_str)?;

    // Transition to target state
    if let Some(from_state) = current_state {
        github
            .transition_state(issue_number, from_state, target_state, &config.labels)
            .await?;
    } else {
        // No current pleb label - just add the target state label
        let target_label = match target_state {
            PlebState::Ready => &config.labels.ready,
            PlebState::Provisioning => &config.labels.provisioning,
            PlebState::Waiting => &config.labels.waiting,
            PlebState::Working => &config.labels.working,
            PlebState::Done => &config.labels.done,
        };
        github.add_label(issue_number, target_label).await?;
    }

    println!("Issue #{} transitioned to {:?}", issue_number, target_state);

    Ok(())
}

async fn handle_status_command(issue_number: u64, config: Config) -> Result<()> {
    // Create GitHub client
    let github = GitHubClient::new(&config.github).await?;

    // Fetch the issue
    let issue = github.get_issue(issue_number).await?;

    // Determine current pleb state
    let current_state = github.get_pleb_state(&issue, &config.labels);

    // Print formatted status
    println!("Issue #{}: {}", issue.number, issue.title);

    match current_state {
        Some(state) => {
            let state_name = match state {
                PlebState::Ready => "ready",
                PlebState::Provisioning => "provisioning",
                PlebState::Waiting => "waiting",
                PlebState::Working => "working",
                PlebState::Done => "done",
            };
            println!("State: {}", state_name);
        }
        None => {
            println!("State: not managed by pleb");
        }
    }

    println!("URL: {}", issue.html_url);

    Ok(())
}

async fn handle_cc_run_hook_command(event: &str, config: Config) -> Result<()> {
    // Read JSON from stdin
    use std::io::Read;
    let mut stdin_content = String::new();
    std::io::stdin()
        .read_to_string(&mut stdin_content)
        .context("Failed to read from stdin")?;

    // Parse JSON to extract cwd
    let json: serde_json::Value =
        serde_json::from_str(&stdin_content).context("Failed to parse JSON from stdin")?;

    let cwd = json["cwd"]
        .as_str()
        .context("Missing or invalid 'cwd' field in hook payload")?;

    // Extract issue number from path
    let issue_number = match hooks::extract_issue_number_from_path(cwd) {
        Some(num) => num,
        None => {
            // Not a pleb-managed directory, exit silently
            tracing::debug!("No issue number found in path: {}", cwd);
            return Ok(());
        }
    };

    // Map event to target state
    let target_state = match event {
        "stop" => PlebState::Waiting,
        "user-prompt" => PlebState::Working,
        _ => {
            tracing::warn!("Unknown hook event: {}", event);
            return Ok(());
        }
    };

    // Create GitHub client and transition
    let github = GitHubClient::new(&config.github).await?;
    let issue = github.get_issue(issue_number).await?;
    let current_state = github.get_pleb_state(&issue, &config.labels);

    if let Some(from_state) = current_state {
        github
            .transition_state(issue_number, from_state, target_state, &config.labels)
            .await?;
        tracing::info!(
            "Hook '{}' transitioned issue #{} to {:?}",
            event,
            issue_number,
            target_state
        );
    }

    Ok(())
}

fn handle_hooks_command(action: HooksAction) -> Result<()> {
    match action {
        HooksAction::Generate => {
            let json = hooks::generate_hooks_json()?;
            println!("{}", json);
        }
        HooksAction::Install => {
            let current_dir = std::env::current_dir().context("Failed to get current directory")?;
            hooks::install_hooks(&current_dir)?;
            println!("Hooks installed to .claude/settings.json");
        }
    }

    Ok(())
}

fn parse_state(state_str: &str) -> Result<PlebState> {
    match state_str.to_lowercase().as_str() {
        "ready" => Ok(PlebState::Ready),
        "provisioning" => Ok(PlebState::Provisioning),
        "waiting" => Ok(PlebState::Waiting),
        "working" => Ok(PlebState::Working),
        "done" => Ok(PlebState::Done),
        _ => anyhow::bail!(
            "Invalid state '{}'. Valid states: ready, provisioning, waiting, working, done",
            state_str
        ),
    }
}

fn handle_log_command(follow: bool, lines: usize, config: Config) -> Result<()> {
    use std::process::Command;

    let log_file_path = config.log_file()?;

    // Check if log file exists
    if !log_file_path.exists() {
        anyhow::bail!(
            "No log file found. Is the daemon running? Expected: {}",
            log_file_path.display()
        );
    }

    // Build tail command
    let mut cmd = Command::new("tail");

    if follow {
        cmd.arg("-f");
    }

    cmd.arg("-n").arg(lines.to_string());
    cmd.arg(&log_file_path);

    // Execute tail - replace current process on Unix, or just run it on other platforms
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        let err = cmd.exec();
        // exec only returns if there's an error
        Err(anyhow::anyhow!("Failed to exec tail: {}", err))
    }

    #[cfg(not(unix))]
    {
        let status = cmd.status().context("Failed to run tail command")?;
        if !status.success() {
            anyhow::bail!("tail command failed with status: {}", status);
        }
        Ok(())
    }
}

fn handle_stop_command(config: Config) -> Result<()> {
    let pid_file_path = config.pid_file()?;

    // Check if PID file exists
    if !pid_file_path.exists() {
        anyhow::bail!(
            "No PID file found. Is the daemon running? Expected: {}",
            pid_file_path.display()
        );
    }

    // Read PID from file
    let pid_str = std::fs::read_to_string(&pid_file_path)
        .with_context(|| format!("Failed to read PID file: {}", pid_file_path.display()))?;
    let pid: i32 = pid_str
        .trim()
        .parse()
        .with_context(|| format!("Invalid PID in file: {}", pid_str.trim()))?;

    // Send SIGTERM to the process
    #[cfg(unix)]
    {
        use nix::sys::signal::{kill, Signal};
        use nix::unistd::Pid;

        match kill(Pid::from_raw(pid), Signal::SIGTERM) {
            Ok(_) => {
                println!("Sent SIGTERM to daemon (PID: {})", pid);
                // Remove PID file
                let _ = std::fs::remove_file(&pid_file_path);
                println!("Daemon stopped.");
            }
            Err(nix::errno::Errno::ESRCH) => {
                // Process doesn't exist, clean up stale PID file
                let _ = std::fs::remove_file(&pid_file_path);
                println!("Daemon was not running (stale PID file removed).");
            }
            Err(e) => {
                anyhow::bail!("Failed to send signal to daemon: {}", e);
            }
        }
    }

    #[cfg(not(unix))]
    {
        anyhow::bail!("Stop command is only supported on Unix systems");
    }

    Ok(())
}

fn run_daemon_mode(config: Config) -> Result<()> {
    use daemonize::Daemonize;
    use std::fs;

    // Ensure daemon directory exists
    let daemon_dir = config.daemon_dir()?;
    fs::create_dir_all(&daemon_dir)
        .with_context(|| format!("Failed to create daemon directory: {}", daemon_dir.display()))?;

    let log_file_path = config.log_file()?;
    let pid_file_path = config.pid_file()?;

    // Check for existing daemon
    if pid_file_path.exists() {
        let pid_str = fs::read_to_string(&pid_file_path)
            .with_context(|| format!("Failed to read PID file: {}", pid_file_path.display()))?;
        if let Ok(pid) = pid_str.trim().parse::<i32>() {
            // Check if process is still running
            #[cfg(unix)]
            {
                use nix::sys::signal::kill;
                use nix::unistd::Pid;

                match kill(Pid::from_raw(pid), None) {
                    Ok(_) => {
                        // Process exists
                        anyhow::bail!(
                            "Daemon already running (PID: {}). Use 'pleb stop' first.",
                            pid
                        );
                    }
                    Err(nix::errno::Errno::ESRCH) => {
                        // Process doesn't exist, stale PID file
                        println!("Removing stale PID file (process {} not found)", pid);
                        let _ = fs::remove_file(&pid_file_path);
                    }
                    Err(e) => {
                        anyhow::bail!("Failed to check if daemon is running: {}", e);
                    }
                }
            }
        }
    }

    // Print info before daemonizing (so user sees it)
    println!("Starting daemon...");
    println!("Log file: {}", log_file_path.display());
    println!("PID file: {}", pid_file_path.display());

    // Create log file appender
    let log_file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_file_path)
        .with_context(|| format!("Failed to open log file: {}", log_file_path.display()))?;

    // Configure daemonize - keep original working directory for relative paths in config
    let current_dir = std::env::current_dir().context("Failed to get current directory")?;
    let daemonize = Daemonize::new()
        .pid_file(&pid_file_path)
        .working_directory(current_dir)
        .stdout(log_file.try_clone()?)
        .stderr(log_file);

    // Fork into background - BEFORE creating tokio runtime
    daemonize.start().context("Failed to daemonize")?;

    // After this point, we're in the daemon process
    // Set up tracing to write to the log file
    let log_file_for_tracing = config.log_file()?;
    let file_appender = tracing_appender::rolling::never(
        log_file_for_tracing.parent().unwrap(),
        log_file_for_tracing.file_name().unwrap(),
    );

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "pleb=info".into()),
        )
        .with(tracing_subscriber::fmt::layer().with_writer(file_appender))
        .init();

    tracing::info!("========================================");
    tracing::info!("Daemon started with PID: {}", std::process::id());

    // NOW create tokio runtime (after fork)
    let runtime = tokio::runtime::Runtime::new().context("Failed to create tokio runtime")?;

    // Run the orchestrator
    runtime.block_on(async {
        let mut orchestrator = Orchestrator::new(config).await?;
        orchestrator.run().await
    })
}

async fn handle_command(command: Commands, config: Config) -> Result<()> {
    match command {
        Commands::Watch { daemon: _ } => {
            // Daemon mode is handled before tokio runtime is created
            // This branch is only reached for non-daemon watch
            let mut orchestrator = Orchestrator::new(config).await?;
            orchestrator.run().await?;
        }
        Commands::Log { follow, lines } => {
            handle_log_command(follow, lines, config)?;
        }
        Commands::Stop => {
            handle_stop_command(config)?;
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
        Commands::Transition {
            issue_number,
            state,
        } => {
            handle_transition_command(issue_number, &state, config).await?;
        }
        Commands::CcRunHook { event } => {
            handle_cc_run_hook_command(&event, config).await?;
        }
        Commands::Status { issue_number } => {
            handle_status_command(issue_number, config).await?;
        }
        Commands::Hooks { action } => {
            handle_hooks_command(action)?;
        }
        Commands::Config { .. } => {
            // Already handled above, shouldn't reach here
            unreachable!("Config command should be handled before this point");
        }
    }

    Ok(())
}
