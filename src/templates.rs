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
    /// Path to the original repository (not the worktree)
    pub repo_path: String,
}

impl IssueContext {
    /// Create an IssueContext from an Issue and worktree information
    #[allow(dead_code)]
    pub fn from_issue(issue: &Issue, branch_name: &str, worktree_path: &Path, repo_path: &Path) -> Self {
        Self {
            issue_number: issue.number,
            title: issue.title.clone(),
            body: issue.body.clone(),
            branch_name: branch_name.to_string(),
            worktree_path: worktree_path.display().to_string(),
            html_url: issue.html_url.clone(),
            repo_path: repo_path.display().to_string(),
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

    /// Render an arbitrary string template with the given issue context
    #[allow(dead_code)]
    pub fn render_string(&self, template_str: &str, context: &IssueContext) -> Result<String> {
        self.handlebars
            .render_template(template_str, context)
            .with_context(|| {
                format!(
                    "Failed to render template string for issue #{}",
                    context.issue_number
                )
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::github::IssueState;

    fn make_test_issue(number: u64, title: &str, body: &str) -> Issue {
        Issue {
            number,
            title: title.to_string(),
            body: body.to_string(),
            labels: vec!["pleb:ready".to_string()],
            state: IssueState::Open,
            html_url: format!("https://github.com/owner/repo/issues/{}", number),
        }
    }

    #[test]
    fn test_issue_context_from_issue() {
        let issue = make_test_issue(42, "Fix the bug", "This bug needs fixing");
        let ctx = IssueContext::from_issue(
            &issue,
            "pleb/issue-42",
            Path::new("/worktrees/issue-42"),
            Path::new("/repo"),
        );

        assert_eq!(ctx.issue_number, 42);
        assert_eq!(ctx.title, "Fix the bug");
        assert_eq!(ctx.body, "This bug needs fixing");
        assert_eq!(ctx.branch_name, "pleb/issue-42");
        assert_eq!(ctx.worktree_path, "/worktrees/issue-42");
        assert_eq!(ctx.html_url, "https://github.com/owner/repo/issues/42");
        assert_eq!(ctx.repo_path, "/repo");
    }

    #[test]
    fn test_issue_context_with_empty_body() {
        let issue = make_test_issue(123, "No description issue", "");
        let ctx = IssueContext::from_issue(&issue, "pleb/issue-123", Path::new("/tmp/wt"), Path::new("/repo"));

        assert_eq!(ctx.issue_number, 123);
        assert_eq!(ctx.body, "");
    }

    #[test]
    fn test_issue_context_with_special_characters() {
        let issue = make_test_issue(
            999,
            "Handle émojis 🎉 and spëcial chars",
            "Body with\nnewlines\tand\ttabs",
        );
        let ctx = IssueContext::from_issue(
            &issue,
            "pleb/issue-999",
            Path::new("/path/with spaces/issue-999"),
            Path::new("/repo"),
        );

        assert_eq!(ctx.title, "Handle émojis 🎉 and spëcial chars");
        assert_eq!(ctx.body, "Body with\nnewlines\tand\ttabs");
        assert_eq!(ctx.worktree_path, "/path/with spaces/issue-999");
    }

    #[test]
    fn test_issue_context_serializes_to_json() {
        let issue = make_test_issue(1, "Test", "Body");
        let ctx = IssueContext::from_issue(&issue, "branch", Path::new("/path"), Path::new("/repo"));

        // IssueContext derives Serialize, so it should serialize to JSON
        let json = serde_json::to_string(&ctx).expect("Should serialize");
        assert!(json.contains("\"issue_number\":1"));
        assert!(json.contains("\"title\":\"Test\""));
        assert!(json.contains("\"repo_path\":\"/repo\""));
    }

    #[test]
    fn test_render_string_provision_hook() {
        let config = crate::config::PromptsConfig {
            dir: PathBuf::from("/tmp"),
            new_issue: "test.md".to_string(),
        };
        let engine = TemplateEngine::new(&config).expect("Should create engine");

        let issue = make_test_issue(42, "Fix the bug", "Body text");
        let ctx = IssueContext::from_issue(
            &issue,
            "42-fix-bug_user_pleb",
            Path::new("/worktrees/42-fix-bug"),
            Path::new("/home/user/repo"),
        );

        // Test a realistic provision hook command
        let cmd = "tmux split-window -h -c '{{repo_path}}'";
        let rendered = engine.render_string(cmd, &ctx).expect("Should render");
        assert_eq!(rendered, "tmux split-window -h -c '/home/user/repo'");

        // Test multiple variables in one command
        let cmd = "echo 'Issue #{{issue_number}}: {{title}}' > {{worktree_path}}/info.txt";
        let rendered = engine.render_string(cmd, &ctx).expect("Should render");
        assert_eq!(
            rendered,
            "echo 'Issue #42: Fix the bug' > /worktrees/42-fix-bug/info.txt"
        );

        // Test all available variables
        let cmd = "{{repo_path}}|{{worktree_path}}|{{issue_number}}|{{branch_name}}|{{html_url}}";
        let rendered = engine.render_string(cmd, &ctx).expect("Should render");
        assert_eq!(
            rendered,
            "/home/user/repo|/worktrees/42-fix-bug|42|42-fix-bug_user_pleb|https://github.com/owner/repo/issues/42"
        );
    }

    #[test]
    fn test_render_string_no_variables() {
        let config = crate::config::PromptsConfig {
            dir: PathBuf::from("/tmp"),
            new_issue: "test.md".to_string(),
        };
        let engine = TemplateEngine::new(&config).expect("Should create engine");

        let issue = make_test_issue(1, "Test", "Body");
        let ctx = IssueContext::from_issue(&issue, "branch", Path::new("/path"), Path::new("/repo"));

        // Command with no variables should pass through unchanged
        let cmd = "tmux split-window -h";
        let rendered = engine.render_string(cmd, &ctx).expect("Should render");
        assert_eq!(rendered, "tmux split-window -h");
    }

    #[test]
    fn test_render_string_missing_variable_fails() {
        let config = crate::config::PromptsConfig {
            dir: PathBuf::from("/tmp"),
            new_issue: "test.md".to_string(),
        };
        let engine = TemplateEngine::new(&config).expect("Should create engine");

        let issue = make_test_issue(1, "Test", "Body");
        let ctx = IssueContext::from_issue(&issue, "branch", Path::new("/path"), Path::new("/repo"));

        // Unknown variable should fail (strict mode is enabled)
        let cmd = "echo {{nonexistent_var}}";
        let result = engine.render_string(cmd, &ctx);
        assert!(result.is_err());
    }
}
