# Milestones

## v1.0.0 - 2026-01-15

**Issue-driven Claude Code orchestrator - initial release**

### What Shipped
- GitHub issue watching with label-based state machine
- Automatic worktree + tmux provisioning per issue
- Claude Code invocation with custom prompt templates
- Hook system for state transitions (Stop, UserPromptSubmit)
- Slash commands: `/pleb-shipit`, `/pleb-abandon`, `/pleb-status`
- Daemon mode with file logging
- 36 unit tests

### Commands
```
pleb watch [--daemon]  # Start watching for issues
pleb list              # List active sessions
pleb stop              # Stop the daemon
pleb log               # Tail the log file
pleb attach            # Attach to tmux session
pleb transition        # Transition issue state
pleb status            # Show issue state
pleb hooks generate    # Generate hooks JSON
pleb hooks install     # Install hooks to project
```

### State Machine
```
ready → provisioning → waiting ⇄ working → done
```

### Success Criteria Met
- [x] Can manage 10 concurrent features
- [x] Zero manual setup per issue
- [x] External visibility via GitHub labels
- [x] Can attach to any session anytime

### Stats
- 11 source files
- 3,208 lines of Rust
- 36 unit tests
- 7 phases completed
