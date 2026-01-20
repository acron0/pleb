use anyhow::{Context, Result};
use std::path::Path;
use tokio::process::Command;

use crate::config::TmuxConfig;

pub struct TmuxManager {
    session_name: String,
    /// Environment variables to pass to tmux sessions (name -> value)
    env_vars: Vec<(String, String)>,
}

impl TmuxManager {
    pub fn new(config: &TmuxConfig) -> Self {
        Self {
            session_name: config.session_name.clone(),
            env_vars: Vec::new(),
        }
    }

    /// Add an environment variable to be passed to tmux sessions
    pub fn with_env(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.env_vars.push((name.into(), value.into()));
        self
    }

    /// Get the session name
    pub fn session_name(&self) -> &str {
        &self.session_name
    }

    /// Ensure the pleb session exists, create if not
    pub async fn ensure_session(&self) -> Result<()> {
        // Check if session exists
        let status = Command::new("tmux")
            .args(["has-session", "-t", &self.session_name])
            .status()
            .await
            .context("Failed to check if tmux session exists")?;

        if !status.success() {
            // Session doesn't exist, create it
            tracing::info!("Creating tmux session: {}", self.session_name);
            Command::new("tmux")
                .args(["new-session", "-d", "-s", &self.session_name])
                .status()
                .await
                .context("Failed to create tmux session")?;
        }

        // Set environment variables for the session (always, to update existing sessions too)
        for (name, value) in &self.env_vars {
            tracing::debug!(
                "Setting tmux environment variable: {}",
                name
            );
            Command::new("tmux")
                .args([
                    "set-environment",
                    "-t",
                    &self.session_name,
                    name,
                    value,
                ])
                .status()
                .await
                .with_context(|| {
                    format!("Failed to set tmux environment variable: {}", name)
                })?;
        }

        Ok(())
    }

    /// Create a new window for an issue in the pleb session
    /// Window name: "issue-{number}"
    /// Working directory: the worktree path
    #[allow(dead_code)]
    pub async fn create_window(&self, issue_number: u64, working_dir: &Path) -> Result<()> {
        // Ensure session exists first
        self.ensure_session().await?;

        let window_name = format!("issue-{}", issue_number);

        // Check if window already exists
        if self.window_exists(issue_number).await? {
            tracing::info!("Window {} already exists", window_name);
            return Ok(());
        }

        // Create the window
        tracing::info!(
            "Creating tmux window {} in session {}",
            window_name,
            self.session_name
        );
        Command::new("tmux")
            .args([
                "new-window",
                "-t",
                &self.session_name,
                "-n",
                &window_name,
                "-c",
                &working_dir.to_string_lossy(),
            ])
            .status()
            .await
            .context("Failed to create tmux window")?;

        Ok(())
    }

    /// Check if a window exists for an issue
    #[allow(dead_code)]
    pub async fn window_exists(&self, issue_number: u64) -> Result<bool> {
        let window_name = format!("issue-{}", issue_number);

        let output = Command::new("tmux")
            .args([
                "list-windows",
                "-t",
                &self.session_name,
                "-F",
                "#{window_name}",
            ])
            .output()
            .await
            .context("Failed to list tmux windows")?;

        if !output.status.success() {
            // Session might not exist yet
            return Ok(false);
        }

        let windows_output = String::from_utf8_lossy(&output.stdout);
        Ok(windows_output.lines().any(|line| line == window_name))
    }

    /// List all issue windows in the session
    pub async fn list_windows(&self) -> Result<Vec<u64>> {
        let output = Command::new("tmux")
            .args([
                "list-windows",
                "-t",
                &self.session_name,
                "-F",
                "#{window_name}",
            ])
            .output()
            .await
            .context("Failed to list tmux windows")?;

        if !output.status.success() {
            // Session doesn't exist, return empty list
            return Ok(Vec::new());
        }

        let windows_output = String::from_utf8_lossy(&output.stdout);
        let mut issue_numbers = Vec::new();

        for line in windows_output.lines() {
            // Parse "issue-{number}" format
            if let Some(num_str) = line.strip_prefix("issue-") {
                if let Ok(number) = num_str.parse::<u64>() {
                    issue_numbers.push(number);
                }
            }
        }

        Ok(issue_numbers)
    }

    /// Kill a window for an issue
    #[allow(dead_code)]
    pub async fn kill_window(&self, issue_number: u64) -> Result<()> {
        let window_name = format!("issue-{}", issue_number);
        let target = format!("{}:{}", self.session_name, window_name);

        tracing::info!("Killing tmux window: {}", target);
        let status = Command::new("tmux")
            .args(["kill-window", "-t", &target])
            .status()
            .await
            .context("Failed to kill tmux window")?;

        if !status.success() {
            tracing::warn!("Window {} may not exist or was already killed", target);
        }

        Ok(())
    }

    /// Send keys to a window (for starting Claude, etc.)
    pub async fn send_keys(&self, issue_number: u64, keys: &str) -> Result<()> {
        let window_name = format!("issue-{}", issue_number);
        let target = format!("{}:{}", self.session_name, window_name);

        tracing::debug!("Sending keys to {}: {}", target, keys);
        Command::new("tmux")
            .args(["send-keys", "-t", &target, keys, "Enter"])
            .status()
            .await
            .context("Failed to send keys to tmux window")?;

        Ok(())
    }

    /// Rename a window to include state indicator (e.g., "issue-42-waiting")
    pub async fn rename_window(&self, issue_number: u64, state: &str) -> Result<()> {
        // Target the window by its current name pattern (issue-N or issue-N-*)
        let target = format!("{}:issue-{}", self.session_name, issue_number);
        let new_name = format!("issue-{}-{}", issue_number, state);

        tracing::debug!("Renaming window {} to {}", target, new_name);
        let status = Command::new("tmux")
            .args(["rename-window", "-t", &target, &new_name])
            .status()
            .await
            .context("Failed to rename tmux window")?;

        if !status.success() {
            tracing::warn!("Failed to rename window to {}", new_name);
        }

        Ok(())
    }

    /// Attach to the pleb session (blocking - replaces current terminal)
    /// This returns a std::process::Command that the caller can exec() or status()
    pub fn attach_command(&self) -> std::process::Command {
        let mut cmd = std::process::Command::new("tmux");
        cmd.args(["attach", "-t", &self.session_name]);
        cmd
    }
}
