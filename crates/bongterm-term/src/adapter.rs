//! Thin adapter over vendored `wezterm-term` + `termwiz`.
//!
//! All `WezTerm` imports are isolated inside this file. An upstream API break is
//! absorbed here without touching the rest of `bongterm-term`.
//!
//! Phase 1 task 1.B.3: real `wezterm-term` wiring — `advance_bytes` delegates to
//! `wezterm_term::Terminal::advance_bytes` per ADR-007.

use crate::surface::{CellPosition, CellRun, CursorState, CursorStyle, DirtyRegion, SurfaceSnapshot};
use std::sync::Arc;

/// Minimal `TerminalConfiguration` implementation: uses wezterm-term defaults for
/// everything; only `color_palette` is required by the trait.
#[derive(Debug)]
struct BongtermConfig;

impl wezterm_term::TerminalConfiguration for BongtermConfig {
    fn color_palette(&self) -> wezterm_term::color::ColorPalette {
        wezterm_term::color::ColorPalette::default()
    }
}

pub struct WezTermAdapter {
    cols: u32,
    rows: u32,
    seq: u64,
    dirty: Vec<DirtyRegion>,
    /// Backing terminal emulator. `pub(crate)` so unit tests can inspect screen state.
    pub(crate) terminal: wezterm_term::Terminal,
}

impl WezTermAdapter {
    #[must_use]
    pub fn new(cols: u32, rows: u32) -> Self {
        let size = wezterm_term::TerminalSize {
            rows: rows as usize,
            cols: cols as usize,
            pixel_width: 0,
            pixel_height: 0,
            dpi: 0,
        };
        let terminal = wezterm_term::Terminal::new(
            size,
            Arc::new(BongtermConfig),
            "BongTerm",
            env!("CARGO_PKG_VERSION"),
            Box::new(std::io::sink()),
        );
        Self { cols, rows, seq: 0, dirty: vec![], terminal }
    }

    pub fn ingest_bytes(&mut self, bytes: &[u8]) {
        if bytes.is_empty() {
            return;
        }
        self.terminal.advance_bytes(bytes);
        // Mark full screen dirty. A future refinement can track precise dirty
        // regions via wezterm_term sequence-number deltas.
        self.dirty.push(DirtyRegion {
            start: CellPosition { row: 0, col: 0 },
            end_inclusive: CellPosition {
                row: self.rows.saturating_sub(1),
                col: self.cols.saturating_sub(1),
            },
        });
    }

    /// Drain and return all pending dirty regions since the last call.
    pub fn take_dirty(&mut self) -> Vec<DirtyRegion> {
        std::mem::take(&mut self.dirty)
    }

    pub fn current_snapshot(&mut self) -> SurfaceSnapshot {
        self.seq += 1;

        // v1: one run per non-blank visible row, default colors/attrs. Colour and
        // per-cell attribute extraction is deferred (the renderer ignores them
        // for now). A fresh terminal has no scrollback, so phys rows 0..rows map
        // to the visible screen.
        let rows = self.rows as usize;
        let lines = self.terminal.screen().lines_in_phys_range(0..rows);
        let mut runs = Vec::new();
        for (row, line) in lines.iter().enumerate() {
            let text = line.as_str();
            let trimmed = text.trim_end();
            if !trimmed.is_empty() {
                runs.push(CellRun {
                    row: u32::try_from(row).unwrap_or(u32::MAX),
                    start_col: 0,
                    text: trimmed.to_string(),
                    fg: 0x00FF_FFFF,
                    bg: 0,
                    attrs: 0,
                });
            }
        }

        let cursor = self.terminal.cursor_pos();
        SurfaceSnapshot {
            cols: self.cols,
            rows: self.rows,
            runs,
            cursor: CursorState {
                position: CellPosition {
                    row: u32::try_from(cursor.y).unwrap_or(0),
                    col: u32::try_from(cursor.x).unwrap_or(0),
                },
                visible: true,
                style: CursorStyle::Block,
            },
            seq: self.seq,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_seq_increments_monotonically() {
        let mut a = WezTermAdapter::new(80, 24);
        let s1 = a.current_snapshot();
        let s2 = a.current_snapshot();
        assert!(s2.seq > s1.seq);
    }

    #[test]
    fn fresh_adapter_has_empty_dirty_region() {
        let mut a = WezTermAdapter::new(80, 24);
        assert!(a.take_dirty().is_empty());
    }

    #[test]
    fn ingest_bytes_marks_full_screen_dirty() {
        let mut a = WezTermAdapter::new(80, 24);
        a.ingest_bytes(b"hello");
        let dirty = a.take_dirty();
        assert!(!dirty.is_empty(), "should have dirty regions after ingest");
        let r = &dirty[0];
        assert_eq!(r.start, CellPosition { row: 0, col: 0 });
        assert_eq!(r.end_inclusive, CellPosition { row: 23, col: 79 });
    }

    #[test]
    fn take_dirty_drains_accumulated_regions() {
        let mut a = WezTermAdapter::new(80, 24);
        a.ingest_bytes(b"first");
        a.ingest_bytes(b"second");
        let first_take = a.take_dirty();
        assert_eq!(first_take.len(), 2, "both ingest calls should produce dirty regions");
        let second_take = a.take_dirty();
        assert!(second_take.is_empty(), "take_dirty should drain the accumulator");
    }

    #[test]
    fn empty_bytes_produce_no_dirty_region() {
        let mut a = WezTermAdapter::new(80, 24);
        a.ingest_bytes(b"");
        assert!(a.take_dirty().is_empty());
    }

    #[test]
    fn current_snapshot_contains_ingested_text() {
        let mut a = WezTermAdapter::new(80, 24);
        a.ingest_bytes(b"hello world");
        let snap = a.current_snapshot();
        let joined: String = snap.runs.iter().map(|r| r.text.clone()).collect();
        assert!(
            joined.contains("hello world"),
            "snapshot runs must contain ingested text, got: {joined:?}"
        );
    }

    /// Feed plain ASCII bytes; verify the terminal model reflects the text on row 0.
    ///
    /// Uses `lines_in_phys_range` (non-test-gated) rather than `visible_lines`
    /// (which is `#[cfg(test)]` in wezterm-term's own test suite and therefore
    /// not available to external consumers).  A fresh terminal has no scrollback
    /// so phys-row 0 is the first visible row.
    #[test]
    fn advance_bytes_writes_text_to_wezterm_screen() {
        let mut a = WezTermAdapter::new(80, 24);
        a.ingest_bytes(b"hello");
        let lines = a.terminal.screen().lines_in_phys_range(0..1);
        let row0 = lines[0].as_str();
        assert!(
            row0.starts_with("hello"),
            "expected row 0 to start with 'hello', got {:?}",
            row0,
        );
    }
}
