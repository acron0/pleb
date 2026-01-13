# Roadmap: Pleb

## Overview

Build an issue-driven Claude Code orchestrator that watches GitHub for labeled issues, automatically provisions work environments (git worktree + tmux), and manages the Claude Code work loop with external visibility via GitHub labels.

## Phases

- [ ] **Phase 1: Foundation** - Rust project, config system, CLI interface
- [ ] **Phase 2: GitHub Integration** - Issue watching, label management, API client
- [ ] **Phase 3: Session Management** - Git worktree + tmux provisioning
- [ ] **Phase 4: Orchestration** - Main daemon loop, state machine
- [ ] **Phase 5: Hooks & Skills** - Claude Code hooks, built-in skills

## Phase Details

### Phase 1: Foundation
**Goal**: Bootable Rust project with config parsing and CLI structure
**Depends on**: Nothing (first phase)
**Plans**: TBD after planning

Plans:
- [ ] 01-01: Rust project scaffold, dependencies, basic CLI
- [ ] 01-02: Config system (TOML/YAML for repo, labels, claude flags)

### Phase 2: GitHub Integration
**Goal**: Can watch a repo for issues with specific labels and modify labels
**Depends on**: Phase 1 (needs config)
**Plans**: TBD after planning

Plans:
- [ ] 02-01: GitHub API client with auth (PAT or GitHub App)
- [ ] 02-02: Issue watching (polling loop, label filtering)
- [ ] 02-03: Label management (add/remove labels on issues)

### Phase 3: Session Management
**Goal**: Can create worktrees and tmux sessions, list active sessions
**Depends on**: Phase 1 (needs config for paths)
**Plans**: TBD after planning

Plans:
- [ ] 03-01: Git worktree creation/cleanup
- [ ] 03-02: Tmux session management (create, attach, list)

### Phase 4: Orchestration
**Goal**: Main daemon that ties everything together with state machine
**Depends on**: Phases 2, 3
**Plans**: TBD after planning

Plans:
- [ ] 04-01: State machine (ready → provisioning → waiting → working)
- [ ] 04-02: Main daemon loop (watch → provision → invoke → manage)
- [ ] 04-03: Claude Code invocation with issue as prompt

### Phase 5: Hooks & Skills
**Goal**: Integration points for Claude Code state transitions and convenience commands
**Depends on**: Phase 4
**Plans**: TBD after planning

Plans:
- [ ] 05-01: Claude Code hooks for label state transitions
- [ ] 05-02: Built-in skills (/shipit, etc.)

## Progress

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 1. Foundation | 0/2 | Not started | - |
| 2. GitHub Integration | 0/3 | Not started | - |
| 3. Session Management | 0/2 | Not started | - |
| 4. Orchestration | 0/3 | Not started | - |
| 5. Hooks & Skills | 0/2 | Not started | - |
