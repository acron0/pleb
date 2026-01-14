use anyhow::{Context, Result};
use handlebars::Handlebars;
use serde::Serialize;
use std::path::{Path, PathBuf};

use crate::config::PromptsConfig;
use crate::github::Issue;

/// Context data for rendering issue templates
#[derive(Debug, Clone, Serialize)]
#[allow(dead_code)]
pub struct IssueContext {
    pub issue_number: u64,
    pub title: String,
    pub body: String,
    pub branch_name: String,
    pub worktree_path: String,
    pub html_url: String,
}

impl IssueContext {
    /// Create an IssueContext from an Issue and worktree information
    #[allow(dead_code)]
    pub fn from_issue(issue: &Issue, branch_name: &str, worktree_path: &Path) -> Self {
        Self {
            issue_number: issue.number,
            title: issue.title.clone(),
            body: issue.body.clone(),
            branch_name: branch_name.to_string(),
            worktree_path: worktree_path.display().to_string(),
            html_url: issue.html_url.clone(),
        }
    }
}

/// Template engine for rendering prompts with issue context
#[allow(dead_code)]
pub struct TemplateEngine {
    handlebars: Handlebars<'static>,
    templates_dir: PathBuf,
}

impl TemplateEngine {
    /// Create a new TemplateEngine with the given prompts configuration
    #[allow(dead_code)]
    pub fn new(config: &PromptsConfig) -> Result<Self> {
        let mut handlebars = Handlebars::new();

        // Enable strict mode to fail on missing variables
        handlebars.set_strict_mode(true);

        Ok(Self {
            handlebars,
            templates_dir: config.dir.clone(),
        })
    }

    /// Load a template from a file in the templates directory
    #[allow(dead_code)]
    pub fn load_template(&mut self, name: &str) -> Result<()> {
        let template_path = self.templates_dir.join(name);

        self.handlebars
            .register_template_file(name, &template_path)
            .with_context(|| {
                format!(
                    "Failed to load template '{}' from {}",
                    name,
                    template_path.display()
                )
            })?;

        tracing::debug!("Loaded template '{}' from {}", name, template_path.display());

        Ok(())
    }

    /// Render a registered template with the given issue context
    #[allow(dead_code)]
    pub fn render(&self, template_name: &str, context: &IssueContext) -> Result<String> {
        self.handlebars
            .render(template_name, context)
            .with_context(|| {
                format!(
                    "Failed to render template '{}' with issue #{}",
                    template_name, context.issue_number
                )
            })
    }
}
