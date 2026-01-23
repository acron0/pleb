---
phase: 11-pr-merge-cleanup
plan: 02
completed: 2026-01-23
---

# Summary: Add PR merge detection in watch loop

## Objective
Implement PR merge detection in the daemon watch loop to automatically detect when a PR associated with an issue has been merged and transition to "finished" state.

## Tasks Completed

### Task 1: Add PR merge check to GitHub client
- Added `check_pr_merged(&self, issue_number: u64) -> Result<Option<bool>>` method to GitHubClient in src/github.rs
- Returns Ok(None) if no PR found for this issue
- Returns Ok(Some(true)) if PR exists and is merged (checks both state=="MERGED" and mergedAt field)
- Returns Ok(Some(false)) if PR exists but not merged
- Uses `gh pr list --state all` to fetch PRs including merged ones
- Filters by branch prefix matching pleb's naming convention: `{issue_number}-`
- Handles errors gracefully (gh not installed, network issues) - logs warning, returns Ok(None)
- Implementation uses `gh` CLI which has its own authentication

**Files modified:**
- `/home/acron/projects/kikin/pleb/src/github.rs`

### Task 2: Detect merged PRs in watch loop
- Added `check_merged_prs(&self) -> Result<()>` method to Orchestrator in src/main.rs
- Fetches issues with working, waiting, AND done labels (all active states)
- For each issue, calls `github.check_pr_merged(issue.number)`
- If merged: transitions to Finished state, updates tmux window title to "finished"
- Added call to `check_merged_prs()` in the poll loop after `poll_cycle()`
- Runs at same frequency as poll cycle
- Logs transitions: "Issue #X PR merged, transitioning from {:?} to Finished"
- Handles errors gracefully: logs error and continues with next issue

**Files modified:**
- `/home/acron/projects/kikin/pleb/src/main.rs`

### Task 3: Update pleb status for finished state
This task was already completed in plan 11-01:
- `handle_status_command()` already handles Finished state display in single-issue branch (line 759)
- Daemon-status branch already includes finished label in labels list to fetch (line 810)
- State_str mapping already includes "[finished]" (line 837)
- Finished issues already appear in "Managed Issues" list

**No changes needed for this task.**

## Verification Results

All verification checks passed:

### cargo build
```
Finished `dev` profile [unoptimized + debuginfo] target(s) in 7.34s
```

### cargo test
```
running 68 tests
test result: ok. 68 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

All tests passed, no regressions.

### cargo clippy
```
Finished `dev` profile [unoptimized + debuginfo] target(s) in 5.37s
```

No warnings from clippy.

## Files Modified

- `/home/acron/projects/kikin/pleb/src/github.rs` - Added check_pr_merged() method
- `/home/acron/projects/kikin/pleb/src/main.rs` - Added check_merged_prs() method and integrated into poll loop

## Success Criteria Met

- [x] Daemon polls all active issues (working/waiting/done) for merged PRs
- [x] When PR is merged, issue transitions to "finished" automatically
- [x] Tmux window title updates to "finished"
- [x] `pleb status` displays finished state correctly (already implemented in 11-01)
- [x] No errors when PR doesn't exist or gh CLI unavailable
- [x] cargo build succeeds
- [x] cargo test passes all tests (68/68)
- [x] cargo clippy has no warnings

## Implementation Details

### PR Merge Detection Logic
The `check_pr_merged()` method uses `gh pr list --state all` to fetch all PRs (including merged ones) and checks:
1. Branch name matches pattern: `{issue_number}-*`
2. State is "MERGED" OR mergedAt field is not null

This dual check ensures we catch merged PRs reliably.

### Error Handling
Both the GitHub client method and the orchestrator method handle errors gracefully:
- gh command failures: log warning, return Ok(None), don't crash daemon
- Network issues: log error, continue with next issue
- Parse failures: log warning, return Ok(None)

This ensures the daemon continues running even if there are temporary issues with gh CLI or network.

### Poll Frequency
The PR merge check runs after every poll cycle, at the same frequency as the ready issue check (configurable via `watch.poll_interval_secs`). This ensures merged PRs are detected promptly.

## Next Steps

The next plan in this phase (11-03) will:
- Implement the `pleb cleanup` command to remove worktrees and tmux sessions for finished issues
- Create a `/pleb-cleanup` Claude Code slash command for easy access
- Handle edge cases like active tmux sessions and dirty worktrees

## Notes

- The implementation correctly skips issues that have just been provisioned (no PR yet) by returning Ok(None) when no PR is found
- The tmux window title update provides immediate visual feedback when a PR is merged
- The finished state is now properly tracked and displayed in both single-issue and daemon status views
