# Abandon Issue

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
