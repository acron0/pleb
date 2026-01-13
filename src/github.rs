use anyhow::{Context, Result};
use octocrab::Octocrab;

use crate::config::GithubConfig;

#[derive(Debug, Clone)]
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
}
