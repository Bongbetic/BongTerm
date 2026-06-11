//! Smart history submodule.

pub const MODULE_NAME: &str = "history";

pub mod filter;
pub mod frecency;
pub use filter::{FilterKind, HistoryEntryMeta, HistoryQuery};
pub use frecency::SmartHistory;
