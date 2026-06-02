//! Clickable patterns submodule.

pub const MODULE_NAME: &str = "patterns";

pub mod matchers;
pub mod url;
pub use matchers::{
    ClickableOverlay, FileSpan, LineRef, OverlaySpan, PatternKind, scan_file_locations,
};
pub use url::{LinkKind, Osc8Link, UrlSpan, parse_osc8, scan_urls, verify_destination};
