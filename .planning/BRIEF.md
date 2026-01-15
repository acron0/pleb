# Pleb

**One-liner**: Issue-driven Claude Code orchestrator that automates the work loop from GitHub issue to working code.

## Problem

The current Claude-based development workflow requires repetitive manual setup (git worktree, tmux session, invoking Claude Code) for each task, plus constant polling across tmux sessions to check if agents need attention. This ceremony and monitoring overhead limits how many concurrent features can be managed effectively.

## Success Criteria

How we know it worked:

- [x] Can manage 10 concurrent features (up from 5 manual)
- [x] Zero manual setup per issue (worktree + tmux + claude invocation automated)
- [x] External visibility via GitHub labels (no tmux polling required)
- [x] Can attach to any session anytime to watch/interact

## Constraints

- Rust (good subprocess handling, reliable for long-running daemon)
- Heavy shell integration (git worktree, tmux, claude CLI)
- Must integrate with existing `wrk` script patterns
- GitHub API for issue watching and label management
- Claude Code hooks for state transitions

## Out of Scope

What we're NOT building:

- Not a coding framework - completely project-agnostic
- Not opinionated about the projects it orchestrates
- Not a replacement for Claude Code - an orchestration layer around it
