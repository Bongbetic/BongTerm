//! Background jobs submodule.

pub const MODULE_NAME: &str = "jobs";

pub mod list;
pub use list::{JobList, JobRow};

pub mod runner;
pub use runner::{JobId, JobOutcome, JobRunner, JobSpec, JobState, Notifier, Toast, ToastKind};
