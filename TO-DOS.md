# TO-DOS

## Config Parent Search Path Resolution - 2026-01-19 18:46

- **Fix relative path resolution when config found in parent** - Change CWD to config file location when pleb.toml is found in a parent directory. **Problem:** Relative paths in pleb.toml (like `worktree_base = "../monorepo-branches"`) don't resolve correctly when pleb runs from a worktree subdirectory - they're relative to where the config lives, not CWD. **Files:** `src/config.rs:182-223` (find_and_load and find_config functions). **Solution:** After finding config in parent, call `std::env::set_current_dir()` to the config's directory before returning.
