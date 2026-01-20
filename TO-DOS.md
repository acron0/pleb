# TO-DOS

## String Interpolation for Provision Hooks - 2026-01-19 19:30

- **Add template variables to on_provision commands** - Support interpolation like `{{issue_number}}`, `{{worktree_path}}` in provision hook commands. **Problem:** Users can't dynamically reference issue context in their provision hooks. **Files:** `src/main.rs` (process_issue on_provision loop), `src/templates.rs` (reuse IssueContext). **Solution:** Use Handlebars to render each command before sending to tmux.

## Firecracker MicroVM for Claude - 2026-01-20 13:27

- **Research running Claude inside Firecracker microVM** - Investigate feasibility and approach for sandboxing Claude Code execution in Firecracker microVMs. **Problem:** Need stronger isolation than current --dangerously-skip-permissions for running untrusted Claude sessions. **Files:** N/A (research task). **Solution:** Evaluate Firecracker setup, networking, filesystem sharing, and integration with pleb's tmux/worktree workflow.
