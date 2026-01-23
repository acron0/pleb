use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

/// Slash command content for `/pleb-shipit`
pub const PLEB_SHIPIT_COMMAND: &str = r#"# Ship It

Create a pull request for the current work and mark the issue as done.

## Steps
1. Stage and commit any uncommitted changes with a descriptive message
2. Push the current branch to origin
3. Create a pull request using `gh pr create`:
   - Title: Use the issue title or branch name
   - Body: Reference the issue number (Fixes #XXX) so GitHub auto-closes it on merge
4. Run: `pleb transition <issue-number> done`
5. Report the PR URL to the user

## Context
- Working directory: Current worktree (contains issue number in path)
- Branch: Already created by pleb (pleb/issue-XXX)
- Issue number: Extract from current directory path

## Important
- If there are no changes to commit, skip step 1
- If PR already exists for this branch, report existing PR instead of creating new one
- Always transition to done state after PR is created/found
- Do NOT close the issue - GitHub will auto-close it when the PR is merged
"#;

/// Slash command content for `/pleb-abandon`
pub const PLEB_ABANDON_COMMAND: &str = r#"# Abandon Issue

Give up on the current issue and clean up.

## Steps
1. Extract issue number from current directory path (worktree path contains issue-XXX)
2. Remove all pleb labels from the issue using:
   ```bash
   pleb transition <issue-number> none
   ```
   (Note: "none" is a special state that removes all pleb:* labels)
3. Ask user for confirmation: "Kill the tmux window for this issue? (yes/no)"
4. If confirmed, kill the tmux window using: `tmux kill-window -t pleb:issue-<issue-number>`
5. Report that the issue has been abandoned and is no longer managed by pleb

## Context
- The issue will remain open on GitHub but won't have any pleb labels
- User can manually re-add `pleb:ready` label to restart work later
- Worktree is preserved to keep any useful partial work
- Killing the tmux window is optional and requires explicit confirmation
"#;

/// Slash command content for `/pleb-status`
pub const PLEB_STATUS_COMMAND: &str = r#"# Pleb Status

Show the current pleb state for this issue.

## Steps
1. Extract issue number from current directory path
2. Run: `pleb status <issue-number>`
3. Display the output to the user

## Output Format
The command will show:
- Issue number and title
- Current pleb state (ready/provisioning/waiting/working/done or "not managed")
- GitHub issue URL
"#;

/// Slash command content for `/pleb-cleanup`
pub const PLEB_CLEANUP_COMMAND: &str = r#"# Pleb Cleanup

Clean up a finished issue by removing the worktree and terminating the tmux window.

## Steps
1. Extract issue number from current directory path
2. **ALWAYS ask for confirmation first**: "This will terminate this tmux window and delete the worktree. Are you sure? (yes/no)"
3. Wait for user response
4. If user responds with anything other than "yes", abort and say "Cleanup cancelled"
5. If user confirms with "yes":
   - Warn user: "This window is about to be terminated. Goodbye!"
   - Run: `pleb cleanup <issue-number>`
   - Note: You should use /exit or stop after running cleanup since this session will be killed

## Important
- NEVER skip the confirmation step - this is a destructive operation
- Only proceed if user explicitly types "yes"
- Warn user that the window will be terminated before running the cleanup
- After cleanup, this Claude Code session will be terminated, so exit gracefully
"#;

/// Generate command file content for a given command name
pub fn generate_command_file(name: &str) -> Option<String> {
    match name {
        "pleb-shipit" => Some(PLEB_SHIPIT_COMMAND.to_string()),
        "pleb-abandon" => Some(PLEB_ABANDON_COMMAND.to_string()),
        "pleb-status" => Some(PLEB_STATUS_COMMAND.to_string()),
        "pleb-cleanup" => Some(PLEB_CLEANUP_COMMAND.to_string()),
        _ => None,
    }
}

/// Install all pleb slash commands to the specified directory
pub fn install_commands(path: &Path) -> Result<()> {
    let commands_dir = path.join(".claude").join("commands");

    // Create .claude/commands directory if it doesn't exist
    if !commands_dir.exists() {
        fs::create_dir_all(&commands_dir).with_context(|| {
            format!("Failed to create directory: {}", commands_dir.display())
        })?;
        tracing::debug!("Created commands directory at: {}", commands_dir.display());
    }

    // Install each command
    let commands = vec!["pleb-shipit", "pleb-abandon", "pleb-status", "pleb-cleanup"];
    let num_commands = commands.len();

    for cmd_name in commands {
        let content = generate_command_file(cmd_name)
            .with_context(|| format!("Unknown command: {}", cmd_name))?;

        let file_path = commands_dir.join(format!("{}.md", cmd_name));
        fs::write(&file_path, content)
            .with_context(|| format!("Failed to write {}", file_path.display()))?;

        tracing::debug!("Installed command: {}", file_path.display());
    }

    tracing::info!(
        "Installed {} Claude Code commands to: {}",
        num_commands,
        commands_dir.display()
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_command_file() {
        // Test valid command names
        assert!(generate_command_file("pleb-shipit").is_some());
        assert!(generate_command_file("pleb-abandon").is_some());
        assert!(generate_command_file("pleb-status").is_some());
        assert!(generate_command_file("pleb-cleanup").is_some());

        // Test invalid command name
        assert!(generate_command_file("invalid-command").is_none());
    }

    #[test]
    fn test_command_content() {
        // Test that command content contains expected keywords
        let shipit = generate_command_file("pleb-shipit").unwrap();
        assert!(shipit.contains("Ship It"));
        assert!(shipit.contains("pleb transition"));
        assert!(shipit.contains("done"));

        let abandon = generate_command_file("pleb-abandon").unwrap();
        assert!(abandon.contains("Abandon Issue"));
        assert!(abandon.contains("pleb transition"));
        assert!(abandon.contains("none"));

        let status = generate_command_file("pleb-status").unwrap();
        assert!(status.contains("Pleb Status"));
        assert!(status.contains("pleb status"));

        let cleanup = generate_command_file("pleb-cleanup").unwrap();
        assert!(cleanup.contains("Pleb Cleanup"));
        assert!(cleanup.contains("pleb cleanup"));
        assert!(cleanup.contains("confirmation"));
        assert!(cleanup.contains("yes"));
    }
}
