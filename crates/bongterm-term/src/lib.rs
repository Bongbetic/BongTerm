//! `BongTerm` terminal session port traits and DTOs.
//!
//! See `docs/superpowers/specs/2026-05-27-bongt-mvp0-design.md` §1.2 for the
//! ownership matrix entry that governs what this crate may and may not own.
//!
//! Real PTY implementation lives in `bongterm-pty`. Wired only by `bongterm-app`.

#![deny(unsafe_code)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::missing_errors_doc)]

use std::sync::{Arc, Mutex};

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Reason a terminal session was terminated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TerminateReason {
    UserRequested,
    ProcessExited { exit_code: i32 },
    Error(String),
}

/// Profile used to launch a terminal session.
#[derive(Debug, Clone)]
pub struct TerminalProfile {
    /// Shell executable, e.g. `"pwsh.exe"` or `"cmd.exe"`.
    pub shell: String,
    /// Working directory for the shell process.
    pub cwd: Option<String>,
    /// Additional environment variables to inject.
    pub env: Vec<(String, String)>,
    /// Initial PTY column count.
    pub cols: u16,
    /// Initial PTY row count.
    pub rows: u16,
}

impl Default for TerminalProfile {
    fn default() -> Self {
        Self {
            shell: "cmd.exe".to_string(),
            cwd: None,
            env: Vec::new(),
            cols: 80,
            rows: 24,
        }
    }
}

/// An event produced by a terminal session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TerminalEvent {
    /// Bytes output by the PTY (VT/ANSI sequences + text).
    Output(Vec<u8>),
    /// The session terminated.
    Terminated(TerminateReason),
}

// ---------------------------------------------------------------------------
// Error
// ---------------------------------------------------------------------------

/// Errors returned by [`TerminalSession`] implementations.
#[derive(Debug, thiserror::Error)]
pub enum SessionError {
    #[error("session not started")]
    NotStarted,
    #[error("session already started")]
    AlreadyStarted,
    #[error("PTY error: {0}")]
    Pty(String),
}

// ---------------------------------------------------------------------------
// Trait
// ---------------------------------------------------------------------------

/// Port interface for a terminal session backed by a PTY.
/// Real implementation in `bongterm-pty`. Wired by `bongterm-app`.
pub trait TerminalSession: Send + Sync {
    /// Start the session using the given profile.
    fn start(&self, profile: TerminalProfile) -> Result<(), SessionError>;

    /// Write raw bytes to the PTY input.
    fn write_input(&self, bytes: &[u8]) -> Result<(), SessionError>;

    /// Returns a bounded receiver for events from this session.
    /// The channel is created when the session is started.
    fn read_events(&self) -> tokio::sync::mpsc::Receiver<TerminalEvent>;

    /// Resize the PTY to the given dimensions.
    fn resize(&self, cols: u16, rows: u16) -> Result<(), SessionError>;

    /// Terminate the session.
    fn terminate(&self, reason: TerminateReason) -> Result<(), SessionError>;
}

// ---------------------------------------------------------------------------
// Mock
// ---------------------------------------------------------------------------

/// Test double for [`TerminalSession`]. Usable in unit tests without a real PTY.
pub struct MockTerminalSession {
    inputs_recorded: Arc<Mutex<Vec<Vec<u8>>>>,
    event_tx: tokio::sync::mpsc::Sender<TerminalEvent>,
    event_rx: Arc<Mutex<Option<tokio::sync::mpsc::Receiver<TerminalEvent>>>>,
    started: Arc<Mutex<bool>>,
}

impl MockTerminalSession {
    /// Create a new mock with a 64-slot event channel.
    #[must_use]
    pub fn new() -> Self {
        let (tx, rx) = tokio::sync::mpsc::channel(64);
        Self {
            inputs_recorded: Arc::new(Mutex::new(Vec::new())),
            event_tx: tx,
            event_rx: Arc::new(Mutex::new(Some(rx))),
            started: Arc::new(Mutex::new(false)),
        }
    }

    /// Inject an event that the next `read_events` consumer will receive.
    pub fn inject_event(&self, event: TerminalEvent) {
        let _ = self.event_tx.try_send(event);
    }

    /// Return a snapshot of all byte slices written via [`TerminalSession::write_input`].
    ///
    /// # Panics
    ///
    /// Panics if the internal mutex is poisoned (only possible if a previous
    /// thread panicked while holding the lock — not expected in normal use).
    #[must_use]
    pub fn recorded_inputs(&self) -> Vec<Vec<u8>> {
        self.inputs_recorded.lock().unwrap().clone()
    }
}

impl Default for MockTerminalSession {
    fn default() -> Self {
        Self::new()
    }
}

impl TerminalSession for MockTerminalSession {
    fn start(&self, _profile: TerminalProfile) -> Result<(), SessionError> {
        let mut s = self.started.lock().unwrap();
        if *s {
            return Err(SessionError::AlreadyStarted);
        }
        *s = true;
        Ok(())
    }

    fn write_input(&self, bytes: &[u8]) -> Result<(), SessionError> {
        self.inputs_recorded.lock().unwrap().push(bytes.to_vec());
        Ok(())
    }

    fn read_events(&self) -> tokio::sync::mpsc::Receiver<TerminalEvent> {
        self.event_rx
            .lock()
            .unwrap()
            .take()
            .expect("read_events called twice")
    }

    fn resize(&self, _cols: u16, _rows: u16) -> Result<(), SessionError> {
        Ok(())
    }

    fn terminate(&self, _reason: TerminateReason) -> Result<(), SessionError> {
        Ok(())
    }
}

pub mod surface;
pub use surface::{CellPosition, CellRun, CursorState, CursorStyle, DirtyRegion, SurfaceSnapshot};

pub mod adapter;
pub use adapter::WezTermAdapter;

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mock_records_input() {
        let mock = MockTerminalSession::new();
        mock.start(TerminalProfile::default()).unwrap();
        mock.write_input(b"a").unwrap();
        mock.write_input(b"b").unwrap();
        mock.write_input(b"c").unwrap();
        let recorded = mock.recorded_inputs();
        assert_eq!(recorded, vec![b"a".to_vec(), b"b".to_vec(), b"c".to_vec()]);
    }

    #[test]
    fn mock_inject_and_receive() {
        let mock = MockTerminalSession::new();
        mock.start(TerminalProfile::default()).unwrap();
        mock.inject_event(TerminalEvent::Output(b"hello".to_vec()));
        let mut rx = mock.read_events();
        let event = rx.try_recv().unwrap();
        assert_eq!(event, TerminalEvent::Output(b"hello".to_vec()));
    }

    #[test]
    fn mock_records_input_produces_three_events() {
        // Plan spec: writing b"abc" produces three input-recorded events.
        let mock = MockTerminalSession::new();
        mock.start(TerminalProfile::default()).unwrap();
        mock.write_input(b"a").unwrap();
        mock.write_input(b"b").unwrap();
        mock.write_input(b"c").unwrap();
        assert_eq!(mock.recorded_inputs().len(), 3);
    }
}
