//! Background-job model: spec, state, toast, and `Notifier` port (gate #13).
//!
//! Jobs run in a background pane off the hot path. On terminal state the runner
//! emits a desktop toast via the [`Notifier`] port. The real implementation
//! wraps WinRT notifications; tests use a recording mock.

/// Unique identifier for a background job.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct JobId(pub uuid::Uuid);

/// What to run in the background.
#[derive(Debug, Clone)]
pub struct JobSpec {
    pub id: JobId,
    pub label: String,
    pub command: String,
    pub args: Vec<String>,
    pub cwd: Option<String>,
}

/// Lifecycle state of a background job. Closed set means exhaustive matches.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JobState {
    /// Queued, not yet started.
    Pending,
    /// Currently executing.
    Running,
    /// Exited 0.
    Succeeded,
    /// Exited non-zero.
    Failed { exit_code: i64 },
    /// User-cancelled.
    Cancelled,
}

impl JobState {
    /// Whether this state is final.
    #[must_use]
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            Self::Succeeded | Self::Failed { .. } | Self::Cancelled
        )
    }
}

/// Severity of a toast.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToastKind {
    Success,
    Failure,
    Info,
}

/// A desktop toast payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Toast {
    pub kind: ToastKind,
    pub title: String,
    pub body: String,
}

impl Toast {
    /// Build the completion toast for a job that reached a terminal state.
    #[must_use]
    pub fn for_completion(spec: &JobSpec, state: &JobState) -> Self {
        match state {
            JobState::Succeeded => Self {
                kind: ToastKind::Success,
                title: "BongTerm".to_string(),
                body: format!("Background job \"{}\" completed.", spec.label),
            },
            JobState::Failed { exit_code } => Self {
                kind: ToastKind::Failure,
                title: "BongTerm".to_string(),
                body: format!(
                    "Background job \"{}\" failed (exit {exit_code}).",
                    spec.label
                ),
            },
            JobState::Cancelled => Self {
                kind: ToastKind::Info,
                title: "BongTerm".to_string(),
                body: format!("Background job \"{}\" was cancelled.", spec.label),
            },
            JobState::Pending | JobState::Running => Self {
                kind: ToastKind::Info,
                title: "BongTerm".to_string(),
                body: format!("Background job \"{}\" is running.", spec.label),
            },
        }
    }
}

/// Port for emitting desktop notifications. Real impl wraps WinRT; tests inject
/// `MockNotifier`, keeping `windows` out of pure job logic.
pub trait Notifier: Send + Sync {
    fn notify(&self, toast: &Toast);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn job_state_terminal_classification() {
        assert!(!JobState::Running.is_terminal());
        assert!(JobState::Succeeded.is_terminal());
        assert!(JobState::Failed { exit_code: 1 }.is_terminal());
        assert!(JobState::Cancelled.is_terminal());
    }

    #[test]
    fn toast_for_state_distinguishes_success_and_failure() {
        let spec = JobSpec {
            id: JobId(uuid::Uuid::nil()),
            label: "npm install".to_string(),
            command: "npm".to_string(),
            args: vec!["install".to_string()],
            cwd: None,
        };
        let ok = Toast::for_completion(&spec, &JobState::Succeeded);
        assert_eq!(ok.kind, ToastKind::Success);
        assert!(ok.body.contains("npm install"));

        let bad = Toast::for_completion(&spec, &JobState::Failed { exit_code: 1 });
        assert_eq!(bad.kind, ToastKind::Failure);
        assert!(bad.body.contains("failed"));
    }

}
