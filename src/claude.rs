use std::path::Path;

use anyhow::{Context, Result};
use tokio::process::Command;

use crate::config::{ClaudeConfig, TmuxConfig};
use crate::tmux::TmuxManager;

/// Manages Claude Code invocation and process monitoring
#[allow(dead_code)]
pub struct ClaudeRunner {
    command: String,
    args: Vec<String>,
    tmux: TmuxManager,
}

impl ClaudeRunner {
    /// Create a new ClaudeRunner
    #[allow(dead_code)]
    pub fn new(config: &ClaudeConfig, tmux_config: &TmuxConfig) -> Self {
        Self {
            command: config.command.clone(),
            args: config.args.clone(),
            tmux: TmuxManager::new(tmux_config),
        }
    }

    /// Invoke Claude Code in the issue's tmux window with the given prompt
    /// Claude starts in interactive mode so user can attach and interact
    #[allow(dead_code)]
    pub async fn invoke(&self, issue_number: u64, prompt: &str, daemon_dir: &Path) -> Result<()> {
        // Create issue-specific directory for prompt file
        let issue_dir = daemon_dir.join(issue_number.to_string());
        std::fs::create_dir_all(&issue_dir)
            .with_context(|| format!("Failed to create issue directory: {:?}", issue_dir))?;

        // Write prompt to persistent location
        let prompt_file = issue_dir.join("prompt.md");
        std::fs::write(&prompt_file, prompt)
            .with_context(|| format!("Failed to write prompt file: {:?}", prompt_file))?;

        // Build claude command with prompt file as argument
        // Using @/path/to/file syntax to read prompt from file
        // Note: --permission-mode must come before other flags
        let mut cmd_parts = vec![self.command.clone()];
        cmd_parts.push("--permission-mode".to_string());
        cmd_parts.push("plan".to_string());
        cmd_parts.extend(self.args.iter().cloned());
        cmd_parts.push(format!("@{}", prompt_file.display()));
        let claude_command = cmd_parts.join(" ");

        tracing::info!(
            "Invoking Claude Code for issue #{} with command: {}",
            issue_number,
            claude_command
        );

        // Start Claude with the prompt file argument
        self.tmux.send_keys(issue_number, &claude_command).await?;

        Ok(())
    }

    /// Check if Claude is currently running in the issue's window
    #[allow(dead_code)]
    pub async fn is_running(&self, issue_number: u64) -> Result<bool> {
        let window_name = format!("issue-{}", issue_number);
        let session_name = &self.tmux.session_name();

        // Get the current command running in the pane
        let output = Command::new("tmux")
            .args([
                "list-panes",
                "-t",
                &format!("{}:{}", session_name, window_name),
                "-F",
                "#{pane_current_command}",
            ])
            .output()
            .await
            .context("Failed to list panes")?;

        if !output.status.success() {
            // Window doesn't exist
            return Ok(false);
        }

        let current_command = String::from_utf8_lossy(&output.stdout);
        let current_command = current_command.trim();

        // Check if the command contains "claude" (case-insensitive)
        Ok(current_command.to_lowercase().contains("claude"))
    }

    /// Check if the window exists but Claude is not running (idle state)
    /// Useful for detecting when Claude has finished
    #[allow(dead_code)]
    pub async fn is_idle(&self, issue_number: u64) -> Result<bool> {
        let window_name = format!("issue-{}", issue_number);
        let session_name = &self.tmux.session_name();

        // Check if window exists
        let status = Command::new("tmux")
            .args([
                "list-windows",
                "-t",
                session_name,
                "-F",
                "#{window_name}",
            ])
            .output()
            .await
            .context("Failed to list windows")?;

        if !status.status.success() {
            // Session doesn't exist
            return Ok(false);
        }

        let windows = String::from_utf8_lossy(&status.stdout);
        let window_exists = windows.lines().any(|line| line == window_name);

        if !window_exists {
            return Ok(false);
        }

        // Window exists, check if Claude is not running
        Ok(!self.is_running(issue_number).await?)
    }
}
