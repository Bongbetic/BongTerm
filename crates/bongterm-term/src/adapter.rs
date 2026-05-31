//! Thin adapter over vendored `wezterm-term` + `termwiz`.
//!
//! All `WezTerm` imports are isolated inside this file. An upstream API break is
//! absorbed here without touching the rest of `bongterm-term`.
//!
//! Phase 1 task 1.B.3: real `wezterm-term` wiring — `advance_bytes` delegates to
//! `wezterm_term::Terminal::advance_bytes` per ADR-007.

use crate::surface::{
    CellPosition, CellRun, CursorState, CursorStyle, DirtyRegion, SurfaceSnapshot,
};
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
        Self {
            cols,
            rows,
            seq: 0,
            dirty: vec![],
            terminal,
        }
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

        // Real per-run colour + attribute extraction. `Line::cluster` walks the
        // cells and groups successive cells with identical attributes into runs,
        // each carrying its starting column (`first_cell_idx`). Colours resolve
        // through the terminal's active palette so SGR 30-37/90-97, 256-colour,
        // and 24-bit truecolor all land correctly. A fresh terminal has no
        // scrollback, so phys rows 0..rows map to the visible screen.
        let palette = self.terminal.palette();
        let rows = self.rows as usize;
        let lines = self.terminal.screen().lines_in_phys_range(0..rows);
        let mut runs = Vec::new();
        for (row, line) in lines.iter().enumerate() {
            let row = u32::try_from(row).unwrap_or(u32::MAX);
            for cluster in line.cluster(None) {
                // Skip all-space runs on the default background — nothing visible
                // to draw. Background-coloured whitespace is revisited when the
                // renderer's background quad pass lands.
                if cluster.text.chars().all(|c| c == ' ') {
                    continue;
                }
                let fg = pack_rgb(palette.resolve_fg(cluster.attrs.foreground()).to_srgb_u8());
                let bg = pack_rgb(palette.resolve_bg(cluster.attrs.background()).to_srgb_u8());
                runs.push(CellRun {
                    row,
                    start_col: u32::try_from(cluster.first_cell_idx).unwrap_or(0),
                    text: cluster.text,
                    fg,
                    bg,
                    attrs: pack_attrs(&cluster.attrs),
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

/// Pack a resolved sRGB colour `(r, g, b, a)` into `0x00RRGGBB` (alpha dropped).
fn pack_rgb(rgb: (u8, u8, u8, u8)) -> u32 {
    (u32::from(rgb.0) << 16) | (u32::from(rgb.1) << 8) | u32::from(rgb.2)
}

/// Pack a cell's SGR attributes into the [`crate::surface::attr`] bitfield.
fn pack_attrs(a: &wezterm_term::CellAttributes) -> u32 {
    use crate::surface::attr;
    let mut bits = 0;
    if matches!(a.intensity(), wezterm_term::Intensity::Bold) {
        bits |= attr::BOLD;
    }
    if a.italic() {
        bits |= attr::ITALIC;
    }
    if !matches!(a.underline(), wezterm_term::Underline::None) {
        bits |= attr::UNDERLINE;
    }
    if a.reverse() {
        bits |= attr::REVERSE;
    }
    if a.strikethrough() {
        bits |= attr::STRIKETHROUGH;
    }
    bits
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
        assert_eq!(
            first_take.len(),
            2,
            "both ingest calls should produce dirty regions"
        );
        let second_take = a.take_dirty();
        assert!(
            second_take.is_empty(),
            "take_dirty should drain the accumulator"
        );
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

    #[test]
    fn snapshot_extracts_truecolor_foreground() {
        let mut a = WezTermAdapter::new(80, 24);
        // SGR 38;2;r;g;b sets a 24-bit truecolor foreground; resolve_fg returns it
        // verbatim, so the packed value is exactly 0xFF0000.
        a.ingest_bytes(b"\x1b[38;2;255;0;0mRED\x1b[0m");
        let snap = a.current_snapshot();
        let run = snap
            .runs
            .iter()
            .find(|r| r.text.contains("RED"))
            .expect("a run containing RED");
        assert_eq!(
            run.fg & 0x00FF_FFFF,
            0x00FF_0000,
            "truecolor foreground should pack to pure red, got {:#08x}",
            run.fg
        );
    }

    #[test]
    fn snapshot_extracts_bold_and_underline_attrs() {
        use crate::surface::attr;
        let mut a = WezTermAdapter::new(80, 24);
        a.ingest_bytes(b"\x1b[1;4mBU\x1b[0m");
        let snap = a.current_snapshot();
        let run = snap
            .runs
            .iter()
            .find(|r| r.text.contains("BU"))
            .expect("a run containing BU");
        assert_ne!(run.attrs & attr::BOLD, 0, "bold bit should be set");
        assert_ne!(
            run.attrs & attr::UNDERLINE,
            0,
            "underline bit should be set"
        );
    }

    #[test]
    fn plain_text_run_starts_at_its_column() {
        // "  hi" — the two leading spaces form their own (skipped) cluster, so the
        // "hi" run must report start_col = 2, preserving indentation.
        let mut a = WezTermAdapter::new(80, 24);
        a.ingest_bytes(b"  hi");
        let snap = a.current_snapshot();
        let run = snap
            .runs
            .iter()
            .find(|r| r.text.contains("hi"))
            .expect("a run containing hi");
        assert_eq!(run.start_col, 2, "leading spaces must offset start_col");
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
            "expected row 0 to start with 'hello', got {row0:?}",
        );
    }
}
