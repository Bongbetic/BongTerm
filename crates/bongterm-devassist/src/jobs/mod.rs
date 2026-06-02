//! Background jobs submodule.

pub const MODULE_NAME: &str = "jobs";

pub mod runner;
pub use runner::{JobId, JobSpec, JobState, Notifier, Toast, ToastKind};
