//! Windows UIA COM shim placeholder.
//!
//! Real `IRawElementProviderSimple` wiring lives here so unsafe Windows interop
//! stays out of the safe model crate surface.

#[cfg(windows)]
pub struct WindowsUiaProvider;
