//! Smart history submodule.

pub const MODULE_NAME: &str = "history";

pub mod filter;
pub use filter::{FilterKind, HistoryEntryMeta, HistoryQuery};
