//! bongterm-process-control
//!
//! See `docs/superpowers/specs/2026-05-27-bongt-mvp0-design.md` §1.2 for the
//! ownership matrix entry that governs what this crate may and may not own.

#![cfg_attr(not(windows), deny(unsafe_code))]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::missing_errors_doc)]

#[cfg(windows)]
pub mod job;

/// Resource limits enforced via Windows Job Objects.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct JobObjectCaps {
    /// RSS memory ceiling in bytes (0 = no limit).
    pub rss_bytes: u64,
    /// CPU rate limit in basis points (0–10000; 0 = no limit; 10000 = 100%).
    pub cpu_rate_bps: u32,
    /// Maximum number of active child processes (0 = no limit).
    pub child_proc_count: u32,
}

impl JobObjectCaps {
    /// No resource limits applied.
    pub const UNLIMITED: Self = Self {
        rss_bytes: 0,
        cpu_rate_bps: 0,
        child_proc_count: 0,
    };
}

/// Process handle type for the governor abstraction.
/// On Windows this maps to a Win32 process ID.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ProcessHandle(pub u32);

/// Reason a process was terminated by the governor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TerminationReason {
    /// Process exceeded its RSS memory limit.
    RssLimitExceeded { limit_bytes: u64, actual_bytes: u64 },
    /// Process exceeded its CPU rate limit.
    CpuLimitExceeded,
    /// Process exceeded its child process count limit.
    ChildProcLimitExceeded,
    /// User explicitly requested termination.
    UserRequested,
    /// An internal error caused termination.
    Error(String),
}

/// Admission verdict returned by the admission controller.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdmissionVerdict {
    /// Proceed — caps are within budget.
    Admit { caps: JobObjectCaps },
    /// Reject — describe why.
    Reject { reason: String },
}

/// Errors returned by [`ProcessGovernor`] operations.
#[derive(Debug, thiserror::Error)]
pub enum GovernorError {
    /// Process handle not currently tracked.
    #[error("process {0:?} not tracked")]
    NotTracked(ProcessHandle),
    /// A Job Object API call failed.
    #[error("job object error: {0}")]
    JobObject(String),
    /// Admission controller denied the request.
    #[error("admission denied: {0}")]
    AdmissionDenied(String),
}

/// Port interface for attaching Job Object caps to a process and monitoring compliance.
pub trait ProcessGovernor: Send + Sync {
    /// Attach Job Object caps to a process.
    fn attach(&self, handle: ProcessHandle, caps: JobObjectCaps) -> Result<(), GovernorError>;

    /// Update caps for an already-attached process.
    fn update_caps(&self, handle: ProcessHandle, caps: JobObjectCaps) -> Result<(), GovernorError>;

    /// Sample current RSS for a process.
    fn sample_rss(&self, handle: ProcessHandle) -> Result<u64, GovernorError>;

    /// Terminate a process via the governor.
    fn terminate(
        &self,
        handle: ProcessHandle,
        reason: TerminationReason,
    ) -> Result<(), GovernorError>;
}

/// Port interface for pre-flight admission control before launching a process.
pub trait AdmissionController: Send + Sync {
    /// Decide whether the requested caps are within the current system budget.
    fn admit(&self, requested: JobObjectCaps) -> AdmissionVerdict;
}

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// In-memory mock implementation of [`ProcessGovernor`] for use in tests.
pub struct MockProcessGovernor {
    attached: Arc<Mutex<HashMap<ProcessHandle, JobObjectCaps>>>,
}

impl MockProcessGovernor {
    /// Create a new empty mock governor.
    #[must_use]
    pub fn new() -> Self {
        Self {
            attached: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Returns the caps recorded for a process, or None.
    ///
    /// # Panics
    ///
    /// Panics if the internal mutex is poisoned.
    #[must_use]
    pub fn caps_for(&self, handle: ProcessHandle) -> Option<JobObjectCaps> {
        self.attached.lock().unwrap().get(&handle).copied()
    }
}

impl Default for MockProcessGovernor {
    fn default() -> Self {
        Self::new()
    }
}

impl ProcessGovernor for MockProcessGovernor {
    fn attach(&self, handle: ProcessHandle, caps: JobObjectCaps) -> Result<(), GovernorError> {
        self.attached.lock().unwrap().insert(handle, caps);
        Ok(())
    }

    fn update_caps(&self, handle: ProcessHandle, caps: JobObjectCaps) -> Result<(), GovernorError> {
        let mut map = self.attached.lock().unwrap();
        if let std::collections::hash_map::Entry::Occupied(mut e) = map.entry(handle) {
            e.insert(caps);
            Ok(())
        } else {
            Err(GovernorError::NotTracked(handle))
        }
    }

    fn sample_rss(&self, handle: ProcessHandle) -> Result<u64, GovernorError> {
        if self.attached.lock().unwrap().contains_key(&handle) {
            Ok(0)
        } else {
            Err(GovernorError::NotTracked(handle))
        }
    }

    fn terminate(
        &self,
        handle: ProcessHandle,
        _reason: TerminationReason,
    ) -> Result<(), GovernorError> {
        if self.attached.lock().unwrap().remove(&handle).is_some() {
            Ok(())
        } else {
            Err(GovernorError::NotTracked(handle))
        }
    }
}

/// Mock [`AdmissionController`] that always admits or always rejects.
pub struct MockAdmissionController {
    always_admit: bool,
}

impl MockAdmissionController {
    /// Returns a controller that admits every request.
    #[must_use]
    pub fn permissive() -> Self {
        Self { always_admit: true }
    }

    /// Returns a controller that rejects every request.
    #[must_use]
    pub fn restrictive() -> Self {
        Self {
            always_admit: false,
        }
    }
}

impl AdmissionController for MockAdmissionController {
    fn admit(&self, requested: JobObjectCaps) -> AdmissionVerdict {
        if self.always_admit {
            AdmissionVerdict::Admit { caps: requested }
        } else {
            AdmissionVerdict::Reject {
                reason: "mock: over budget".to_string(),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mock_attach_records_caps() {
        let gov = MockProcessGovernor::new();
        let handle = ProcessHandle(1234);
        let caps = JobObjectCaps {
            rss_bytes: 512 * 1024 * 1024,
            cpu_rate_bps: 5000,
            child_proc_count: 4,
        };
        gov.attach(handle, caps).unwrap();
        assert_eq!(gov.caps_for(handle), Some(caps));
    }

    #[test]
    fn mock_update_caps() {
        let gov = MockProcessGovernor::new();
        let handle = ProcessHandle(42);
        let initial = JobObjectCaps {
            rss_bytes: 100,
            cpu_rate_bps: 1000,
            child_proc_count: 2,
        };
        let updated = JobObjectCaps {
            rss_bytes: 200,
            cpu_rate_bps: 2000,
            child_proc_count: 4,
        };
        gov.attach(handle, initial).unwrap();
        gov.update_caps(handle, updated).unwrap();
        assert_eq!(gov.caps_for(handle), Some(updated));
    }

    #[test]
    fn update_untracked_errors() {
        let gov = MockProcessGovernor::new();
        let err = gov
            .update_caps(ProcessHandle(999), JobObjectCaps::UNLIMITED)
            .unwrap_err();
        assert!(matches!(err, GovernorError::NotTracked(_)));
    }

    #[test]
    fn permissive_admission_admits() {
        let ac = MockAdmissionController::permissive();
        let caps = JobObjectCaps {
            rss_bytes: 1024,
            cpu_rate_bps: 100,
            child_proc_count: 1,
        };
        assert!(matches!(ac.admit(caps), AdmissionVerdict::Admit { .. }));
    }

    #[test]
    fn restrictive_admission_rejects() {
        let ac = MockAdmissionController::restrictive();
        let caps = JobObjectCaps {
            rss_bytes: 1024,
            cpu_rate_bps: 100,
            child_proc_count: 1,
        };
        assert!(matches!(ac.admit(caps), AdmissionVerdict::Reject { .. }));
    }
}
