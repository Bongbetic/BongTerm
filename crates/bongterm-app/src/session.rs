//! Terminal session: ConPTY child + VT parser, iced-free and synchronous.
//!
//! This is the testable core of the terminal: it spawns a shell, exposes the
//! master read half for a pump loop, feeds those bytes through the
//! `bongterm-term` VT parser, and renders a grid snapshot. The iced layer is a
//! thin shell over this unit; the headless integration test drives it directly.

use std::io::{Read, Write};
use std::path::PathBuf;

use anyhow::{Context, Result};
use bongterm_pty::{ChildSpec, PortablePtyHost, PtyChild, PtyHost};
use bongterm_term::{SurfaceSnapshot, WezTermAdapter};

/// Read half of the PTY master, handed to a reader thread / pump loop.
pub type PtyReader = Box<dyn Read + Send>;

/// A live terminal: owns the spawned child (writer + master/child handles kept
/// alive so the pipes stay open) and the VT parser/grid. Pump bytes from the
/// reader returned by [`TerminalSession::spawn_command`] into
/// [`TerminalSession::feed`]; render via [`TerminalSession::snapshot`].
pub struct TerminalSession {
    child: PtyChild,
    adapter: WezTermAdapter,
}

impl TerminalSession {
    /// Spawn `program` in a ConPTY of `cols`x`rows` and build the parser.
    /// Returns the session plus the master read half — pump it into [`Self::feed`].
    ///
    /// # Errors
    /// Returns an error if the child cannot be spawned or its reader is missing.
    pub fn spawn_command(
        program: &str,
        args: &[&str],
        cols: u16,
        rows: u16,
    ) -> Result<(Self, PtyReader)> {
        let spec = ChildSpec {
            command: PathBuf::from(program),
            args: args.iter().map(|s| (*s).to_string()).collect(),
            cwd: None,
            env: Vec::new(),
            cols,
            rows,
        };
        let mut child = PortablePtyHost
            .spawn(spec)
            .with_context(|| format!("spawn {program} in ConPTY"))?;
        let reader = child
            .take_reader()
            .context("PTY child reader already taken")?;
        let adapter = WezTermAdapter::new(u32::from(cols), u32::from(rows));
        Ok((Self { child, adapter }, reader))
    }

    /// Write bytes to the child's stdin (e.g. keystrokes).
    ///
    /// # Errors
    /// Returns an error if the write or flush to the PTY fails.
    pub fn write_input(&mut self, bytes: &[u8]) -> Result<()> {
        self.child.writer.write_all(bytes).context("write to PTY")?;
        self.child.writer.flush().context("flush PTY")?;
        Ok(())
    }

    /// Feed ConPTY output bytes through the VT parser into the grid.
    pub fn feed(&mut self, bytes: &[u8]) {
        self.adapter.ingest_bytes(bytes);
    }

    /// Current grid snapshot for rendering.
    pub fn snapshot(&mut self) -> SurfaceSnapshot {
        self.adapter.current_snapshot()
    }

    /// The snapshot's visible text: rows joined by newlines, each row's runs
    /// laid out at their column. Used by tests and simple status display.
    #[must_use]
    pub fn snapshot_text(&mut self) -> String {
        let snap = self.adapter.current_snapshot();
        let mut lines: Vec<String> = vec![String::new(); snap.rows.max(1) as usize];
        let mut runs: Vec<_> = snap.runs.iter().collect();
        runs.sort_by_key(|r| (r.row, r.start_col));
        for run in runs {
            let row = run.row as usize;
            if row >= lines.len() {
                continue;
            }
            let line = &mut lines[row];
            let start = run.start_col as usize;
            let cur = line.chars().count();
            if cur < start {
                line.push_str(&" ".repeat(start - cur));
            }
            line.push_str(&run.text);
        }
        lines.join("\n")
    }
}
