use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs;
use std::path::Path;

#[derive(Debug, Serialize, Deserialize)]
pub struct Hook {
    #[serde(rename = "type")]
    pub hook_type: String,
    pub command: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HookEntry {
    pub hooks: Vec<Hook>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HooksConfig {
    pub hooks: std::collections::HashMap<String, Vec<HookEntry>>,
}

/// Generate the Claude Code hooks configuration
pub fn generate_hooks_json() -> Result<String> {
    let mut hooks = std::collections::HashMap::new();

    // Stop hook - transitions to waiting state
    hooks.insert(
        "Stop".to_string(),
        vec![HookEntry {
            hooks: vec![Hook {
                hook_type: "command".to_string(),
                command: "pleb cc-run-hook Stop".to_string(),
            }],
        }],
    );

    // UserPromptSubmit hook - transitions to working state
    hooks.insert(
        "UserPromptSubmit".to_string(),
        vec![HookEntry {
            hooks: vec![Hook {
                hook_type: "command".to_string(),
                command: "pleb cc-run-hook UserPromptSubmit".to_string(),
            }],
        }],
    );

    // PostToolUse hook - future extensibility for tool monitoring
    hooks.insert(
        "PostToolUse".to_string(),
        vec![HookEntry {
            hooks: vec![Hook {
                hook_type: "command".to_string(),
                command: "pleb cc-run-hook PostToolUse".to_string(),
            }],
        }],
    );

    // PermissionRequest hook - future extensibility for permission monitoring
    hooks.insert(
        "PermissionRequest".to_string(),
        vec![HookEntry {
            hooks: vec![Hook {
                hook_type: "command".to_string(),
                command: "pleb cc-run-hook PermissionRequest".to_string(),
            }],
        }],
    );

    let config = HooksConfig { hooks };

    let json = serde_json::to_string_pretty(&config)
        .context("Failed to serialize hooks config to JSON")?;

    Ok(json)
}

/// Install hooks to the specified directory's .claude/settings.json
/// Also installs slash commands to .claude/commands/
pub fn install_hooks(path: &Path) -> Result<()> {
    let claude_dir = path.join(".claude");
    let settings_file = claude_dir.join("settings.json");

    // Create .claude directory if it doesn't exist
    if !claude_dir.exists() {
        fs::create_dir_all(&claude_dir)
            .with_context(|| format!("Failed to create directory: {}", claude_dir.display()))?;
        tracing::debug!("Created .claude directory at: {}", claude_dir.display());
    }

    // Generate the hooks configuration
    let hooks_config = generate_hooks_json()?;
    let hooks_value: Value =
        serde_json::from_str(&hooks_config).context("Failed to parse hooks JSON")?;

    // Read existing settings or create new object
    let mut settings: Value = if settings_file.exists() {
        let content = fs::read_to_string(&settings_file)
            .with_context(|| format!("Failed to read {}", settings_file.display()))?;
        serde_json::from_str(&content)
            .with_context(|| format!("Failed to parse {}", settings_file.display()))?
    } else {
        serde_json::json!({})
    };

    // Merge hooks into settings
    if let Some(obj) = settings.as_object_mut() {
        obj.insert("hooks".to_string(), hooks_value["hooks"].clone());
    }

    // Write back to file
    let settings_str = serde_json::to_string_pretty(&settings)
        .context("Failed to serialize settings to JSON")?;
    fs::write(&settings_file, settings_str)
        .with_context(|| format!("Failed to write {}", settings_file.display()))?;

    tracing::info!(
        "Installed Claude Code hooks to: {}",
        settings_file.display()
    );

    // Install slash commands
    crate::commands::install_commands(path)
        .context("Failed to install slash commands")?;

    Ok(())
}

/// Parse issue number from a worktree path
/// Supports both old format "/path/worktrees/issue-123" and
/// new format "/path/worktrees/123-slug_username_suffix"
pub fn extract_issue_number_from_path(path: &str) -> Option<u64> {
    for component in path.split('/') {
        // Try old format: "issue-{number}"
        if let Some(issue_part) = component.strip_prefix("issue-") {
            if let Ok(number) = issue_part.parse::<u64>() {
                return Some(number);
            }
        }

        // Try new format: "{number}-{rest}" where number is at the start
        if let Some(dash_pos) = component.find('-') {
            let prefix = &component[..dash_pos];
            if let Ok(number) = prefix.parse::<u64>() {
                return Some(number);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_issue_number() {
        // Old format: issue-{number}
        assert_eq!(
            extract_issue_number_from_path("/path/worktrees/issue-123"),
            Some(123)
        );
        assert_eq!(
            extract_issue_number_from_path("/home/user/worktrees/issue-42/src"),
            Some(42)
        );
        assert_eq!(extract_issue_number_from_path("issue-456"), Some(456));

        // New format: {number}-{slug}_{username}_{suffix}
        assert_eq!(
            extract_issue_number_from_path("/path/worktrees/2592-add-invoices-table_user_pleb"),
            Some(2592)
        );
        assert_eq!(
            extract_issue_number_from_path("/home/acron/projects/kikin/monorepo-branches/2592-add-invoices-table-to-the_acron0_pleb"),
            Some(2592)
        );

        // No issue number
        assert_eq!(extract_issue_number_from_path("/path/no-issue-here"), None);
        assert_eq!(extract_issue_number_from_path("/path/main"), None);
    }

    #[test]
    fn test_generate_hooks_json() {
        let json = generate_hooks_json().unwrap();

        // Verify all 4 hook types are present
        assert!(json.contains("Stop"));
        assert!(json.contains("UserPromptSubmit"));
        assert!(json.contains("PostToolUse"));
        assert!(json.contains("PermissionRequest"));

        // Verify commands use correct event names
        assert!(json.contains("pleb cc-run-hook Stop"));
        assert!(json.contains("pleb cc-run-hook UserPromptSubmit"));
        assert!(json.contains("pleb cc-run-hook PostToolUse"));
        assert!(json.contains("pleb cc-run-hook PermissionRequest"));
    }
}
