//! `ConPTY` host trait + Phase 0 placeholder + Phase 1 real implementation.
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
    /// Read half of the PTY master. `take_reader()` hands it to `PtyReaderTask`.
    reader: Option<Box<dyn std::io::Read + Send>>,
    /// Write half of the PTY master (child stdin).
    pub writer: Box<dyn std::io::Write + Send>,
    // Keep master alive: ConPTY `Arc<Mutex<Inner>>` holds the HPCON; dropping
    // master closes it and breaks the reader/writer pipes.
    _master: Box<dyn portable_pty::MasterPty + Send>,
    // Keep child handle alive for future wait()/kill() calls.
    _child: Box<dyn portable_pty::Child + Send + Sync>,
}

impl PtyChild {
    /// Take the read half of the PTY master. Returns `None` if already taken.
    pub fn take_reader(&mut self) -> Option<Box<dyn std::io::Read + Send>> {
        self.reader.take()
    }
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

/// Phase 1 real implementation backed by `portable-pty` / Windows `ConPTY`.
pub struct PortablePtyHost;

impl PtyHost for PortablePtyHost {
    fn spawn(&self, spec: ChildSpec) -> Result<PtyChild> {
        use portable_pty::{native_pty_system, CommandBuilder, PtySize};

        let pty_system = native_pty_system();
        let pair = pty_system.openpty(PtySize {
            rows: spec.rows,
            cols: spec.cols,
            pixel_width: 0,
            pixel_height: 0,
        })?;

        let mut cmd = CommandBuilder::new(&spec.command);
        for arg in &spec.args {
            cmd.arg(arg);
        }
        if let Some(ref cwd) = spec.cwd {
            cmd.cwd(cwd);
        }
        for (key, val) in &spec.env {
            cmd.env(key, val);
        }

        let child = pair.slave.spawn_command(cmd)?;
        let pid = child
            .process_id()
            .ok_or_else(|| anyhow::anyhow!("child has no pid"))?;
        let reader = pair.master.try_clone_reader()?;
        let writer = pair.master.take_writer()?;
        let master = pair.master;

        Ok(PtyChild {
            pid,
            reader: Some(reader),
            writer,
            _master: master,
            _child: child,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cmd_spec(args: Vec<&str>) -> ChildSpec {
        ChildSpec {
            command: PathBuf::from("cmd.exe"),
            args: args.into_iter().map(str::to_string).collect(),
            cwd: None,
            env: vec![],
            cols: 80,
            rows: 24,
        }
    }

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

    #[test]
    fn portable_pty_host_spawn_returns_ok() {
        let h = PortablePtyHost;
        let r = h.spawn(cmd_spec(vec!["/C", "exit", "0"]));
        assert!(r.is_ok(), "spawn failed: {:?}", r.err());
    }

    #[test]
    fn portable_pty_host_child_pid_is_nonzero() {
        let h = PortablePtyHost;
        let child = h
            .spawn(cmd_spec(vec!["/C", "exit", "0"]))
            .expect("spawn should succeed");
        assert!(child.pid > 0);
    }

    #[test]
    fn portable_pty_host_writer_accepts_input() {
        use std::io::Write;
        let h = PortablePtyHost;
        let mut child = h
            .spawn(cmd_spec(vec!["/K"]))
            .expect("spawn should succeed");
        assert!(child.writer.write_all(b"exit\r\n").is_ok());
    }
}
