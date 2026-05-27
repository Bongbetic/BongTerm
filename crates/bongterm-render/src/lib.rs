//! bongterm-render
//!
//! See `docs/superpowers/specs/2026-05-27-bongt-mvp0-design.md` §1.2 for the
//! ownership matrix entry that governs what this crate may and may not own.
//!
//! SCAFFOLD ONLY — product renderer wgpu+glyphon implementation begins only after
//! ADR-002/003/004a/004b are accepted (Wave 0 spikes).

#![deny(unsafe_code)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::missing_errors_doc)]

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Stable identifier for a snapshot frame (monotonically increasing).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SnapshotId(pub u64);

/// Identifies a loaded font.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FontKey {
    pub family: String,
    pub weight: u16,
    pub italic: bool,
}

/// A single glyph to upload to the atlas.
#[derive(Debug, Clone)]
pub struct GlyphData {
    pub codepoint: char,
    pub width: u32,
    pub height: u32,
    pub bitmap: Vec<u8>, // grayscale alpha
}

/// A rectangular dirty region (col, row, width, height).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DirtyRegion {
    pub col: u16,
    pub row: u16,
    pub width: u16,
    pub height: u16,
}

/// A minimal surface snapshot passed to the renderer.
#[derive(Debug, Clone)]
pub struct SurfaceSnapshot {
    pub id: SnapshotId,
    pub cols: u16,
    pub rows: u16,
    /// Flat grid: row-major, cols*rows entries.
    pub cells: Vec<u32>, // simplified: just codepoints for scaffold
}

/// Metrics collected from the renderer.
#[derive(Debug, Clone, Default)]
pub struct RendererMetrics {
    pub frames_rendered: u64,
    pub vram_used_bytes: u64,
    pub glyphs_cached: u64,
}

// ---------------------------------------------------------------------------
// Error
// ---------------------------------------------------------------------------

/// Errors returned by renderer backend implementations.
#[derive(Debug, thiserror::Error)]
pub enum RenderError {
    #[error("device lost")]
    DeviceLost,
    #[error("atlas full")]
    AtlasFull,
    #[error("backend error: {0}")]
    Backend(String),
}

// ---------------------------------------------------------------------------
// Trait
// ---------------------------------------------------------------------------

/// Port interface for the terminal renderer.
/// Real implementation (wgpu + glyphon) arrives after ADR-002/003/004 accepted.
pub trait RendererBackend: Send + Sync {
    /// Upload glyphs for a font to the atlas.
    fn upload_glyphs(&self, font_key: &FontKey, glyphs: &[GlyphData]) -> Result<(), RenderError>;

    /// Render a frame for the given snapshot, updating only dirty regions.
    fn render_frame(
        &self,
        snapshot: &SurfaceSnapshot,
        dirty_regions: &[DirtyRegion],
    ) -> Result<(), RenderError>;

    /// Set the VRAM budget ceiling in bytes.
    fn set_vram_budget(&self, bytes: u64);

    /// Collect current renderer metrics.
    fn collect_metrics(&self) -> RendererMetrics;
}

// ---------------------------------------------------------------------------
// Mock
// ---------------------------------------------------------------------------

use std::sync::{Arc, Mutex};

#[derive(Debug, Default)]
struct MockState {
    last_snapshot_id: Option<SnapshotId>,
    frames_rendered: u64,
    vram_budget: u64,
}

/// In-memory mock renderer for tests; records calls without touching GPU.
pub struct MockRendererBackend {
    state: Arc<Mutex<MockState>>,
}

impl MockRendererBackend {
    /// Creates a new, empty [`MockRendererBackend`].
    #[must_use]
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(MockState::default())),
        }
    }

    /// Returns the snapshot ID of the last `render_frame` call.
    ///
    /// # Panics
    ///
    /// Panics if the internal mutex is poisoned.
    #[must_use]
    pub fn last_snapshot_id(&self) -> Option<SnapshotId> {
        self.state.lock().unwrap().last_snapshot_id
    }

    /// Returns how many frames have been rendered.
    ///
    /// # Panics
    ///
    /// Panics if the internal mutex is poisoned.
    #[must_use]
    pub fn frames_rendered(&self) -> u64 {
        self.state.lock().unwrap().frames_rendered
    }
}

impl Default for MockRendererBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl RendererBackend for MockRendererBackend {
    fn upload_glyphs(&self, _font_key: &FontKey, _glyphs: &[GlyphData]) -> Result<(), RenderError> {
        Ok(())
    }

    /// # Panics
    ///
    /// Panics if the internal mutex is poisoned.
    fn render_frame(
        &self,
        snapshot: &SurfaceSnapshot,
        _dirty_regions: &[DirtyRegion],
    ) -> Result<(), RenderError> {
        let mut s = self.state.lock().unwrap();
        s.last_snapshot_id = Some(snapshot.id);
        s.frames_rendered += 1;
        Ok(())
    }

    /// # Panics
    ///
    /// Panics if the internal mutex is poisoned.
    fn set_vram_budget(&self, bytes: u64) {
        self.state.lock().unwrap().vram_budget = bytes;
    }

    /// # Panics
    ///
    /// Panics if the internal mutex is poisoned.
    fn collect_metrics(&self) -> RendererMetrics {
        let s = self.state.lock().unwrap();
        RendererMetrics {
            frames_rendered: s.frames_rendered,
            vram_used_bytes: 0,
            glyphs_cached: 0,
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mock_records_snapshot_id() {
        let mock = MockRendererBackend::new();
        let snapshot = SurfaceSnapshot {
            id: SnapshotId(42),
            cols: 80,
            rows: 24,
            cells: vec![],
        };
        mock.render_frame(&snapshot, &[]).unwrap();
        assert_eq!(mock.last_snapshot_id(), Some(SnapshotId(42)));
    }

    #[test]
    fn mock_tracks_frame_count() {
        let mock = MockRendererBackend::new();
        let snapshot = SurfaceSnapshot {
            id: SnapshotId(1),
            cols: 80,
            rows: 24,
            cells: vec![],
        };
        mock.render_frame(&snapshot, &[]).unwrap();
        mock.render_frame(&snapshot, &[]).unwrap();
        assert_eq!(mock.frames_rendered(), 2);
    }

    #[test]
    fn mock_metrics_frame_count() {
        let mock = MockRendererBackend::new();
        let snapshot = SurfaceSnapshot {
            id: SnapshotId(0),
            cols: 80,
            rows: 24,
            cells: vec![],
        };
        mock.render_frame(&snapshot, &[]).unwrap();
        assert_eq!(mock.collect_metrics().frames_rendered, 1);
    }
}
