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
            PlebState::Finished => config.finished.clone(),
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
            } else if label == &labels_config.finished {
                return Some(PlebState::Finished);
            }
        }

        // No pleb label found
        None
    }

    /// Get the username of the authenticated user
    pub async fn get_authenticated_user(&self) -> Result<String> {
        let user = self
            .client
            .current()
            .user()
            .await
            .context("Failed to get authenticated user")?;

        Ok(user.login)
    }

    /// Find an open pull request associated with an issue number.
    ///
    /// Searches for PRs whose head branch starts with `{issue_number}-` which
    /// matches pleb's branch naming convention: `{issue_number}-{slug}_{user}_{suffix}`.
    /// Returns the PR URL if found.
    ///
    /// Uses `gh` CLI which has its own authentication.
    pub async fn get_pull_request_for_issue(&self, issue_number: u64) -> Result<Option<String>> {
        use std::process::Command;

        // Use gh CLI to list PRs and filter by branch prefix
        // gh pr list --repo owner/repo --state open --json headRefName,url
        let output = Command::new("gh")
            .args([
                "pr",
                "list",
                "--repo",
                &format!("{}/{}", self.owner, self.repo),
                "--state",
                "open",
                "--json",
                "headRefName,url",
                "--limit",
                "200",
            ])
            .output()
            .context("Failed to execute gh command")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("gh pr list failed: {}", stderr);
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let prs: Vec<serde_json::Value> =
            serde_json::from_str(&stdout).context("Failed to parse gh output")?;

        let branch_prefix = format!("{}-", issue_number);

        for pr in prs {
            if let (Some(head_ref), Some(url)) = (
                pr.get("headRefName").and_then(|v| v.as_str()),
                pr.get("url").and_then(|v| v.as_str()),
            ) {
                if head_ref.starts_with(&branch_prefix) {
                    return Ok(Some(url.to_string()));
                }
            }
        }

        Ok(None)
    }

    /// Fetch the issue body_html which contains signed URLs for private attachments.
    ///
    /// GitHub user-attachments (images/videos uploaded to issues) require special
    /// authentication. When fetching with `Accept: application/vnd.github.full+json`,
    /// GitHub returns body_html with short-lived JWT tokens in the image URLs.
    ///
    /// Note: We use reqwest directly here because octocrab doesn't easily support
    /// custom Accept headers per-request.
    pub async fn get_issue_body_html(&self, issue_number: u64, github_token: &str) -> Result<String> {
        let url = format!(
            "https://api.github.com/repos/{}/{}/issues/{}",
            self.owner, self.repo, issue_number
        );

        let client = reqwest::Client::new();
        let response = client
            .get(&url)
            .header("Accept", "application/vnd.github.full+json")
            .header("Authorization", format!("Bearer {}", github_token))
            .header("User-Agent", "pleb")
            .send()
            .await
            .with_context(|| {
                format!(
                    "Failed to fetch body_html for issue #{} from {}/{}",
                    issue_number, self.owner, self.repo
                )
            })?;

        if !response.status().is_success() {
            anyhow::bail!(
                "GitHub API returned {} for issue #{}",
                response.status(),
                issue_number
            );
        }

        let json: serde_json::Value = response
            .json::<serde_json::Value>()
            .await
            .context("Failed to parse response as JSON")?;

        let body_html = json
            .get("body_html")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        tracing::info!(
            "Fetched body_html for issue #{} ({} chars)",
            issue_number,
            body_html.len()
        );

        Ok(body_html)
    }
}
