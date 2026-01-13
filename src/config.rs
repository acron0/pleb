use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    pub github: GithubConfig,
    pub labels: LabelConfig,
    pub claude: ClaudeConfig,
    pub paths: PathConfig,
    pub prompts: PromptsConfig,
    pub watch: WatchConfig,
    pub tmux: TmuxConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GithubConfig {
    pub owner: String,
    pub repo: String,
    #[serde(default = "default_token_env")]
    pub token_env: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LabelConfig {
    #[serde(default = "default_label_ready")]
    pub ready: String,
    #[serde(default = "default_label_provisioning")]
    pub provisioning: String,
    #[serde(default = "default_label_waiting")]
    pub waiting: String,
    #[serde(default = "default_label_working")]
    pub working: String,
    #[serde(default = "default_label_done")]
    pub done: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ClaudeConfig {
    #[serde(default = "default_claude_command")]
    pub command: String,
    #[serde(default = "default_claude_args")]
    pub args: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PathConfig {
    #[serde(default = "default_repo_dir")]
    pub repo_dir: PathBuf,
    #[serde(default = "default_worktree_base")]
    pub worktree_base: PathBuf,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PromptsConfig {
    #[serde(default = "default_prompts_dir")]
    pub dir: PathBuf,
    #[serde(default = "default_prompt_new_issue")]
    pub new_issue: String,
    #[serde(default = "default_prompt_planning_done")]
    pub planning_done: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct WatchConfig {
    #[serde(default = "default_poll_interval_secs")]
    pub poll_interval_secs: u64,
}

// Default value functions
fn default_token_env() -> String {
    "GITHUB_TOKEN".to_string()
}

fn default_label_ready() -> String {
    "pleb:ready".to_string()
}

fn default_label_provisioning() -> String {
    "pleb:provisioning".to_string()
}

fn default_label_waiting() -> String {
    "pleb:waiting".to_string()
}

fn default_label_working() -> String {
    "pleb:working".to_string()
}

fn default_label_done() -> String {
    "pleb:done".to_string()
}

fn default_claude_command() -> String {
    "claude".to_string()
}

fn default_claude_args() -> Vec<String> {
    vec!["--dangerously-skip-permissions".to_string()]
}

fn default_repo_dir() -> PathBuf {
    PathBuf::from("./repo")
}

fn default_worktree_base() -> PathBuf {
    PathBuf::from("./worktrees")
}

fn default_prompts_dir() -> PathBuf {
    PathBuf::from("./prompts")
}

fn default_prompt_new_issue() -> String {
    "new_issue.md".to_string()
}

fn default_prompt_planning_done() -> String {
    "planning_done.md".to_string()
}

fn default_poll_interval_secs() -> u64 {
    5
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TmuxConfig {
    #[serde(default = "default_session_name")]
    pub session_name: String,
}

fn default_session_name() -> String {
    "pleb".to_string()
}

impl Config {
    /// Load configuration from the specified file path
    pub fn load(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read config file: {}", path.display()))?;

        let config: Config = toml::from_str(&content)
            .with_context(|| format!("Failed to parse config file: {}", path.display()))?;

        Ok(config)
    }

    /// Load configuration from the default location (./pleb.toml)
    pub fn load_default() -> Result<Self> {
        Self::load(Path::new("pleb.toml"))
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<()> {
        // Validate GitHub config
        anyhow::ensure!(
            !self.github.owner.is_empty(),
            "github.owner must not be empty"
        );
        anyhow::ensure!(
            !self.github.repo.is_empty(),
            "github.repo must not be empty"
        );
        anyhow::ensure!(
            !self.github.token_env.is_empty(),
            "github.token_env must not be empty"
        );

        // Validate labels don't conflict
        let labels = [
            &self.labels.ready,
            &self.labels.provisioning,
            &self.labels.waiting,
            &self.labels.working,
            &self.labels.done,
        ];

        for (i, label1) in labels.iter().enumerate() {
            for label2 in labels.iter().skip(i + 1) {
                anyhow::ensure!(
                    label1 != label2,
                    "Label conflict: '{}' is used for multiple states",
                    label1
                );
            }
        }

        // Warn if repo_dir doesn't exist (will be cloned later)
        if !self.paths.repo_dir.exists() {
            tracing::warn!(
                "Repo directory does not exist: {} (it will be cloned when needed)",
                self.paths.repo_dir.display()
            );
        }

        // Warn if worktree_base doesn't exist (will be created later)
        if !self.paths.worktree_base.exists() {
            tracing::warn!(
                "Worktree base directory does not exist: {} (it will be created when needed)",
                self.paths.worktree_base.display()
            );
        }

        // Validate prompts config
        anyhow::ensure!(
            !self.prompts.new_issue.is_empty(),
            "prompts.new_issue must not be empty"
        );
        anyhow::ensure!(
            !self.prompts.planning_done.is_empty(),
            "prompts.planning_done must not be empty"
        );

        // Warn if prompts directory doesn't exist
        if !self.prompts.dir.exists() {
            tracing::warn!(
                "Prompts directory does not exist: {} (prompt files should be created here)",
                self.prompts.dir.display()
            );
        }

        // Validate watch config
        anyhow::ensure!(
            self.watch.poll_interval_secs > 0,
            "watch.poll_interval_secs must be greater than 0"
        );

        Ok(())
    }
}
