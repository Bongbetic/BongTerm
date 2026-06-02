//! bongterm-devassist
//!
//! Developer-UX features for BongTerm MVP-0: Cmd-K AI assist, failed-command
//! explainer, smart history, snippets, background jobs, clickable patterns.
//!
//! See `docs/superpowers/specs/2026-05-27-bongt-mvp0-design.md` Section 1.2 for
//! the ownership matrix entry. This crate MUST NOT touch the terminal hot path.

#![deny(unsafe_code)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::missing_errors_doc)]

pub mod ai;
pub mod history;
pub mod jobs;
pub mod patterns;
pub mod snippets;

/// Errors returned by devassist features.
#[derive(Debug, thiserror::Error)]
pub enum DevassistError {
    #[error("AI backend error: {0}")]
    Backend(String),
    #[error("AI assist unavailable: {0}")]
    Unavailable(String),
    #[error("parse error: {0}")]
    Parse(String),
    #[error("missing parameter: {0}")]
    MissingParam(String),
    #[error("storage error: {0}")]
    Storage(String),
    #[error("job error: {0}")]
    Job(String),
}

#[cfg(test)]
mod wiring_tests {
    use crate::DevassistError;

    #[test]
    fn error_display_is_nonempty() {
        let e = DevassistError::Backend("claude exited 1".to_string());
        assert!(!format!("{e}").is_empty());
    }

    #[test]
    fn submodules_are_declared() {
        // Compile-time proof the modules exist and are public.
        let _ = crate::ai::MODULE_NAME;
        let _ = crate::history::MODULE_NAME;
        let _ = crate::snippets::MODULE_NAME;
        let _ = crate::jobs::MODULE_NAME;
        let _ = crate::patterns::MODULE_NAME;
    }
}
