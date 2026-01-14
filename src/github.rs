use anyhow::{Context, Result};
use octocrab::Octocrab;

use crate::config::{GithubConfig, LabelConfig};
use crate::state::PlebState;

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct Issue {
    pub number: u64,
    pub title: String,
    pub body: String,
    pub labels: Vec<String>,
    pub state: IssueState,
    pub html_url: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum IssueState {
    Open,
    Closed,
}

impl From<octocrab::models::issues::Issue> for Issue {
    fn from(issue: octocrab::models::issues::Issue) -> Self {
        Issue {
            number: issue.number,
            title: issue.title,
            body: issue.body.unwrap_or_default(),
            labels: issue
                .labels
                .into_iter()
                .map(|label| label.name)
                .collect(),
            state: match issue.state {
                octocrab::models::IssueState::Open => IssueState::Open,
                octocrab::models::IssueState::Closed => IssueState::Closed,
                _ => IssueState::Open, // Default to Open for unknown states
            },
            html_url: issue.html_url.to_string(),
        }
    }
}

#[allow(dead_code)]
pub struct GitHubClient {
    client: Octocrab,
    owner: String,
    repo: String,
}

#[allow(dead_code)]
impl GitHubClient {
    /// Create a new GitHub client with authentication
    pub async fn new(config: &GithubConfig) -> Result<Self> {
        // Read token from environment variable specified in config
        let token = std::env::var(&config.token_env).with_context(|| {
            format!(
                "GitHub token not found in environment variable '{}'. \
                 Please set it with: export {}=<your-token>",
                config.token_env, config.token_env
            )
        })?;

        // Create octocrab instance with personal token authentication
        let client = Octocrab::builder()
            .personal_token(token)
            .build()
            .context("Failed to build GitHub client")?;

        Ok(Self {
            client,
            owner: config.owner.clone(),
            repo: config.repo.clone(),
        })
    }

    /// Verify that the client can connect to GitHub and access the repository
    pub async fn verify_connection(&self) -> Result<()> {
        // Fetch repository information to verify token works and repo is accessible
        self.client
            .repos(&self.owner, &self.repo)
            .get()
            .await
            .with_context(|| {
                format!(
                    "Failed to access repository {}/{}. \
                     Verify that the repository exists and your token has 'repo' scope.",
                    self.owner, self.repo
                )
            })?;

        tracing::info!(
            "Successfully connected to GitHub repository: {}/{}",
            self.owner,
            self.repo
        );

        Ok(())
    }

    /// Fetch all open issues with the specified label
    pub async fn get_issues_with_label(&self, label: &str) -> Result<Vec<Issue>> {
        let label_vec = vec![label.to_string()];
        let issues = self
            .client
            .issues(&self.owner, &self.repo)
            .list()
            .state(octocrab::params::State::Open)
            .labels(&label_vec)
            .send()
            .await
            .with_context(|| {
                format!(
                    "Failed to fetch issues with label '{}' from {}/{}",
                    label, self.owner, self.repo
                )
            })?;

        // Convert octocrab issues to our Issue type
        let our_issues: Vec<Issue> = issues.into_iter().map(Issue::from).collect();

        tracing::debug!(
            "Fetched {} issues with label '{}' from {}/{}",
            our_issues.len(),
            label,
            self.owner,
            self.repo
        );

        Ok(our_issues)
    }

    /// Fetch a single issue by number
    pub async fn get_issue(&self, number: u64) -> Result<Issue> {
        let issue = self
            .client
            .issues(&self.owner, &self.repo)
            .get(number)
            .await
            .with_context(|| {
                format!(
                    "Failed to fetch issue #{} from {}/{}",
                    number, self.owner, self.repo
                )
            })?;

        tracing::debug!(
            "Fetched issue #{} from {}/{}",
            number,
            self.owner,
            self.repo
        );

        Ok(Issue::from(issue))
    }

    /// Add a label to an issue
    pub async fn add_label(&self, issue_number: u64, label: &str) -> Result<()> {
        self.client
            .issues(&self.owner, &self.repo)
            .add_labels(issue_number, &[label.to_string()])
            .await
            .with_context(|| {
                format!(
                    "Failed to add label '{}' to issue #{} in {}/{}",
                    label, issue_number, self.owner, self.repo
                )
            })?;

        tracing::debug!(
            "Added label '{}' to issue #{} in {}/{}",
            label,
            issue_number,
            self.owner,
            self.repo
        );

        Ok(())
    }

    /// Remove a label from an issue
    pub async fn remove_label(&self, issue_number: u64, label: &str) -> Result<()> {
        // Attempt to remove the label, but don't fail if it doesn't exist
        match self
            .client
            .issues(&self.owner, &self.repo)
            .remove_label(issue_number, label)
            .await
        {
            Ok(_) => {
                tracing::debug!(
                    "Removed label '{}' from issue #{} in {}/{}",
                    label,
                    issue_number,
                    self.owner,
                    self.repo
                );
                Ok(())
            }
            Err(e) => {
                // Check if error is "label not found" (404) - this is not an error for us
                if e.to_string().contains("404") {
                    tracing::debug!(
                        "Label '{}' not found on issue #{} (already removed or never existed)",
                        label,
                        issue_number
                    );
                    Ok(())
                } else {
                    Err(e).with_context(|| {
                        format!(
                            "Failed to remove label '{}' from issue #{} in {}/{}",
                            label, issue_number, self.owner, self.repo
                        )
                    })
                }
            }
        }
    }

    /// Replace one label with another (atomic state transition)
    pub async fn replace_label(
        &self,
        issue_number: u64,
        old_label: &str,
        new_label: &str,
    ) -> Result<()> {
        // Remove old label (ignore if it doesn't exist)
        self.remove_label(issue_number, old_label).await?;

        // Add new label
        self.add_label(issue_number, new_label).await?;

        tracing::debug!(
            "Replaced label '{}' with '{}' on issue #{} in {}/{}",
            old_label,
            new_label,
            issue_number,
            self.owner,
            self.repo
        );

        Ok(())
    }

    /// Transition an issue from one pleb state to another
    pub async fn transition_state(
        &self,
        issue_number: u64,
        from: PlebState,
        to: PlebState,
        labels_config: &LabelConfig,
    ) -> Result<()> {
        let old_label = self.state_to_label(from, labels_config);
        let new_label = self.state_to_label(to, labels_config);

        self.replace_label(issue_number, &old_label, &new_label)
            .await?;

        tracing::info!(
            "Transitioned issue #{} from {:?} to {:?}",
            issue_number,
            from,
            to
        );

        Ok(())
    }

    /// Convert a PlebState to the corresponding label string from config
    fn state_to_label(&self, state: PlebState, config: &LabelConfig) -> String {
        match state {
            PlebState::Ready => config.ready.clone(),
            PlebState::Provisioning => config.provisioning.clone(),
            PlebState::Waiting => config.waiting.clone(),
            PlebState::Working => config.working.clone(),
            PlebState::Done => config.done.clone(),
        }
    }

    /// Determine current pleb state from issue labels
    pub fn get_pleb_state(&self, issue: &Issue, labels_config: &LabelConfig) -> Option<PlebState> {
        // Check which pleb label the issue has
        for label in &issue.labels {
            if label == &labels_config.ready {
                return Some(PlebState::Ready);
            } else if label == &labels_config.provisioning {
                return Some(PlebState::Provisioning);
            } else if label == &labels_config.waiting {
                return Some(PlebState::Waiting);
            } else if label == &labels_config.working {
                return Some(PlebState::Working);
            } else if label == &labels_config.done {
                return Some(PlebState::Done);
            }
        }

        // No pleb label found
        None
    }
}
