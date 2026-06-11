//! `bongterm-app` library: composition root for the terminal runtime.
//!
//! `bongterm-app` is the only crate allowed to wire the terminal session
//! (`bongterm-pty` spawn + `bongterm-term` parser) together; `bongterm-ui`
//! stays presentation-only. See the module ownership matrix in `CLAUDE.md`.

pub mod session;
pub mod shell_app;
pub mod terminal_app;

pub use shell_app::{AppMessage, BongTermApp};
pub use terminal_app::{Message, TerminalApp};
