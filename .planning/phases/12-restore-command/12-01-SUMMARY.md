# Phase 12 Plan 1: Restore Command Summary

**Added `pleb restore` command to verify and recreate missing sessions**

## Accomplishments
- Added Restore command variant to CLI enum in src/cli.rs
- Implemented handle_restore_command function in src/main.rs that:
  - Fetches all issues with managed labels (working, waiting, done, finished)
  - Deduplicates issues that may have multiple labels
  - Checks each issue for missing tmux window or worktree
  - Recreates missing infrastructure using existing create_worktree/create_window functions
  - Installs hooks and copies pleb.toml to restored worktrees
  - Does NOT invoke Claude, process media, or change labels (infrastructure-only restoration)
  - Prints summary of checked and restored issues
- Wired up Restore command in handle_command match statement

## Files Created/Modified
- `src/cli.rs` - Added Restore command variant
- `src/main.rs` - Added handle_restore_command implementation and wired up command handler

## Decisions Made
None

## Issues Encountered
- Initial compilation error: borrowed moved value when iterating over all_issues vector
- Resolution: Changed `for issue in all_issues` to `for issue in &all_issues` to iterate over reference instead of moving the value

## Next Step
Phase complete, ready for next phase
