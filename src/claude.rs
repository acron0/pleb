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
    pub async fn invoke(&self, issue_number: u64, prompt: &str) -> Result<()> {
        let window_name = format!("issue-{}", issue_number);
        let session_name = self.tmux.session_name();
        let target = format!("{}:{}", session_name, window_name);

        // Write prompt to a temp file
        let temp_file = format!("/tmp/pleb-prompt-{}.md", issue_number);
        std::fs::write(&temp_file, prompt)
            .with_context(|| format!("Failed to write prompt to temp file: {}", temp_file))?;

        // Build claude command (always start in plan mode for issue-driven work)
        let mut cmd_parts = vec![self.command.clone()];
        cmd_parts.extend(self.args.iter().cloned());
        cmd_parts.push("--permission-mode".to_string());
        cmd_parts.push("plan".to_string());
        let claude_command = cmd_parts.join(" ");

        tracing::info!(
            "Invoking Claude Code for issue #{} with command: {}",
            issue_number,
            claude_command
        );

        // Step 1: Load prompt into tmux buffer
        Command::new("tmux")
            .args(["load-buffer", &temp_file])
            .status()
            .await
            .context("Failed to load prompt into tmux buffer")?;

        // Step 2: Start Claude (interactive mode, no piping)
        self.tmux.send_keys(issue_number, &claude_command).await?;

        // Step 3: Wait for Claude to initialize
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

        // Step 4: Paste the prompt from buffer
        Command::new("tmux")
            .args(["paste-buffer", "-t", &target])
            .status()
            .await
            .context("Failed to paste prompt buffer")?;

        // Step 5: Small delay to ensure paste is processed
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Step 6: Send Enter to submit the prompt
        Command::new("tmux")
            .args(["send-keys", "-t", &target, "Enter"])
            .status()
            .await
            .context("Failed to send Enter key")?;

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
