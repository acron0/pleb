use anyhow::{Context, Result};
use std::path::PathBuf;
use tokio::process::Command;

use crate::config::PathConfig;

#[allow(dead_code)]
pub struct WorktreeManager {
    repo_dir: PathBuf,      // where the main repo clone lives
    worktree_base: PathBuf, // where worktrees are created
}

#[allow(dead_code)]
impl WorktreeManager {
    pub fn new(config: &PathConfig) -> Self {
        Self {
            repo_dir: config.repo_dir.clone(),
            worktree_base: config.worktree_base.clone(),
        }
    }

    /// Create a worktree for an issue (idempotent)
    /// Creates branch `pleb/issue-{number}` and worktree at `worktree_base/issue-{number}`
    /// Handles edge cases: orphaned directories, stale git tracking, existing branches
    pub async fn create_worktree(&self, issue_number: u64) -> Result<PathBuf> {
        let worktree_path = self.worktree_base.join(format!("issue-{}", issue_number));
        let branch_name = format!("pleb/issue-{}", issue_number);

        // 1. Check git's worktree tracking (not just filesystem)
        let is_registered = self.is_worktree_registered(issue_number).await?;
        let path_exists = worktree_path.exists();

        match (is_registered, path_exists) {
            // Already exists and registered - return it
            (true, true) => {
                tracing::debug!(
                    "Worktree for issue #{} already exists at {}",
                    issue_number,
                    worktree_path.display()
                );
                return Ok(worktree_path);
            }
            // Registered but path missing - clean up stale git tracking
            (true, false) => {
                tracing::debug!(
                    "Cleaning up stale worktree registration for issue #{}",
                    issue_number
                );
                let _ = Command::new("git")
                    .arg("-C")
                    .arg(&self.repo_dir)
                    .arg("worktree")
                    .arg("remove")
                    .arg(&worktree_path)
                    .arg("--force")
                    .output()
                    .await;
                // Also prune to clean up
                let _ = Command::new("git")
                    .arg("-C")
                    .arg(&self.repo_dir)
                    .arg("worktree")
                    .arg("prune")
                    .output()
                    .await;
            }
            // Not registered but path exists - remove orphaned directory
            (false, true) => {
                tracing::debug!(
                    "Removing orphaned worktree directory for issue #{}",
                    issue_number
                );
                tokio::fs::remove_dir_all(&worktree_path).await.with_context(|| {
                    format!(
                        "Failed to remove orphaned worktree directory: {}",
                        worktree_path.display()
                    )
                })?;
            }
            // Neither - fresh create
            (false, false) => {}
        }

        // 2. Create branch from main/master: git branch pleb/issue-{number}
        // First, determine the default branch (main or master)
        let default_branch = self.get_default_branch().await?;

        // Create the branch from the default branch
        let branch_output = Command::new("git")
            .arg("-C")
            .arg(&self.repo_dir)
            .arg("branch")
            .arg(&branch_name)
            .arg(&default_branch)
            .output()
            .await
            .with_context(|| {
                format!(
                    "Failed to create branch '{}' from '{}'",
                    branch_name, default_branch
                )
            })?;

        if !branch_output.status.success() {
            let stderr = String::from_utf8_lossy(&branch_output.stderr);
            // If branch already exists, that's okay - we'll use it
            if !stderr.contains("already exists") {
                anyhow::bail!(
                    "Failed to create branch '{}': {}",
                    branch_name,
                    stderr
                );
            }
        }

        // 3. Create worktree: git worktree add {path} {branch}
        // Ensure worktree_base directory exists
        tokio::fs::create_dir_all(&self.worktree_base)
            .await
            .with_context(|| {
                format!(
                    "Failed to create worktree base directory: {}",
                    self.worktree_base.display()
                )
            })?;

        let worktree_output = Command::new("git")
            .arg("-C")
            .arg(&self.repo_dir)
            .arg("worktree")
            .arg("add")
            .arg(&worktree_path)
            .arg(&branch_name)
            .output()
            .await
            .with_context(|| {
                format!(
                    "Failed to create worktree at {}",
                    worktree_path.display()
                )
            })?;

        if !worktree_output.status.success() {
            let stderr = String::from_utf8_lossy(&worktree_output.stderr);
            anyhow::bail!(
                "Failed to create worktree for issue #{}: {}",
                issue_number,
                stderr
            );
        }

        tracing::info!(
            "Created worktree for issue #{} at {}",
            issue_number,
            worktree_path.display()
        );

        // 4. Return worktree path
        Ok(worktree_path)
    }

    /// Get the path to a worktree for an issue (if it exists)
    pub fn get_worktree_path(&self, issue_number: u64) -> Option<PathBuf> {
        let path = self.worktree_base.join(format!("issue-{}", issue_number));
        if path.exists() {
            Some(path)
        } else {
            None
        }
    }

    /// Remove a worktree for an issue
    pub async fn remove_worktree(&self, issue_number: u64) -> Result<()> {
        let worktree_path = self.worktree_base.join(format!("issue-{}", issue_number));
        let branch_name = format!("pleb/issue-{}", issue_number);

        // 1. Get worktree path
        if !worktree_path.exists() {
            tracing::debug!(
                "Worktree for issue #{} doesn't exist at {}",
                issue_number,
                worktree_path.display()
            );
            return Ok(());
        }

        // 2. Run: git worktree remove {path} --force
        let remove_output = Command::new("git")
            .arg("-C")
            .arg(&self.repo_dir)
            .arg("worktree")
            .arg("remove")
            .arg(&worktree_path)
            .arg("--force")
            .output()
            .await
            .with_context(|| {
                format!(
                    "Failed to remove worktree at {}",
                    worktree_path.display()
                )
            })?;

        if !remove_output.status.success() {
            let stderr = String::from_utf8_lossy(&remove_output.stderr);
            anyhow::bail!(
                "Failed to remove worktree for issue #{}: {}",
                issue_number,
                stderr
            );
        }

        tracing::info!(
            "Removed worktree for issue #{} at {}",
            issue_number,
            worktree_path.display()
        );

        // 3. Optionally delete the branch: git branch -D pleb/issue-{number}
        let branch_output = Command::new("git")
            .arg("-C")
            .arg(&self.repo_dir)
            .arg("branch")
            .arg("-D")
            .arg(&branch_name)
            .output()
            .await
            .with_context(|| {
                format!("Failed to delete branch '{}'", branch_name)
            })?;

        if !branch_output.status.success() {
            let stderr = String::from_utf8_lossy(&branch_output.stderr);
            tracing::warn!(
                "Failed to delete branch '{}' (may have been already deleted): {}",
                branch_name,
                stderr
            );
        } else {
            tracing::debug!("Deleted branch '{}'", branch_name);
        }

        Ok(())
    }

    /// List all active issue worktrees
    pub async fn list_worktrees(&self) -> Result<Vec<u64>> {
        // 1. Run: git worktree list --porcelain
        let output = Command::new("git")
            .arg("-C")
            .arg(&self.repo_dir)
            .arg("worktree")
            .arg("list")
            .arg("--porcelain")
            .output()
            .await
            .context("Failed to list worktrees")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Failed to list worktrees: {}", stderr);
        }

        let stdout = String::from_utf8_lossy(&output.stdout);

        // 2. Parse output for worktrees in worktree_base
        // The porcelain format outputs:
        // worktree <path>
        // HEAD <sha>
        // branch <refs/heads/branch-name>
        // (blank line between entries)

        let mut issue_numbers = Vec::new();
        let worktree_base_str = self.worktree_base.to_string_lossy();

        for line in stdout.lines() {
            if line.starts_with("worktree ") {
                let path = line.trim_start_matches("worktree ").trim();

                // 3. Extract issue numbers from paths (issue-123 -> 123)
                if path.starts_with(worktree_base_str.as_ref()) {
                    // Extract the directory name from the path
                    if let Some(dir_name) = std::path::Path::new(path)
                        .file_name()
                        .and_then(|n| n.to_str())
                    {
                        if let Some(issue_str) = dir_name.strip_prefix("issue-") {
                            if let Ok(issue_number) = issue_str.parse::<u64>() {
                                issue_numbers.push(issue_number);
                            }
                        }
                    }
                }
            }
        }

        tracing::debug!(
            "Found {} active issue worktrees: {:?}",
            issue_numbers.len(),
            issue_numbers
        );

        // 4. Return list of issue numbers
        Ok(issue_numbers)
    }

    /// Check if a worktree for an issue is registered with git
    async fn is_worktree_registered(&self, issue_number: u64) -> Result<bool> {
        let worktrees = self.list_worktrees().await?;
        Ok(worktrees.contains(&issue_number))
    }

    /// Check if repo_dir exists and is a git repo, clone if needed
    pub async fn ensure_repo(&self, owner: &str, repo: &str) -> Result<()> {
        // 1. If repo_dir exists and has .git, return Ok
        let git_dir = self.repo_dir.join(".git");

        if self.repo_dir.exists() && git_dir.exists() {
            tracing::debug!(
                "Repository already exists at {}",
                self.repo_dir.display()
            );
            return Ok(());
        }

        // 2. Otherwise, clone: git clone git@github.com:{owner}/{repo}.git {repo_dir}
        tracing::info!(
            "Cloning repository {}/{} to {}",
            owner,
            repo,
            self.repo_dir.display()
        );

        // Ensure parent directory exists
        if let Some(parent) = self.repo_dir.parent() {
            tokio::fs::create_dir_all(parent).await.with_context(|| {
                format!(
                    "Failed to create parent directory for repo: {}",
                    parent.display()
                )
            })?;
        }

        let clone_url = format!("git@github.com:{}/{}.git", owner, repo);
        let clone_output = Command::new("git")
            .arg("clone")
            .arg(&clone_url)
            .arg(&self.repo_dir)
            .output()
            .await
            .with_context(|| {
                format!(
                    "Failed to clone repository {} to {}",
                    clone_url,
                    self.repo_dir.display()
                )
            })?;

        if !clone_output.status.success() {
            let stderr = String::from_utf8_lossy(&clone_output.stderr);
            anyhow::bail!(
                "Failed to clone repository {}/{}: {}",
                owner,
                repo,
                stderr
            );
        }

        tracing::info!(
            "Successfully cloned repository {}/{} to {}",
            owner,
            repo,
            self.repo_dir.display()
        );

        Ok(())
    }

    /// Get the default branch name (main or master)
    async fn get_default_branch(&self) -> Result<String> {
        let output = Command::new("git")
            .arg("-C")
            .arg(&self.repo_dir)
            .arg("rev-parse")
            .arg("--abbrev-ref")
            .arg("HEAD")
            .output()
            .await
            .context("Failed to determine default branch")?;

        if !output.status.success() {
            anyhow::bail!(
                "Failed to determine default branch: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }

        let branch = String::from_utf8_lossy(&output.stdout)
            .trim()
            .to_string();

        Ok(branch)
    }
}
