# Ship It

Create a pull request for the current work and label the issue as done.

## Steps
1. Stage and commit any uncommitted changes with a descriptive message
2. Push the current branch to origin
3. Create a pull request using `gh pr create`:
   - Title: Use the issue title or branch name
   - Body: Reference the issue number (Fixes #XXX) so GitHub auto-closes it on merge
4. Check if the branch is behind the base branch:
   - Fetch the latest from origin
   - Compare the current branch with origin/main (or the base branch)
   - If behind, update the branch by merging origin/main into the current branch
   - Push the updated branch to origin
5. Run: `pleb transition <issue-number> done`
6. Report the PR URL to the user

## Context
- Working directory: Current worktree (contains issue number in path)
- Branch: Already created by pleb (pleb/issue-XXX)
- Issue number: Extract from current directory path

## Important
- If there are no changes to commit, skip step 1
- If PR already exists for this branch, report existing PR instead of creating new one
- Always check if the branch is out-of-date and update it before completing
- When updating the branch, prefer merging main into the current branch (not rebasing)
- Always transition to done state after PR is created/found and updated
- Do NOT close the issue manually - GitHub will auto-close it when the PR is merged
