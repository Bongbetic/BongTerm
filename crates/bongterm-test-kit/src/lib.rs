//! `BongTerm` test utilities — mocks, conformance harnesses, error classification.
//!
//! This crate is workspace-internal and must never be published or imported by
//! production crates. It may depend on every other trait crate.

#![deny(unsafe_code)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::missing_errors_doc)]

use std::fmt;

// Re-export EnforcementLevel from the canonical definition in bongterm-security.
pub use bongterm_security::EnforcementLevel;

/// Broad classification of error kinds for test assertions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ErrorClass {
    /// Configuration or argument problem.
    Configuration,
    /// IO or network failure.
    Io,
    /// Authentication or authorization failure.
    Auth,
    /// Resource exhausted (memory, CPU, file handles).
    ResourceExhausted,
    /// Internal logic error / invariant violation.
    Internal,
    /// Timeout waiting for an operation.
    Timeout,
    /// Operation was cancelled.
    Cancelled,
    /// Unknown / unclassified.
    Unknown,
}

impl fmt::Display for ErrorClass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Configuration => "configuration",
            Self::Io => "io",
            Self::Auth => "auth",
            Self::ResourceExhausted => "resource-exhausted",
            Self::Internal => "internal",
            Self::Timeout => "timeout",
            Self::Cancelled => "cancelled",
            Self::Unknown => "unknown",
        };
        f.write_str(s)
    }
}

/// Risk of data loss associated with an operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum DataLossRisk {
    /// No data at risk.
    None,
    /// Data might be lost but is recoverable (e.g., in a trash or backup).
    Recoverable,
    /// Data will be permanently destroyed.
    Permanent,
}

impl fmt::Display for DataLossRisk {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::None => "none",
            Self::Recoverable => "recoverable",
            Self::Permanent => "permanent",
        };
        f.write_str(s)
    }
}

/// Application-level error type for `BongTerm` operations.
#[derive(Debug, thiserror::Error)]
pub enum BongtError {
    #[error("configuration error: {0}")]
    Configuration(String),
    #[error("IO error: {0}")]
    Io(String),
    #[error("authentication error: {0}")]
    Auth(String),
    #[error("resource exhausted: {0}")]
    ResourceExhausted(String),
    #[error("internal error: {0}")]
    Internal(String),
    #[error("timed out: {0}")]
    Timeout(String),
    #[error("cancelled: {0}")]
    Cancelled(String),
}

impl BongtError {
    /// Returns the `ErrorClass` for this error.
    #[must_use]
    pub fn class(&self) -> ErrorClass {
        match self {
            Self::Configuration(_) => ErrorClass::Configuration,
            Self::Io(_) => ErrorClass::Io,
            Self::Auth(_) => ErrorClass::Auth,
            Self::ResourceExhausted(_) => ErrorClass::ResourceExhausted,
            Self::Internal(_) => ErrorClass::Internal,
            Self::Timeout(_) => ErrorClass::Timeout,
            Self::Cancelled(_) => ErrorClass::Cancelled,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_class_display_does_not_panic() {
        let variants = [
            ErrorClass::Configuration,
            ErrorClass::Io,
            ErrorClass::Auth,
            ErrorClass::ResourceExhausted,
            ErrorClass::Internal,
            ErrorClass::Timeout,
            ErrorClass::Cancelled,
            ErrorClass::Unknown,
        ];
        for v in &variants {
            let s = format!("{v}");
            assert!(!s.is_empty(), "Display for {v:?} returned empty string");
        }
    }

    #[test]
    fn data_loss_risk_display_does_not_panic() {
        let variants = [DataLossRisk::None, DataLossRisk::Recoverable, DataLossRisk::Permanent];
        for v in &variants {
            let s = format!("{v}");
            assert!(!s.is_empty(), "Display for {v:?} returned empty string");
        }
    }

    #[test]
    fn enforcement_level_reexport_display_does_not_panic() {
        let variants = [
            EnforcementLevel::Advisory,
            EnforcementLevel::RequireApproval,
            EnforcementLevel::Deny,
        ];
        for v in &variants {
            let s = format!("{v}");
            assert!(!s.is_empty(), "Display for EnforcementLevel::{v:?} returned empty string");
        }
    }

    #[test]
    fn bongt_error_display_does_not_panic() {
        let errors = [
            BongtError::Configuration("bad config".to_string()),
            BongtError::Io("disk full".to_string()),
            BongtError::Auth("token expired".to_string()),
            BongtError::ResourceExhausted("OOM".to_string()),
            BongtError::Internal("invariant violated".to_string()),
            BongtError::Timeout("deadline exceeded".to_string()),
            BongtError::Cancelled("user pressed Ctrl+C".to_string()),
        ];
        for e in &errors {
            let s = format!("{e}");
            assert!(!s.is_empty(), "Display for {e:?} returned empty string");
        }
    }

    #[test]
    fn bongt_error_class_matches() {
        assert_eq!(BongtError::Io("err".to_string()).class(), ErrorClass::Io);
        assert_eq!(BongtError::Auth("err".to_string()).class(), ErrorClass::Auth);
        assert_eq!(BongtError::Internal("err".to_string()).class(), ErrorClass::Internal);
    }
}
