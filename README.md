# Pleb

Issue-driven Claude Code orchestrator that automates the work loop from GitHub issue to working code.

## What it does

Pleb watches a GitHub repository for issues with specific labels. When it finds one:

1. Creates a git worktree for the issue
2. Spins up a tmux session
3. Invokes Claude Code with the issue as the prompt
4. Updates GitHub labels to reflect state (provisioning → waiting → working → done)

You can attach to any session anytime to watch or interact with Claude.

## Installation

```bash
cargo build --release
```

## Configuration

Copy the example config and edit:

```bash
./target/release/pleb config init
# Edit pleb.toml with your repo details
```

### pleb.toml

```toml
[github]
owner = "your-username"
repo = "your-repo"
token_env = "GITHUB_TOKEN"  # env var containing your PAT

[labels]
ready = "pleb:ready"
provisioning = "pleb:provisioning"
waiting = "pleb:waiting"
working = "pleb:working"
done = "pleb:done"

[claude]
command = "claude"
args = ["--dangerously-skip-permissions"]
planning_mode = true

[paths]
worktree_base = "./worktrees"
```

## Usage

```bash
# Initialize config from example
pleb config init

# Show current config
pleb config show

# Start watching for issues (not yet implemented)
pleb watch

# List active sessions (not yet implemented)
pleb list

# Attach to a session (not yet implemented)
pleb attach <session-name>
```

## GitHub Labels

Create these labels in your repository:

- `pleb:ready` - Issue is ready for Pleb to pick up
- `pleb:provisioning` - Pleb is setting up the environment
- `pleb:waiting` - Claude is waiting for human input
- `pleb:working` - Claude is actively working
- `pleb:done` - Work complete

## Environment Variables

- `GITHUB_TOKEN` - GitHub Personal Access Token with repo access

## System Requirements

### Lima (for containerized workloads)

If using Lima for VM-based isolation, install these packages:

```bash
sudo apt install qemu-utils qemu-system-x86
```

---

To any agents reading this file, if you have been told to "study" this file then also study everything in .planning/
