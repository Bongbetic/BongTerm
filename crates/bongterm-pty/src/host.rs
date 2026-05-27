//! `ConPTY` host trait + Phase 0 placeholder implementation.
//!
//! Real `ConPTY` wiring lands in Phase 1 task 1.B.*; this scaffold proves the
//! surface compiles and locks the trait shape for downstream crates.

use anyhow::Result;
use std::path::PathBuf;

/// Specification for a child process to spawn inside a PTY.
#[derive(Debug, Clone)]
pub struct ChildSpec {
    /// Path to the executable.
    pub command: PathBuf,
    /// Arguments passed to the executable.
    pub args: Vec<String>,
    /// Working directory for the child process.
    pub cwd: Option<PathBuf>,
    /// Additional environment variables injected into the child process.
    pub env: Vec<(String, String)>,
    /// Initial PTY column count.
    pub cols: u16,
    /// Initial PTY row count.
    pub rows: u16,
}

/// Port interface for a `ConPTY`-backed process host.
pub trait PtyHost: Send + Sync {
    /// Spawn a child process in a PTY.
    fn spawn(&self, spec: ChildSpec) -> Result<PtyChild>;
}

/// Handle to a spawned PTY child process.
pub struct PtyChild {
    /// OS process identifier.
    pub pid: u32,
    // Real impl holds master read/write halves + child process handle.
}

/// Phase 0 scaffold: always returns an error.
///
/// Real `ConPTY` wiring arrives in Phase 1 task 1.B.*.
pub struct ScaffoldPtyHost;

impl PtyHost for ScaffoldPtyHost {
    fn spawn(&self, _spec: ChildSpec) -> Result<PtyChild> {
        Err(anyhow::anyhow!(
            "ConPTY spawn not yet implemented in Phase 0; arrives in Phase 1 task 1.B.*"
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scaffold_host_returns_error() {
        let h = ScaffoldPtyHost;
        let r = h.spawn(ChildSpec {
            command: PathBuf::from("pwsh.exe"),
            args: vec![],
            cwd: None,
            env: vec![],
            cols: 80,
            rows: 24,
        });
        assert!(r.is_err());
    }
}
