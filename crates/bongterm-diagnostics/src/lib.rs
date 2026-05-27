//! Crash dumps, perf snapshots, redacted export bundles.
//!
//! Spec §4.2: app-wide panic is caught at tokio runtime root + UI thread
//! `catch_unwind`; a structured log is written to
//! `%LOCALAPPDATA%\BongTerm\crashes\<utc>.log`.
//! `minidump-writer` full wiring deferred to Phase 5 (MSIX packaging).

#![deny(unsafe_code)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]

use std::path::PathBuf;

/// Returns the directory where crash logs and minidumps are written.
#[must_use]
pub fn crash_dir() -> PathBuf {
    let local = std::env::var("LOCALAPPDATA").unwrap_or_else(|_| "C:\\Temp".to_string());
    PathBuf::from(local).join("BongTerm").join("crashes")
}

/// Install a process-wide panic hook that writes a structured log under `crash_dir()`.
///
/// Call once at `main()` startup before spawning any threads.
pub fn install_panic_hook() {
    std::panic::set_hook(Box::new(|info| {
        let _ = std::fs::create_dir_all(crash_dir());
        let utc = time::OffsetDateTime::now_utc()
            .format(&time::format_description::well_known::Iso8601::DEFAULT)
            .unwrap_or_else(|_| "unknown-time".into());
        let log_path = crash_dir().join(format!("{utc}.log"));
        let _ = std::fs::write(&log_path, format!("panic: {info}\n"));
        eprintln!("PANIC: {info}\nlog: {}", log_path.display());
    }));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crash_dir_contains_bongterm() {
        let d = crash_dir();
        assert!(d.to_string_lossy().to_ascii_lowercase().contains("bongterm"));
    }
}
