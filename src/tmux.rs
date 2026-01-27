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
            let create_status = Command::new("tmux")
                .args(["new-session", "-d", "-s", &self.session_name])
                .status()
                .await
                .context("Failed to execute tmux new-session command")?;

            if !create_status.success() {
                anyhow::bail!("tmux new-session command failed for session '{}': {}", self.session_name, create_status);
            }
        }

        // Set environment variables for the session (always, to update existing sessions too)
        for (name, value) in &self.env_vars {
            tracing::debug!(
                "Setting tmux environment variable: {}",
                name
            );
            let env_status = Command::new("tmux")
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
                    format!("Failed to execute tmux set-environment command for variable: {}", name)
                })?;

            if !env_status.success() {
                anyhow::bail!("tmux set-environment command failed for variable '{}': {}", name, env_status);
            }
        }

        Ok(())
    }

    /// Get the next available window index in the session
    async fn next_available_window_index(&self) -> Result<u32> {
        let output = Command::new("tmux")
            .args([
                "list-windows",
                "-t",
                &self.session_name,
                "-F",
                "#{window_index}",
            ])
            .output()
            .await
            .context("Failed to list tmux windows")?;

        if !output.status.success() {
            // Session doesn't exist yet, start at index 0
            return Ok(0);
        }

        let windows_output = String::from_utf8_lossy(&output.stdout);
        let mut indices: Vec<u32> = windows_output
            .lines()
            .filter_map(|line| line.parse().ok())
            .collect();

        if indices.is_empty() {
            return Ok(0);
        }

        indices.sort_unstable();

        // Find the first gap or return max + 1
        for (i, &index) in indices.iter().enumerate() {
            if index != i as u32 {
                return Ok(i as u32);
            }
        }

        Ok(indices.len() as u32)
    }

    /// Create a new window for an issue in the pleb session
    /// Window name: "{branch_name}" (e.g., "2592-add-invoices-table_acron_pleb")
    /// Working directory: the worktree path
    #[allow(dead_code)]
    pub async fn create_window(&self, branch_name: &str, working_dir: &Path) -> Result<()> {
        // Ensure session exists first
        self.ensure_session().await?;

        let window_name = branch_name.to_string();

        // Extract issue number from branch name (first part before '-')
        let issue_number = branch_name
            .split('-')
            .next()
            .and_then(|s| s.parse::<u64>().ok())
            .with_context(|| format!("Failed to extract issue number from branch name: {}", branch_name))?;

        // Check if window already exists
        if self.window_exists(issue_number).await? {
            tracing::info!("Window for issue #{} already exists", issue_number);
            return Ok(());
        }

        // Find next available window index to avoid conflicts
        let next_index = self.next_available_window_index().await?;
        let target = format!("{}:{}", self.session_name, next_index);

        // Create the window at a specific index
        tracing::info!(
            "Creating tmux window {} at index {} in session {}",
            window_name,
            next_index,
            self.session_name
        );
        let output = Command::new("tmux")
            .args([
                "new-window",
                "-t",
                &target,
                "-n",
                &window_name,
                "-c",
                &working_dir.to_string_lossy(),
            ])
            .output()
            .await
            .context("Failed to execute tmux new-window command")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);

            // If window creation failed due to index being in use, but our window exists,
            // this is a race condition or manual window creation - just continue
            if stderr.contains("in use") && self.window_exists(issue_number).await? {
                tracing::debug!(
                    "Window creation failed with 'in use' but window #{} exists, continuing",
                    issue_number
                );
                return Ok(());
            }

            anyhow::bail!("Failed to create tmux window: {}", stderr.trim());
        }

        Ok(())
    }

    /// Check if a window exists for an issue
    /// Searches for windows with names starting with "{issue_number}-"
    #[allow(dead_code)]
    pub async fn window_exists(&self, issue_number: u64) -> Result<bool> {
        let window_prefix = format!("{}-", issue_number);

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
        // Strip any state suffix (e.g., ":waiting") before checking prefix
        Ok(windows_output.lines().any(|line| {
            let base_name = line.split(':').next().unwrap_or(line);
            base_name.starts_with(&window_prefix)
        }))
    }

    /// List all issue windows in the session
    /// Returns issue numbers extracted from branch-name windows
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
            // Strip state suffix if present (e.g., "2592-branch:waiting" -> "2592-branch")
            let base_name = line.split(':').next().unwrap_or(line);

            // Extract issue number from branch name (first part before '-')
            if let Some(first_part) = base_name.split('-').next() {
                if let Ok(number) = first_part.parse::<u64>() {
                    issue_numbers.push(number);
                }
            }
        }

        Ok(issue_numbers)
    }

    /// Kill a window for an issue
    /// Finds the window by searching for names starting with "{issue_number}-"
    #[allow(dead_code)]
    pub async fn kill_window(&self, issue_number: u64) -> Result<()> {
        // Find the window name by listing windows
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
            tracing::warn!("Session {} doesn't exist, nothing to kill", self.session_name);
            return Ok(());
        }

        let windows_output = String::from_utf8_lossy(&output.stdout);
        let window_prefix = format!("{}-", issue_number);

        // Find the window matching this issue
        for line in windows_output.lines() {
            let base_name = line.split(':').next().unwrap_or(line);
            if base_name.starts_with(&window_prefix) {
                let target = format!("{}:{}", self.session_name, line);
                tracing::info!("Killing tmux window: {}", target);
                let status = Command::new("tmux")
                    .args(["kill-window", "-t", &target])
                    .status()
                    .await
                    .context("Failed to kill tmux window")?;

                if !status.success() {
                    tracing::warn!("Window {} may not exist or was already killed", target);
                }
                return Ok(());
            }
        }

        tracing::warn!("No window found for issue #{}", issue_number);
        Ok(())
    }

    /// Send keys to a window (for starting Claude, etc.)
    /// Finds the window by searching for names starting with "{issue_number}-"
    pub async fn send_keys(&self, issue_number: u64, keys: &str) -> Result<()> {
        // Find the window name by listing windows
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

        let windows_output = String::from_utf8_lossy(&output.stdout);
        let window_prefix = format!("{}-", issue_number);

        // Find the window matching this issue
        for line in windows_output.lines() {
            let base_name = line.split(':').next().unwrap_or(line);
            if base_name.starts_with(&window_prefix) {
                let target = format!("{}:{}", self.session_name, line);
                tracing::debug!("Sending keys to {}: {}", target, keys);
                let status = Command::new("tmux")
                    .args(["send-keys", "-t", &target, keys, "Enter"])
                    .status()
                    .await
                    .context("Failed to execute tmux send-keys command")?;

                if !status.success() {
                    anyhow::bail!("tmux send-keys command failed for target '{}': {}", target, status);
                }
                return Ok(());
            }
        }

        anyhow::bail!("No window found for issue #{}", issue_number)
    }

    /// Rename a window to include state indicator (e.g., "2592-branch:waiting")
    /// Finds the window by searching for names starting with "{issue_number}-"
    pub async fn rename_window(&self, issue_number: u64, state: &str) -> Result<()> {
        // Find the window name by listing windows
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

        let windows_output = String::from_utf8_lossy(&output.stdout);
        let window_prefix = format!("{}-", issue_number);

        // Find the window matching this issue
        for line in windows_output.lines() {
            let base_name = line.split(':').next().unwrap_or(line);
            if base_name.starts_with(&window_prefix) {
                let target = format!("{}:{}", self.session_name, line);
                let new_name = format!("{}:{}", base_name, state);

                tracing::debug!("Renaming window {} to {}", target, new_name);
                let status = Command::new("tmux")
                    .args(["rename-window", "-t", &target, &new_name])
                    .status()
                    .await
                    .context("Failed to rename tmux window")?;

                if !status.success() {
                    tracing::warn!("Failed to rename window to {}", new_name);
                }
                return Ok(());
            }
        }

        tracing::warn!("No window found for issue #{} to rename", issue_number);
        Ok(())
    }

    /// Select a specific pane in a window (e.g., pane 0 after on_provision hooks)
    /// Finds the window by searching for names starting with "{issue_number}-"
    pub async fn select_pane(&self, issue_number: u64, pane_index: u32) -> Result<()> {
        // Find the window name by listing windows
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

        let windows_output = String::from_utf8_lossy(&output.stdout);
        let window_prefix = format!("{}-", issue_number);

        // Find the window matching this issue
        for line in windows_output.lines() {
            let base_name = line.split(':').next().unwrap_or(line);
            if base_name.starts_with(&window_prefix) {
                let target = format!("{}:{}.{}", self.session_name, base_name, pane_index);

                tracing::debug!("Selecting pane {}", target);
                let status = Command::new("tmux")
                    .args(["select-pane", "-t", &target])
                    .status()
                    .await
                    .context("Failed to execute tmux select-pane command")?;

                if !status.success() {
                    anyhow::bail!("tmux select-pane command failed for target '{}': {}", target, status);
                }
                return Ok(());
            }
        }

        anyhow::bail!("No window found for issue #{} to select pane", issue_number)
    }

    /// Attach to the pleb session (blocking - replaces current terminal)
    /// This returns a std::process::Command that the caller can exec() or status()
    pub fn attach_command(&self) -> std::process::Command {
        let mut cmd = std::process::Command::new("tmux");
        cmd.args(["attach", "-t", &self.session_name]);
        cmd
    }
}
