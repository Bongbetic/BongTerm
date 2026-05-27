//! Thin adapter scaffold over vendored `wezterm-term` + `termwiz`.
//!
//! All `WezTerm` imports are isolated inside this file. An upstream API break is
//! absorbed here without touching the rest of `bongterm-term`.
//!
//! Phase 0 scaffold: real `wezterm-term` wiring deferred to Phase 1 task 1.B.3
//! after ADR-005 defines the submodule API contract.

use crate::surface::{CellPosition, CursorState, CursorStyle, SurfaceSnapshot};

pub struct WezTermAdapter {
    cols: u32,
    rows: u32,
    seq: u64,
}

impl WezTermAdapter {
    #[must_use]
    pub fn new(cols: u32, rows: u32) -> Self {
        Self { cols, rows, seq: 0 }
    }

    #[allow(clippy::unused_self)]
    pub fn ingest_bytes(&mut self, _bytes: &[u8]) {
        // Phase 0 scaffold: parser hookup deferred to Phase 1.B.3.
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
}
