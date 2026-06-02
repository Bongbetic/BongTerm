//! Job-list view-model for the background-jobs panel (gate #13).
//!
//! Pure presentation state owned by devassist; UI reads snapshots. No process
//! spawn here.

use crate::jobs::runner::{JobId, JobSpec, JobState};

/// One row in the job panel.
#[derive(Debug, Clone)]
pub struct JobRow {
    pub id: JobId,
    pub label: String,
    pub state: JobState,
}

/// Ordered list of background jobs.
#[derive(Debug, Clone, Default)]
pub struct JobList {
    rows: Vec<JobRow>,
}

impl JobList {
    #[must_use]
    pub fn new() -> Self {
        Self { rows: Vec::new() }
    }

    /// Register a new job with an initial state.
    pub fn register(&mut self, spec: JobSpec, state: JobState) {
        self.rows.push(JobRow {
            id: spec.id,
            label: spec.label,
            state,
        });
    }

    /// Update the state of an existing job. No-op if the id is unknown.
    pub fn update(&mut self, id: JobId, state: JobState) {
        if let Some(row) = self.rows.iter_mut().find(|r| r.id == id) {
            row.state = state;
        }
    }

    /// Snapshot of all rows in registration order.
    #[must_use]
    pub fn snapshot(&self) -> Vec<JobRow> {
        self.rows.clone()
    }

    /// Count of non-terminal jobs.
    #[must_use]
    pub fn active_count(&self) -> usize {
        self.rows
            .iter()
            .filter(|row| !row.state.is_terminal())
            .count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::jobs::runner::{JobId, JobSpec, JobState};
    use uuid::Uuid;

    fn spec(label: &str) -> JobSpec {
        JobSpec {
            id: JobId(Uuid::new_v4()),
            label: label.to_string(),
            command: "x".to_string(),
            args: vec![],
            cwd: None,
        }
    }

    #[test]
    fn register_then_snapshot_shows_running_jobs() {
        let mut list = JobList::new();
        let s = spec("npm install");
        let id = s.id;
        list.register(s, JobState::Running);
        let snap = list.snapshot();
        assert_eq!(snap.len(), 1);
        assert_eq!(snap[0].label, "npm install");
        assert_eq!(snap[0].state, JobState::Running);
        assert_eq!(snap[0].id, id);
    }

    #[test]
    fn update_transitions_state_in_place() {
        let mut list = JobList::new();
        let s = spec("build");
        let id = s.id;
        list.register(s, JobState::Running);
        list.update(id, JobState::Succeeded);
        let snap = list.snapshot();
        assert_eq!(snap.len(), 1);
        assert_eq!(snap[0].state, JobState::Succeeded);
    }

    #[test]
    fn active_count_excludes_terminal_jobs() {
        let mut list = JobList::new();
        let a = spec("a");
        let b = spec("b");
        let (ida, idb) = (a.id, b.id);
        list.register(a, JobState::Running);
        list.register(b, JobState::Running);
        list.update(ida, JobState::Succeeded);
        assert_eq!(list.active_count(), 1);
        let _ = idb;
    }
}
