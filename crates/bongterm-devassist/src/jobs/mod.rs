//! Background jobs submodule.

pub const MODULE_NAME: &str = "jobs";

pub mod list;
pub use list::{JobList, JobListSnapshot, JobRow, JobRowView};

pub mod runner;
pub use runner::{JobId, JobOutcome, JobRunner, JobSpec, JobState, Notifier, Toast, ToastKind};
