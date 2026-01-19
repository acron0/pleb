# Phase 10 Plan 1: Provision Hooks Summary

**Configurable shell commands that run after tmux window creation but before Claude starts**

## Accomplishments
- Added `ProvisionConfig` struct with `on_provision: Vec<String>` field
- Added `[provision]` section to config with `#[serde(default)]`
- Enabled `send_keys` method in `TmuxManager` (removed dead_code annotation)
- Execute on_provision commands in `process_issue` after window creation, before Claude invocation
- Added config tests for default empty and explicit command list parsing

## Files Created/Modified
- `src/config.rs` - Added `ProvisionConfig` struct, `provision` field to `Config`, new tests
- `src/tmux.rs` - Removed `#[allow(dead_code)]` from `send_keys` method
- `src/main.rs` - Added on_provision command execution loop in `process_issue`

## Usage

```toml
[provision]
on_provision = [
  "tmux split-window -h",
  "tmux send-keys -t {next} './my-watch-script.sh' Enter"
]
```

Commands run:
- In sequence with 100ms delay between each
- In the tmux window's working directory (the worktree)
- After window creation, before media download and Claude invocation

## Decisions Made
- Used shell strings (not separated args) since commands run in shell context via tmux send-keys
- 100ms delay between commands to allow each to start before next
- Non-fatal: if a command fails, it doesn't block provisioning (tmux send-keys doesn't report command exit status)

## Deviations from Plan
None - all tasks completed as specified.

## Issues Encountered
None.

---
*Phase: 10-provision-hooks*
*Completed: 2026-01-19*
