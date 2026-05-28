//! Thin adapter scaffold over vendored `wezterm-term` + `termwiz`.
//!
//! All `WezTerm` imports are isolated inside this file. An upstream API break is
//! absorbed here without touching the rest of `bongterm-term`.
//!
//! Phase 0 scaffold: real `wezterm-term` wiring deferred to Phase 1 task 1.B.3
//! after ADR-005 defines the submodule API contract.

use crate::surface::{CellPosition, CursorState, CursorStyle, DirtyRegion, SurfaceSnapshot};

pub struct WezTermAdapter {
    cols: u32,
    rows: u32,
    seq: u64,
    dirty: Vec<DirtyRegion>,
}

impl WezTermAdapter {
    #[must_use]
    pub fn new(cols: u32, rows: u32) -> Self {
        Self { cols, rows, seq: 0, dirty: vec![] }
    }

    pub fn ingest_bytes(&mut self, bytes: &[u8]) {
        if bytes.is_empty() {
            return;
        }
        // Scaffold: mark full screen dirty. Phase 1.B.3 will compute precise
        // regions once wezterm-term parser is wired.
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
        SurfaceSnapshot {
            cols: self.cols,
            rows: self.rows,
            runs: vec![],
            cursor: CursorState {
                position: CellPosition { row: 0, col: 0 },
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
}
