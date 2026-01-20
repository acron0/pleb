# TO-DOS

## String Interpolation for Provision Hooks - 2026-01-19 19:30

- **Add template variables to on_provision commands** - Support interpolation like `{{issue_number}}`, `{{worktree_path}}` in provision hook commands. **Problem:** Users can't dynamically reference issue context in their provision hooks. **Files:** `src/main.rs` (process_issue on_provision loop), `src/templates.rs` (reuse IssueContext). **Solution:** Use Handlebars to render each command before sending to tmux.

## Firecracker MicroVM for Claude - 2026-01-20 13:27

- **Research running Claude inside Firecracker microVM** - Investigate feasibility and approach for sandboxing Claude Code execution in Firecracker microVMs. **Problem:** Need stronger isolation than current --dangerously-skip-permissions for running untrusted Claude sessions. **Files:** N/A (research task). **Solution:** Evaluate Firecracker setup, networking, filesystem sharing, and integration with pleb's tmux/worktree workflow.

## Interactive Issue Creation Command - 2026-01-20 14:08

- **Add `pleb new` command for interactive issue creation** - Launch interactive prompt that asks user for issue description, generates title, creates GitHub issue with `pleb:ready` label. **Problem:** Currently must manually create issues in GitHub UI before pleb can pick them up. **Files:** `src/cli.rs` (add New command), `src/main.rs` (handle_new_command), `src/github.rs` (create_issue method). **Solution:** Use stdin prompt or editor for description, optionally use Claude to generate title from description, call GitHub API to create issue with label.

## Quick Session Without Issue - 2026-01-20 14:09

- **Add `pleb quick <branch-name>` command** - Provision tmux window + worktree without a GitHub issue. **Problem:** Sometimes need ad-hoc Claude sessions for quick tasks that don't warrant a full issue. **Files:** `src/cli.rs` (add Quick command), `src/main.rs` (handle_quick_command), `src/tmux.rs`, `src/worktree.rs`. **Solution:** Create worktree with given branch name, tmux window named after branch, invoke Claude with minimal prompt, skip all GitHub label management.

## Enhanced Status Command - 2026-01-20 14:12

- **Enhance `pleb status` with daemon info and issue listing** - Show daemon status (running, uptime, PID) and list all issues when no issue number specified. **Problem:** Current `pleb status` only shows single issue state, no way to see daemon health or overview of all tracked issues. **Files:** `src/cli.rs` (make issue_number optional), `src/main.rs` (handle_status_command). **Solution:** Read PID file to check daemon status, calculate uptime from file mtime or process start time, list all issues with pleb labels when no issue specified.
