use anyhow::{Context, Result};
use octocrab::Octocrab;

use crate::config::GithubConfig;

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
}
