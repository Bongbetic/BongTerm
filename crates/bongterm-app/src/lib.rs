//! `bongterm-app` library: composition root for the terminal runtime.
//!
//! `bongterm-app` is the only crate allowed to wire the terminal session
//! (`bongterm-pty` spawn + `bongterm-term` parser) together; `bongterm-ui`
//! stays presentation-only. See the module ownership matrix in `CLAUDE.md`.

pub mod session;
pub mod terminal_app;
