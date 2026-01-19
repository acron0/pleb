# TO-DOS

## Update Tmux Window Names with Pleb Status - 2026-01-19 19:05

- **Add status indicator to tmux window names** - Update window names to reflect current pleb state (waiting/working). **Problem:** When managing multiple issues, it's hard to tell at a glance which windows need attention vs which are actively working. Currently all windows are just named "issue-{number}". **Files:** `src/tmux.rs` (add rename_window method), `src/main.rs` (call on state transitions). **Solution:** Rename windows to "issue-{number}-{state}" (e.g., "issue-42-waiting", "issue-42-working") when transitioning states via hooks or daemon.

## Config Parent Search Path Resolution - 2026-01-19 18:46

- **Fix relative path resolution when config found in parent** - Change CWD to config file location when pleb.toml is found in a parent directory. **Problem:** Relative paths in pleb.toml (like `worktree_base = "../monorepo-branches"`) don't resolve correctly when pleb runs from a worktree subdirectory - they're relative to where the config lives, not CWD. **Files:** `src/config.rs:182-223` (find_and_load and find_config functions). **Solution:** After finding config in parent, call `std::env::set_current_dir()` to the config's directory before returning.
