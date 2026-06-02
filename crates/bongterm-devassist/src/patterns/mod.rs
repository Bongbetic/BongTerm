//! Clickable patterns submodule.

pub const MODULE_NAME: &str = "patterns";

pub mod matchers;
pub use matchers::{FileSpan, PatternKind, scan_file_locations};
