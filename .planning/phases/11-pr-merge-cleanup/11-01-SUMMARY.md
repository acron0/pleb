---
phase: 11-pr-merge-cleanup
plan: 01
completed: 2026-01-23
---

# Summary: Add finished state foundation

## Objective
Add the "finished" state foundation - config and state machine support to enable tracking of issues whose PRs have been merged, distinct from "done" (PR created but not yet merged).

## Tasks Completed

### Task 1: Add finished label to LabelConfig
- Added `finished` field to `LabelConfig` struct with serde default
- Added `default_label_finished()` function returning "pleb:finished"
- Updated label validation in `Config::validate()` to include `finished` in the uniqueness check
- Updated test configs (MINIMAL_CONFIG, FULL_CONFIG) to include the new field
- Added test `test_parse_custom_finished_label` for custom finished label parsing
- Updated `test_defaults_applied` to verify finished label default
- Updated `test_parse_full_config` to check custom finished label

**Files modified:**
- `/home/acron/projects/kikin/pleb/src/config.rs`

### Task 2: Add Finished state to PlebState
- Added `Finished` variant to `PlebState` enum in state.rs
- Updated `valid_transitions()` to support transitions to Finished:
  - Working → Finished (PR merged while working)
  - Waiting → Finished (PR merged while waiting)
  - Done → Finished (PR merged after PR created)
- Updated `is_terminal()` - Finished is now the only terminal state (Done is no longer terminal)
- Updated `state_to_label()` in github.rs to map `Finished` to `config.labels.finished`
- Updated `get_pleb_state()` in github.rs to detect finished label
- Updated `parse_state()` in main.rs to parse "finished" string
- Updated `handle_transition_command()` in main.rs to include finished label in "none" removal list
- Updated `handle_status_command()` in main.rs to display finished state in both single-issue and daemon status modes
- Added unit tests:
  - `test_transition_to_finished` - tests transitions from Working, Waiting, and Done to Finished
  - Updated `test_valid_transitions` to reflect new transition paths
  - Updated `test_is_terminal` to verify Finished is terminal and Done is not
  - Updated `test_terminal_state_transition` to test Finished as terminal state

**Files modified:**
- `/home/acron/projects/kikin/pleb/src/state.rs`
- `/home/acron/projects/kikin/pleb/src/github.rs`
- `/home/acron/projects/kikin/pleb/src/main.rs`

### Bug Fixes
- Fixed clippy warning: removed needless borrow in `handle_cc_run_hook_command` call

## Verification Results

All verification checks passed:

### cargo build
```
Finished `dev` profile [unoptimized + debuginfo] target(s) in 4m 38s
```

### cargo test
```
running 68 tests
test result: ok. 68 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

All tests passed including new tests for finished state transitions.

### cargo clippy
```
Finished `dev` profile [unoptimized + debuginfo] target(s) in 2.55s
```

No warnings after fixing the needless borrow issue.

## Files Modified

- `/home/acron/projects/kikin/pleb/src/config.rs` - Added finished label configuration
- `/home/acron/projects/kikin/pleb/src/state.rs` - Added Finished state and updated transitions
- `/home/acron/projects/kikin/pleb/src/github.rs` - Added finished state label mapping
- `/home/acron/projects/kikin/pleb/src/main.rs` - Updated command handlers for finished state

## Success Criteria Met

- [x] LabelConfig includes `finished` with default "pleb:finished"
- [x] PlebState::Finished exists and is a terminal state
- [x] Transitions to Finished are valid from Working, Waiting, and Done
- [x] All label/state mappings updated consistently
- [x] All tests pass (68/68)
- [x] cargo build succeeds
- [x] cargo clippy has no warnings

## Next Steps

This plan provides the foundation for PR merge detection. The next plans in this phase will:
- **11-02**: Add PR merge detection logic to the watch loop
- **11-03**: Implement cleanup command and /pleb-cleanup slash command to remove worktrees/sessions for finished issues

## Notes

The state transition logic was updated to make Finished the only terminal state, while Done now transitions to Finished. This reflects the real-world workflow where:
1. Done = PR created but not yet merged
2. Finished = PR has been merged

This distinction will enable automatic cleanup of worktrees and sessions once PRs are merged in subsequent plans.
