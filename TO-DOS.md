# TO-DOS

## Lima VM Session Isolation - 2026-01-21 09:43

- **Use Lima for session isolation** - Run Claude Code sessions inside Lima VMs for sandboxed execution. **Problem:** Need stronger isolation than current --dangerously-skip-permissions for running untrusted Claude sessions. **Files:** `src/main.rs`, `src/tmux.rs` (session provisioning logic). **Solution:** Evaluate Lima VM setup, use limactl to provision lightweight VMs, configure filesystem mounts for worktrees, integrate with pleb's tmux workflow - Lima may be simpler than Firecracker for this use case.

## Interactive Issue Creation Command - 2026-01-20 14:08

- **Add `pleb new` command for interactive issue creation** - Launch interactive prompt that asks user for issue description, generates title, creates GitHub issue with `pleb:ready` label. **Problem:** Currently must manually create issues in GitHub UI before pleb can pick them up. **Files:** `src/cli.rs` (add New command), `src/main.rs` (handle_new_command), `src/github.rs` (create_issue method). **Solution:** Use stdin prompt or editor for description, optionally use Claude to generate title from description, call GitHub API to create issue with label. **Alternative:** Could be a slash command instead of CLI command - Claude could ask clarifying questions and construct a proper issue report with title, description, and acceptance criteria before creating it via `gh issue create`.

## Quick Session Without Issue - 2026-01-20 14:09

- **Add `pleb quick <branch-name>` command** - Provision tmux window + worktree without a GitHub issue. **Problem:** Sometimes need ad-hoc Claude sessions for quick tasks that don't warrant a full issue. **Files:** `src/cli.rs` (add Quick command), `src/main.rs` (handle_quick_command), `src/tmux.rs`, `src/worktree.rs`. **Solution:** Create worktree with given branch name, tmux window named after branch, invoke Claude with minimal prompt, skip all GitHub label management.

## Restore Command for Session Verification - 2026-01-23 14:26

- **Add `pleb restore` command** - Check that all pleb-associated issues have a tmux session and worktree, recreate missing ones. **Problem:** Sessions can be lost due to daemon restarts, crashes, or manual tmux/worktree cleanup, leaving issues in working/waiting/done states without active environments. **Files:** `src/cli.rs` (add Restore command), `src/main.rs` (handle_restore_command), `src/tmux.rs` (window_exists), `src/worktree.rs` (worktree_exists). **Solution:** Fetch all issues with working/waiting/done labels, check if tmux window and worktree exist for each, recreate missing sessions using existing process_issue logic, log which issues were restored.

