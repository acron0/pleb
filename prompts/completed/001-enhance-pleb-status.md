<objective>
Enhance the `pleb status` command to show daemon info and list all issues when no issue number is specified.

Currently `pleb status` requires an issue number and only shows single issue state. After this change:
- `pleb status` (no args) → shows daemon health + lists all pleb-managed issues
- `pleb status <issue_number>` → shows single issue state (existing behavior)
</objective>

<context>
This is a Rust CLI tool using clap for argument parsing. The codebase follows standard Rust patterns.

Key files to examine:
- `src/cli.rs` - Command definitions using clap derive macros (line 56-60 has Status command)
- `src/main.rs` - Command handlers (line 650-682 has handle_status_command)
- `src/config.rs` - Has `pid_file()` and `daemon_dir()` methods for locating daemon files

The daemon writes its PID to `pleb.pid` and logs to `pleb.log` in the daemon directory.
GitHub issues with pleb labels can be fetched via the existing `GitHubClient`.
</context>

<requirements>
1. Make `issue_number` optional in the Status command (cli.rs line 59)
2. Modify `handle_status_command` to handle both cases:

   **When no issue number provided:**
   - Check if daemon is running by reading PID file and checking if process exists
   - Show daemon status: running/stopped, PID (if running), uptime (from PID file mtime)
   - Fetch and list all issues with any pleb label (ready, provisioning, waiting, working, done)
   - For each issue, show: issue number, title (truncated), current pleb state

   **When issue number provided:**
   - Keep existing behavior unchanged

3. Output format for `pleb status` (no args):
   ```
   Daemon: running (PID: 12345, uptime: 2h 15m)

   Managed Issues:
     #123 [working]   Fix authentication bug
     #124 [waiting]   Add user dashboard
     #125 [ready]     Update documentation

   Use 'pleb status <issue_number>' for detailed issue info.
   ```

   Or if daemon not running:
   ```
   Daemon: stopped

   No active daemon. Start with 'pleb watch --daemon'.
   ```
</requirements>

<implementation>
1. In `cli.rs`, change Status command's issue_number to `Option<u64>`
2. In `main.rs`, update the match arm for Status to pass the Option
3. In `handle_status_command`:
   - If `issue_number.is_some()` → existing single-issue logic
   - If `issue_number.is_none()` → new daemon + issue list logic
4. Add helper function to check daemon status using PID file (similar to stop command logic in lines 824-873)
5. Fetch issues with pleb labels using GitHubClient - you may need to fetch issues for each label or add a method to fetch by multiple labels
</implementation>

<verification>
Before declaring complete, verify:
- `pleb status` shows daemon info and issue list (or appropriate message if daemon stopped)
- `pleb status 123` still shows single issue details as before
- Code compiles with `cargo build`
- No clippy warnings with `cargo clippy`
</verification>

<success_criteria>
- Optional issue_number argument implemented correctly
- Daemon status display shows running/stopped, PID, and uptime
- All pleb-managed issues listed with their states
- Existing single-issue status behavior preserved
- Clean Rust code following project conventions
</success_criteria>
