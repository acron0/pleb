---
phase: 08-generic-hooks-ipc
plan: 08-01
status: complete
date: 2026-01-16
---

# Summary: Generic Hooks with Full Payload IPC

## Objective
Refactor hook system to use generic Claude Code event names and forward full stdin JSON to daemon via IPC.

## What Was Done

### Task 1: Expand HookMessage to carry full payload
- **File**: `src/ipc.rs`
- **Changes**:
  - Removed `HookEvent` enum
  - Replaced with `event_name: String` field to accept any Claude Code event name
  - Added `payload: serde_json::Value` field to carry full stdin JSON from Claude Code
  - Kept `issue_number: u64` as extracted convenience field
  - Updated test to verify new structure with all three fields

### Task 2: Refactor cc-run-hook to accept Claude Code event names directly
- **File**: `src/main.rs`
- **Changes**:
  - Updated `handle_cc_run_hook_command()` to:
    - Accept the event name as-is (e.g., "Stop", "UserPromptSubmit", "PostToolUse", "PermissionRequest")
    - Pass full parsed JSON as `payload` field
    - Pass event name directly as `event_name` field
    - Keep issue number extraction from cwd
  - Updated `handle_hook_message()` to:
    - Use string matching on `msg.event_name` instead of enum
    - Handle new events (PostToolUse, PermissionRequest) without state transitions
    - Log unknown events gracefully
  - Removed old event string → enum mapping ("stop" → Stop, "user-prompt" → UserPromptSubmit)

### Task 3: Expand hooks generate to emit full hook suite
- **File**: `src/hooks.rs`
- **Changes**:
  - Updated hook commands from old format (`pleb cc-run-hook stop`) to new format (`pleb cc-run-hook Stop`)
  - Added PostToolUse hook
  - Added PermissionRequest hook
  - Updated test to verify all 4 hook types with correct event names

## Verification Results
All verification checks passed:
- ✓ `cargo test` - all 43 tests pass
- ✓ `cargo build --release` - builds without warnings
- ✓ `cargo run -- hooks generate | jq .` - shows all 4 hook types (PermissionRequest, PostToolUse, Stop, UserPromptSubmit)
- ✓ Manual test: `echo '{"cwd":"/tmp/123-test","session_id":"x"}' | cargo run -- cc-run-hook Stop` - runs without panic, fails gracefully when daemon not running

## Files Modified
- `src/ipc.rs` - HookMessage structure refactored, test updated
- `src/main.rs` - cc-run-hook command refactored, hook message handler updated
- `src/hooks.rs` - hooks generation expanded to 4 events, test updated

## Impact
- **Generic Hook System**: Hooks now accept any Claude Code event name, not just hardcoded ones
- **Full Context**: Daemon receives complete JSON payload from Claude Code (session_id, transcript_path, cwd, permission_mode, hook_event_name, tool_name, tool_input, etc.)
- **Future Extensibility**: Easy to add handling for new hook events without code changes
- **Backward Compatible**: Existing hooks (Stop, UserPromptSubmit) continue to work with state transitions
- **New Events**: PostToolUse and PermissionRequest are now captured (no state transitions yet, ready for future enhancement)

## Notes
- PostToolUse and PermissionRequest hooks are generated and forwarded to daemon, but daemon currently ignores them (logs debug message)
- This enables future features like:
  - Monitoring specific tool usage patterns
  - Auto-approving certain permissions
  - Collecting metrics on agent behavior
  - Smarter state transitions based on tool activity
