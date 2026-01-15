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

    /// Get the daemon directory for this repo: ~/.pleb/{owner}-{repo}/
    pub fn daemon_dir(&self) -> Result<PathBuf> {
        let home = dirs::home_dir().context("Failed to determine home directory")?;
        let dir_name = format!("{}-{}", self.github.owner, self.github.repo);
        Ok(home.join(".pleb").join(dir_name))
    }

    /// Get the log file path: ~/.pleb/{owner}-{repo}/pleb.log
    pub fn log_file(&self) -> Result<PathBuf> {
        Ok(self.daemon_dir()?.join("pleb.log"))
    }

    /// Get the PID file path: ~/.pleb/{owner}-{repo}/pleb.pid
    pub fn pid_file(&self) -> Result<PathBuf> {
        Ok(self.daemon_dir()?.join("pleb.pid"))
    }

    /// Parse configuration from a TOML string (useful for testing)
    #[allow(dead_code)]
    pub fn from_str(content: &str) -> Result<Self> {
        toml::from_str(content).context("Failed to parse config")
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

        // Validate prompts config - directory and files must exist
        anyhow::ensure!(
            !self.prompts.new_issue.is_empty(),
            "prompts.new_issue must not be empty"
        );
        anyhow::ensure!(
            !self.prompts.planning_done.is_empty(),
            "prompts.planning_done must not be empty"
        );

        anyhow::ensure!(
            self.prompts.dir.exists(),
            "Prompts directory does not exist: {}",
            self.prompts.dir.display()
        );

        let new_issue_path = self.prompts.dir.join(&self.prompts.new_issue);
        anyhow::ensure!(
            new_issue_path.exists(),
            "Prompt file does not exist: {}",
            new_issue_path.display()
        );

        let planning_done_path = self.prompts.dir.join(&self.prompts.planning_done);
        anyhow::ensure!(
            planning_done_path.exists(),
            "Prompt file does not exist: {}",
            planning_done_path.display()
        );

        // Validate watch config
        anyhow::ensure!(
            self.watch.poll_interval_secs > 0,
            "watch.poll_interval_secs must be greater than 0"
        );

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MINIMAL_CONFIG: &str = r#"
[github]
owner = "testowner"
repo = "testrepo"

[labels]
[claude]
[paths]
[prompts]
[watch]
[tmux]
"#;

    const FULL_CONFIG: &str = r#"
[github]
owner = "myorg"
repo = "myrepo"
token_env = "MY_GITHUB_TOKEN"

[labels]
ready = "custom:ready"
provisioning = "custom:provisioning"
waiting = "custom:waiting"
working = "custom:working"
done = "custom:done"

[claude]
command = "/usr/local/bin/claude"
args = ["--verbose", "--no-cache"]

[paths]
repo_dir = "/custom/repo"
worktree_base = "/custom/worktrees"

[prompts]
dir = "/custom/prompts"
new_issue = "custom_new.md"
planning_done = "custom_done.md"

[watch]
poll_interval_secs = 30

[tmux]
session_name = "custom-session"
"#;

    // ===================
    // TOML Parsing Tests
    // ===================

    #[test]
    fn test_parse_minimal_config() {
        let config = Config::from_str(MINIMAL_CONFIG).expect("Should parse minimal config");
        assert_eq!(config.github.owner, "testowner");
        assert_eq!(config.github.repo, "testrepo");
    }

    #[test]
    fn test_parse_full_config() {
        let config = Config::from_str(FULL_CONFIG).expect("Should parse full config");
        assert_eq!(config.github.owner, "myorg");
        assert_eq!(config.github.repo, "myrepo");
        assert_eq!(config.github.token_env, "MY_GITHUB_TOKEN");
        assert_eq!(config.labels.ready, "custom:ready");
        assert_eq!(config.claude.command, "/usr/local/bin/claude");
        assert_eq!(config.claude.args, vec!["--verbose", "--no-cache"]);
        assert_eq!(config.paths.repo_dir, PathBuf::from("/custom/repo"));
        assert_eq!(config.watch.poll_interval_secs, 30);
        assert_eq!(config.tmux.session_name, "custom-session");
    }

    #[test]
    fn test_parse_invalid_toml_syntax() {
        let invalid = "this is not valid toml [[[";
        let result = Config::from_str(invalid);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_missing_required_section() {
        // Missing [github] section entirely
        let missing_github = r#"
[labels]
[claude]
[paths]
[prompts]
[watch]
[tmux]
"#;
        let result = Config::from_str(missing_github);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_missing_required_field() {
        // Has [github] but missing owner
        let missing_owner = r#"
[github]
repo = "testrepo"

[labels]
[claude]
[paths]
[prompts]
[watch]
[tmux]
"#;
        let result = Config::from_str(missing_owner);
        assert!(result.is_err());
    }

    // ===================
    // Defaults Tests
    // ===================

    #[test]
    fn test_defaults_applied() {
        let config = Config::from_str(MINIMAL_CONFIG).expect("Should parse");

        // GitHub defaults
        assert_eq!(config.github.token_env, "GITHUB_TOKEN");

        // Label defaults
        assert_eq!(config.labels.ready, "pleb:ready");
        assert_eq!(config.labels.provisioning, "pleb:provisioning");
        assert_eq!(config.labels.waiting, "pleb:waiting");
        assert_eq!(config.labels.working, "pleb:working");
        assert_eq!(config.labels.done, "pleb:done");

        // Claude defaults
        assert_eq!(config.claude.command, "claude");
        assert_eq!(config.claude.args, vec!["--dangerously-skip-permissions"]);

        // Path defaults
        assert_eq!(config.paths.repo_dir, PathBuf::from("./repo"));
        assert_eq!(config.paths.worktree_base, PathBuf::from("./worktrees"));

        // Prompts defaults
        assert_eq!(config.prompts.dir, PathBuf::from("./prompts"));
        assert_eq!(config.prompts.new_issue, "new_issue.md");
        assert_eq!(config.prompts.planning_done, "planning_done.md");

        // Watch defaults
        assert_eq!(config.watch.poll_interval_secs, 5);

        // Tmux defaults
        assert_eq!(config.tmux.session_name, "pleb");
    }

    // ===================
    // Validation Tests
    // ===================

    #[test]
    fn test_validate_empty_owner() {
        let toml = r#"
[github]
owner = ""
repo = "testrepo"

[labels]
[claude]
[paths]
[prompts]
[watch]
[tmux]
"#;
        let config = Config::from_str(toml).expect("Should parse");
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("owner"));
    }

    #[test]
    fn test_validate_empty_repo() {
        let toml = r#"
[github]
owner = "testowner"
repo = ""

[labels]
[claude]
[paths]
[prompts]
[watch]
[tmux]
"#;
        let config = Config::from_str(toml).expect("Should parse");
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("repo"));
    }

    #[test]
    fn test_validate_empty_token_env() {
        let toml = r#"
[github]
owner = "testowner"
repo = "testrepo"
token_env = ""

[labels]
[claude]
[paths]
[prompts]
[watch]
[tmux]
"#;
        let config = Config::from_str(toml).expect("Should parse");
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("token_env"));
    }

    #[test]
    fn test_validate_duplicate_labels() {
        let toml = r#"
[github]
owner = "testowner"
repo = "testrepo"

[labels]
ready = "same-label"
done = "same-label"

[claude]
[paths]
[prompts]
[watch]
[tmux]
"#;
        let config = Config::from_str(toml).expect("Should parse");
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Label conflict"));
    }

    #[test]
    fn test_validate_zero_poll_interval() {
        let toml = r#"
[github]
owner = "testowner"
repo = "testrepo"

[labels]
[claude]
[paths]
[prompts]
[watch]
poll_interval_secs = 0

[tmux]
"#;
        let config = Config::from_str(toml).expect("Should parse");
        let result = config.validate();
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("poll_interval_secs"));
    }

    // ===================
    // Path Construction Tests
    // ===================

    #[test]
    fn test_daemon_dir_construction() {
        let config = Config::from_str(MINIMAL_CONFIG).expect("Should parse");
        let daemon_dir = config.daemon_dir().expect("Should get daemon dir");

        // Should end with .pleb/testowner-testrepo
        let path_str = daemon_dir.to_string_lossy();
        assert!(path_str.contains(".pleb"));
        assert!(path_str.ends_with("testowner-testrepo"));
    }

    #[test]
    fn test_log_file_construction() {
        let config = Config::from_str(MINIMAL_CONFIG).expect("Should parse");
        let log_file = config.log_file().expect("Should get log file");

        assert!(log_file.to_string_lossy().ends_with("pleb.log"));
    }

    #[test]
    fn test_pid_file_construction() {
        let config = Config::from_str(MINIMAL_CONFIG).expect("Should parse");
        let pid_file = config.pid_file().expect("Should get pid file");

        assert!(pid_file.to_string_lossy().ends_with("pleb.pid"));
    }
}
