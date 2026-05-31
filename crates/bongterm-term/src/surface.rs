//! `BongTerm`-owned cell-grid snapshot types.
//!
//! Spec §1.2 + §3.1: `bongterm-render` consumes ONLY these types. It must not
//! import `wezterm-term` or `termwiz` directly.

use serde::{Deserialize, Serialize};

/// Cell attribute bit positions for [`CellRun::attrs`].
///
/// A bitfield rather than a struct so a run's formatting fits one `u32` and
/// crosses the `bongterm-render` boundary without pulling in `termwiz` types
/// (spec §1.2 ownership: render consumes only `bongterm-term` surface types).
pub mod attr {
    pub const BOLD: u32 = 1 << 0;
    pub const ITALIC: u32 = 1 << 1;
    pub const UNDERLINE: u32 = 1 << 2;
    pub const BLINK: u32 = 1 << 3;
    pub const REVERSE: u32 = 1 << 4;
    pub const STRIKETHROUGH: u32 = 1 << 5;
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CellPosition {
    pub row: u32,
    pub col: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CursorState {
    pub position: CellPosition,
    pub visible: bool,
    pub style: CursorStyle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CursorStyle {
    Block,
    Underline,
    Bar,
}

/// A run of cells with identical formatting on one row.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CellRun {
    pub row: u32,
    pub start_col: u32,
    pub text: String,
    pub fg: u32,
    pub bg: u32,
    /// Bitfield: bold, italic, underline, blink, reverse, strikethrough.
    pub attrs: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DirtyRegion {
    pub start: CellPosition,
    pub end_inclusive: CellPosition,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SurfaceSnapshot {
    pub cols: u32,
    pub rows: u32,
    pub runs: Vec<CellRun>,
    pub cursor: CursorState,
    /// Monotonic per-pane sequence number — renderer uses this to detect skips.
    pub seq: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_round_trips_through_json() {
        let snap = SurfaceSnapshot {
            cols: 80,
            rows: 24,
            runs: vec![CellRun {
                row: 0,
                start_col: 0,
                text: "hello".into(),
                fg: 0xff_ff_ff,
                bg: 0,
                attrs: 0,
            }],
            cursor: CursorState {
                position: CellPosition { row: 0, col: 5 },
                visible: true,
                style: CursorStyle::Block,
            },
            seq: 1,
        };
        let json = serde_json::to_string(&snap).expect("ser");
        let back: SurfaceSnapshot = serde_json::from_str(&json).expect("de");
        assert_eq!(snap, back);
    }

    #[test]
    fn dirty_region_can_be_point() {
        let r = DirtyRegion {
            start: CellPosition { row: 0, col: 0 },
            end_inclusive: CellPosition { row: 0, col: 0 },
        };
        assert_eq!(r.start, r.end_inclusive);
    }
}
