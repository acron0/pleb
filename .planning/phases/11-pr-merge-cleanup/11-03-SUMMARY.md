---
phase: 11-pr-merge-cleanup
plan: 03
completed: 2026-01-23
---

# Summary: Add cleanup command and /pleb-cleanup slash command

## Objective
Implement cleanup functionality via CLI command and slash command to allow users to clean up worktrees and tmux sessions for finished issues, with explicit confirmation.

## Tasks Completed

### Task 1: Add cleanup CLI command and daemon handler
- Added `Cleanup { issue_number: u64 }` variant to Commands enum in src/cli.rs
- Added `handle_cleanup_command()` function in src/main.rs that:
  - Creates WorktreeManager and TmuxManager instances directly (no daemon needed)
  - Checks if worktree and tmux window exist
  - Removes worktree using `worktree.remove_worktree()`
  - Kills tmux window using `tmux.kill_window()`
  - Prints confirmation of what was cleaned up
- Updated `handle_command()` to dispatch Cleanup command to the handler
- Note: `remove_worktree()` and `kill_window()` methods already existed in their respective managers from previous work

**Files modified:**
- `/home/acron/projects/kikin/pleb/src/cli.rs`
- `/home/acron/projects/kikin/pleb/src/main.rs`

### Task 2: Create /pleb-cleanup slash command
- Added `PLEB_CLEANUP_COMMAND` constant with markdown instructions in src/commands.rs
- Instructions include:
  - Extract issue number from current directory path
  - **ALWAYS ask for confirmation first**: "This will terminate this tmux window and delete the worktree. Are you sure? (yes/no)"
  - Wait for user response
  - Only proceed if user confirms with "yes"
  - Warn user: "This window is about to be terminated. Goodbye!"
  - Run: `pleb cleanup <issue-number>`
  - Note that Claude should exit after running cleanup since the session will be killed
- Added "pleb-cleanup" to `generate_command_file()` match statement
- Added "pleb-cleanup" to `install_commands()` commands list (4 commands total now)
- Updated tests:
  - Added pleb-cleanup to `test_generate_command_file()`
  - Added test case in `test_command_content()` to verify cleanup command contains required keywords: "Pleb Cleanup", "pleb cleanup", "confirmation", "yes"

**Files modified:**
- `/home/acron/projects/kikin/pleb/src/commands.rs`

## Verification Results

All verification checks passed:

### cargo build
```
Finished `dev` profile [unoptimized + debuginfo] target(s) in 2.36s
```

No warnings or errors.

### cargo test
```
running 68 tests
test result: ok. 68 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

All tests passed including new tests for pleb-cleanup command.

### cargo clippy
```
Finished `dev` profile [unoptimized + debuginfo] target(s) in 2.38s
```

No warnings from clippy.

## Files Modified

- `/home/acron/projects/kikin/pleb/src/cli.rs` - Added Cleanup command variant
- `/home/acron/projects/kikin/pleb/src/main.rs` - Added handle_cleanup_command() and command dispatch
- `/home/acron/projects/kikin/pleb/src/commands.rs` - Added /pleb-cleanup slash command with confirmation requirement

## Success Criteria Met

- [x] `pleb cleanup <issue>` CLI command works standalone
- [x] `/pleb-cleanup` slash command always asks for confirmation before proceeding
- [x] Cleanup removes the git worktree (via existing remove_worktree method)
- [x] Cleanup kills the tmux window (via existing kill_window method)
- [x] User is warned that their session is about to terminate
- [x] All tests pass (68/68)
- [x] cargo build succeeds
- [x] cargo clippy has no warnings

## Implementation Details

### CLI Command Implementation
The `pleb cleanup <issue-number>` command runs standalone without requiring the daemon:
1. Creates managers directly (WorktreeManager and TmuxManager)
2. Checks if worktree exists using `get_worktree_path()`
3. Checks if tmux window exists using `window_exists()`
4. Removes worktree if it exists using `remove_worktree()` (deletes worktree and branch)
5. Kills tmux window if it exists using `kill_window()`
6. Prints confirmation of what was cleaned up

### Slash Command Implementation
The `/pleb-cleanup` slash command enforces a strict confirmation workflow:
1. Extracts issue number from current directory path
2. **ALWAYS asks for confirmation first** before proceeding
3. Only proceeds if user explicitly types "yes"
4. Warns user that the window is about to be terminated
5. Runs `pleb cleanup <issue-number>`
6. Notes that Claude should exit after running cleanup

The confirmation requirement is emphasized multiple times in the command instructions to ensure Claude never skips this critical safety check.

## Next Steps

Phase 11 is now complete. The PR merge detection and cleanup workflow is fully implemented:
1. Daemon detects merged PRs and transitions issues to "finished" state (11-01, 11-02)
2. Users can manually clean up finished issues with confirmation (11-03)

Potential future enhancements:
- Automatic cleanup of finished issues (with configurable delay)
- Bulk cleanup command to clean up all finished issues
- Integration with GitHub issue closing (though PRs already auto-close issues)

## Notes

- The cleanup command is destructive but safe: it requires explicit invocation and the slash command requires explicit confirmation
- The tmux window is terminated after cleanup, so users should save any work before running this command
- The worktree removal also deletes the local branch (using `git branch -D`)
- If either worktree or window doesn't exist, cleanup gracefully skips that step and continues
- The implementation reuses existing methods from WorktreeManager and TmuxManager, keeping the code DRY
