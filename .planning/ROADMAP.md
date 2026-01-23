# Roadmap: Pleb

## Overview

Build an issue-driven Claude Code orchestrator that watches GitHub for labeled issues, automatically provisions work environments (git worktree + tmux), and manages the Claude Code work loop with external visibility via GitHub labels.

## Phases

- [x] **Phase 1: Foundation** - Rust project, config system, CLI interface
- [x] **Phase 2: GitHub Integration** - Issue watching, label management, API client
- [x] **Phase 3: Session Management** - Git worktree + tmux provisioning
- [x] **Phase 4: Orchestration** - Main daemon loop, state machine
- [x] **Phase 5: Hooks & Skills** - Claude Code hooks, built-in skills
- [x] **Phase 6: Daemon Mode** - Background daemon with file logging
- [x] **Phase 7: Unit Tests** - Pure function unit tests for untested modules
- [x] **Phase 8: Generic Hooks & IPC** - Generic hook events, full payload forwarding to daemon
- [x] **Phase 9: Issue Media Downloading** - Download images/videos from GitHub issues to local files
- [x] **Phase 10: Provision Hooks** - Configurable shell commands run after window creation, before Claude
- [ ] **Phase 11: PR Merge Detection & Cleanup** - Track merged PRs, new "finished" state, cleanup command

## Phase Details

### Phase 1: Foundation
**Goal**: Bootable Rust project with config parsing and CLI structure
**Depends on**: Nothing (first phase)
**Status**: Complete

Plans:
- [x] 01-01: Rust project scaffold, dependencies, basic CLI
- [x] 01-02: Config system (TOML/YAML for repo, labels, claude flags)

### Phase 2: GitHub Integration
**Goal**: Can watch a repo for issues with specific labels and modify labels
**Depends on**: Phase 1 (needs config)
**Status**: Complete

Plans:
- [x] 02-01: GitHub API client with auth (PAT or GitHub App)
- [x] 02-02: Issue watching (polling loop, label filtering)
- [x] 02-03: Label management (add/remove labels on issues)

### Phase 3: Session Management
**Goal**: Can create worktrees and tmux sessions, list active sessions
**Depends on**: Phase 1 (needs config for paths)
**Status**: Complete

Plans:
- [x] 03-01: Git worktree creation/cleanup
- [x] 03-02: Tmux session management (create, attach, list)

### Phase 4: Orchestration
**Goal**: Main daemon that ties everything together with state machine
**Depends on**: Phases 2, 3
**Status**: Complete

Plans:
- [x] 04-01: State machine (ready → provisioning → waiting → working)
- [x] 04-02: Prompt template system (prompts/ dir, Handlebars templating, config for prompt paths)
- [x] 04-03: Claude Code invocation within tmux windows
- [x] 04-04: Main daemon loop (watch → provision → invoke → manage)

### Phase 5: Hooks & Skills
**Goal**: Integration points for Claude Code state transitions and convenience commands
**Depends on**: Phase 4
**Status**: Complete

Plans:
- [x] 05-01: Hook Infrastructure (`pleb transition`, `pleb cc-run-hook`, `pleb hooks generate|install`, auto-install during provisioning)
- [x] 05-02: Slash Commands (`/pleb-shipit`, `/pleb-abandon`, `/pleb-status`)

### Phase 6: Daemon Mode
**Goal**: Run pleb as a background daemon with file logging
**Depends on**: Phase 4 (needs watch command)
**Status**: Complete

Plans:
- [x] 06-01: Daemon mode (`--daemon` flag, file logging, PID file, `pleb log` command)

### Phase 7: Unit Tests
**Goal**: Unit test coverage for pure functions in untested modules
**Depends on**: Nothing (independent)
**Status**: Complete

Plans:
- [x] 07-01: Config module tests (TOML parsing, defaults, validation logic)
- [x] 07-02: Template + coverage review (IssueContext tests, expand existing tests)

### Phase 8: Generic Hooks & IPC
**Goal**: Refactor hooks to use Claude Code event names directly, forward full stdin JSON payload to daemon
**Depends on**: Phase 5 (hooks infrastructure)
**Status**: Complete

Plans:
- [x] 08-01: Generic hook command, expanded HookMessage, full hook suite generation

### Phase 9: Issue Media Downloading
**Goal**: Download images and videos from GitHub issue descriptions to local disk, replacing URLs with local file paths in prompts so Claude can view them
**Depends on**: Phase 4 (orchestration, prompt generation)
**Status**: Complete

Plans:
- [x] 09-01: Media extraction, downloading, and prompt integration

### Phase 10: Provision Hooks
**Goal**: Configurable shell commands that run after tmux window creation but before Claude starts
**Depends on**: Phase 3 (session management), Phase 4 (orchestration)
**Status**: Complete

Plans:
- [x] 10-01: ProvisionConfig, on_provision command execution in process_issue

### Phase 11: PR Merge Detection & Cleanup
**Goal**: Automatically detect merged PRs, transition to "finished" state, enable cleanup of worktrees/sessions
**Depends on**: Phase 2 (GitHub), Phase 3 (session management), Phase 5 (hooks)
**Status**: In Progress

Plans:
- [x] 11-01: Config & state foundation (finished label, Finished state)
- [x] 11-02: PR merge detection in watch loop
- [ ] 11-03: Cleanup command and /pleb-cleanup slash command

## Progress

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 1. Foundation | 2/2 | Complete | 2026-01-13 |
| 2. GitHub Integration | 3/3 | Complete | 2026-01-13 |
| 3. Session Management | 2/2 | Complete | 2026-01-13 |
| 4. Orchestration | 4/4 | Complete | 2026-01-14 |
| 5. Hooks & Skills | 2/2 | Complete | 2026-01-15 |
| 6. Daemon Mode | 1/1 | Complete | 2026-01-15 |
| 7. Unit Tests | 2/2 | Complete | 2026-01-15 |
| 8. Generic Hooks & IPC | 1/1 | Complete | 2026-01-16 |
| 9. Issue Media Downloading | 1/1 | Complete | 2026-01-16 |
| 10. Provision Hooks | 1/1 | Complete | 2026-01-19 |
| 11. PR Merge Detection & Cleanup | 2/3 | In Progress | - |
