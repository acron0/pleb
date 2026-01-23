use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Instant;

/// Represents the lifecycle state of an issue being managed by pleb
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PlebState {
    Ready,
    Provisioning,
    Waiting,
    Working,
    Done,
    Finished,
}

impl PlebState {
    /// Returns the valid next states from the current state
    #[allow(dead_code)]
    pub fn valid_transitions(&self) -> Vec<PlebState> {
        match self {
            PlebState::Ready => vec![PlebState::Provisioning],
            PlebState::Provisioning => vec![PlebState::Waiting, PlebState::Working],
            PlebState::Waiting => vec![PlebState::Working, PlebState::Finished],
            PlebState::Working => vec![PlebState::Waiting, PlebState::Done, PlebState::Finished],
            PlebState::Done => vec![PlebState::Finished],
            PlebState::Finished => vec![], // Terminal state
        }
    }

    /// Returns true if this is a terminal state (no valid transitions)
    #[allow(dead_code)]
    pub fn is_terminal(&self) -> bool {
        self.valid_transitions().is_empty()
    }
}

/// Represents a single tracked issue with its current state and metadata
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct TrackedIssue {
    pub issue_number: u64,
    pub state: PlebState,
    pub worktree_path: Option<PathBuf>,
    pub started_at: Instant,
    pub last_updated: Instant,
}

/// Manages the state of all issues being tracked by pleb
#[derive(Debug)]
#[allow(dead_code)]
pub struct IssueTracker {
    tracked: HashMap<u64, TrackedIssue>,
}

impl IssueTracker {
    /// Create a new empty issue tracker
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self {
            tracked: HashMap::new(),
        }
    }

    /// Start tracking an issue with the given state
    #[allow(dead_code)]
    pub fn track(&mut self, issue_number: u64, state: PlebState) {
        let now = Instant::now();
        let tracked_issue = TrackedIssue {
            issue_number,
            state,
            worktree_path: None,
            started_at: now,
            last_updated: now,
        };
        self.tracked.insert(issue_number, tracked_issue);
    }

    /// Stop tracking an issue
    #[allow(dead_code)]
    pub fn untrack(&mut self, issue_number: u64) -> Option<TrackedIssue> {
        self.tracked.remove(&issue_number)
    }

    /// Get a tracked issue by number
    #[allow(dead_code)]
    pub fn get(&self, issue_number: u64) -> Option<&TrackedIssue> {
        self.tracked.get(&issue_number)
    }

    /// Get a mutable reference to a tracked issue by number
    #[allow(dead_code)]
    pub fn get_mut(&mut self, issue_number: u64) -> Option<&mut TrackedIssue> {
        self.tracked.get_mut(&issue_number)
    }

    /// Get all issues in a specific state
    #[allow(dead_code)]
    pub fn get_by_state(&self, state: PlebState) -> Vec<&TrackedIssue> {
        self.tracked
            .values()
            .filter(|issue| issue.state == state)
            .collect()
    }

    /// Update the state of a tracked issue
    #[allow(dead_code)]
    pub fn update_state(&mut self, issue_number: u64, new_state: PlebState) -> Result<()> {
        let issue = self.tracked.get_mut(&issue_number).with_context(|| {
            format!("Issue #{} is not being tracked", issue_number)
        })?;

        issue.state = new_state;
        issue.last_updated = Instant::now();
        Ok(())
    }

    /// Set the worktree path for a tracked issue
    #[allow(dead_code)]
    pub fn set_worktree_path(&mut self, issue_number: u64, path: PathBuf) -> Result<()> {
        let issue = self.tracked.get_mut(&issue_number).with_context(|| {
            format!("Issue #{} is not being tracked", issue_number)
        })?;

        issue.worktree_path = Some(path);
        issue.last_updated = Instant::now();
        Ok(())
    }

    /// Transition an issue to a new state with validation
    #[allow(dead_code)]
    pub fn transition(&mut self, issue_number: u64, to_state: PlebState) -> Result<()> {
        let issue = self.tracked.get(&issue_number).with_context(|| {
            format!("Issue #{} is not being tracked", issue_number)
        })?;

        let current_state = issue.state;
        let valid_transitions = current_state.valid_transitions();

        if !valid_transitions.contains(&to_state) {
            anyhow::bail!(
                "Cannot transition issue #{} from {:?} to {:?}. Valid transitions from {:?} are: {:?}",
                issue_number,
                current_state,
                to_state,
                current_state,
                valid_transitions
            );
        }

        // Transition is valid, update the state
        self.update_state(issue_number, to_state)
    }
}

impl Default for IssueTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_transitions() {
        assert_eq!(
            PlebState::Ready.valid_transitions(),
            vec![PlebState::Provisioning]
        );
        assert_eq!(
            PlebState::Provisioning.valid_transitions(),
            vec![PlebState::Waiting, PlebState::Working]
        );
        assert_eq!(
            PlebState::Waiting.valid_transitions(),
            vec![PlebState::Working, PlebState::Finished]
        );
        assert_eq!(
            PlebState::Working.valid_transitions(),
            vec![PlebState::Waiting, PlebState::Done, PlebState::Finished]
        );
        assert_eq!(
            PlebState::Done.valid_transitions(),
            vec![PlebState::Finished]
        );
        assert_eq!(PlebState::Finished.valid_transitions(), vec![]);
    }

    #[test]
    fn test_is_terminal() {
        assert!(!PlebState::Ready.is_terminal());
        assert!(!PlebState::Provisioning.is_terminal());
        assert!(!PlebState::Waiting.is_terminal());
        assert!(!PlebState::Working.is_terminal());
        assert!(!PlebState::Done.is_terminal());
        assert!(PlebState::Finished.is_terminal());
    }

    #[test]
    fn test_track_untrack() {
        let mut tracker = IssueTracker::new();

        tracker.track(123, PlebState::Ready);
        assert!(tracker.get(123).is_some());
        assert_eq!(tracker.get(123).unwrap().state, PlebState::Ready);

        let untracked = tracker.untrack(123);
        assert!(untracked.is_some());
        assert!(tracker.get(123).is_none());
    }

    #[test]
    fn test_update_state() {
        let mut tracker = IssueTracker::new();
        tracker.track(123, PlebState::Ready);

        tracker.update_state(123, PlebState::Working).unwrap();
        assert_eq!(tracker.get(123).unwrap().state, PlebState::Working);
    }

    #[test]
    fn test_get_by_state() {
        let mut tracker = IssueTracker::new();
        tracker.track(123, PlebState::Ready);
        tracker.track(456, PlebState::Working);
        tracker.track(789, PlebState::Ready);

        let ready_issues = tracker.get_by_state(PlebState::Ready);
        assert_eq!(ready_issues.len(), 2);

        let working_issues = tracker.get_by_state(PlebState::Working);
        assert_eq!(working_issues.len(), 1);
    }

    #[test]
    fn test_valid_transition() {
        let mut tracker = IssueTracker::new();
        tracker.track(123, PlebState::Ready);

        // Valid transition
        tracker.transition(123, PlebState::Provisioning).unwrap();
        assert_eq!(tracker.get(123).unwrap().state, PlebState::Provisioning);
    }

    #[test]
    fn test_invalid_transition() {
        let mut tracker = IssueTracker::new();
        tracker.track(123, PlebState::Ready);

        // Invalid transition (Ready -> Working requires going through Provisioning)
        let result = tracker.transition(123, PlebState::Working);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Cannot transition"));

        // State should not have changed
        assert_eq!(tracker.get(123).unwrap().state, PlebState::Ready);
    }

    #[test]
    fn test_terminal_state_transition() {
        let mut tracker = IssueTracker::new();
        tracker.track(123, PlebState::Finished);

        // Cannot transition from Finished (terminal state)
        let result = tracker.transition(123, PlebState::Working);
        assert!(result.is_err());
    }

    #[test]
    fn test_transition_to_finished() {
        let mut tracker = IssueTracker::new();

        // From Working to Finished
        tracker.track(123, PlebState::Working);
        tracker.transition(123, PlebState::Finished).unwrap();
        assert_eq!(tracker.get(123).unwrap().state, PlebState::Finished);

        // From Waiting to Finished
        tracker.track(456, PlebState::Waiting);
        tracker.transition(456, PlebState::Finished).unwrap();
        assert_eq!(tracker.get(456).unwrap().state, PlebState::Finished);

        // From Done to Finished
        tracker.track(789, PlebState::Done);
        tracker.transition(789, PlebState::Finished).unwrap();
        assert_eq!(tracker.get(789).unwrap().state, PlebState::Finished);
    }

    #[test]
    fn test_set_worktree_path() {
        let mut tracker = IssueTracker::new();
        tracker.track(123, PlebState::Ready);

        let path = PathBuf::from("/tmp/worktree/issue-123");
        tracker.set_worktree_path(123, path.clone()).unwrap();

        let issue = tracker.get(123).unwrap();
        assert_eq!(issue.worktree_path, Some(path));
    }

    #[test]
    fn test_update_state_nonexistent_issue() {
        let mut tracker = IssueTracker::new();
        let result = tracker.update_state(999, PlebState::Working);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not being tracked"));
    }

    #[test]
    fn test_set_worktree_path_nonexistent_issue() {
        let mut tracker = IssueTracker::new();
        let result = tracker.set_worktree_path(999, PathBuf::from("/tmp"));
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not being tracked"));
    }

    #[test]
    fn test_tracker_default() {
        let tracker = IssueTracker::default();
        assert!(tracker.get(1).is_none());
    }

    #[test]
    fn test_state_equality_and_copy() {
        let s1 = PlebState::Ready;
        let s2 = PlebState::Ready;
        let s3 = PlebState::Working;

        assert_eq!(s1, s2);
        assert_ne!(s1, s3);

        // Test Copy trait
        let s4 = s1;
        assert_eq!(s1, s4);
    }

    #[test]
    fn test_get_mut() {
        let mut tracker = IssueTracker::new();
        tracker.track(123, PlebState::Ready);

        if let Some(issue) = tracker.get_mut(123) {
            issue.state = PlebState::Working;
        }

        assert_eq!(tracker.get(123).unwrap().state, PlebState::Working);
    }
}
