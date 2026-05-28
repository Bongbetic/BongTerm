//! bongterm-render
//!
//! See `docs/superpowers/specs/2026-05-27-bongt-mvp0-design.md` §1.2 for the
//! ownership matrix entry that governs what this crate may and may not own.
//!
//! SCAFFOLD ONLY — product renderer wgpu+cryoglyph implementation begins at Phase 1.C.1
//! per ADR-001/003/004/005/008.

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
// Atlas VRAM ceiling (ADR-004)
// ---------------------------------------------------------------------------

/// Drop-and-recreate threshold for the glyph atlas (128 MB per ADR-004).
pub const ATLAS_VRAM_CEILING_BYTES: u64 = 128 * 1024 * 1024;

/// Returns `true` if estimated atlas VRAM meets or exceeds the ceiling.
#[must_use]
pub fn atlas_vram_exceeded(current_bytes: u64) -> bool {
    current_bytes >= ATLAS_VRAM_CEILING_BYTES
}

// ---------------------------------------------------------------------------
// Cell NDC helpers
// ---------------------------------------------------------------------------

/// NDC quad bounds for a single terminal cell.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CellNdc {
    pub top_left: [f32; 2],
    pub bottom_right: [f32; 2],
}

/// Returns the NDC quad for cell `(col, row)` in a `cols × rows` grid.
///
/// Evaluates as `n * 2 / cols` (left-to-right) so the last cell's edge
/// lands on exactly ±1.0 without floating-point drift.
#[must_use]
pub fn cell_quad_ndc(col: u16, row: u16, cols: u16, rows: u16) -> CellNdc {
    CellNdc {
        top_left: [
            -1.0_f32 + f32::from(col) * 2.0_f32 / f32::from(cols),
            1.0_f32 - f32::from(row) * 2.0_f32 / f32::from(rows),
        ],
        bottom_right: [
            -1.0_f32 + (f32::from(col) + 1.0_f32) * 2.0_f32 / f32::from(cols),
            1.0_f32 - (f32::from(row) + 1.0_f32) * 2.0_f32 / f32::from(rows),
        ],
    }
}

/// Number of quads a dirty region produces (one per cell).
#[must_use]
pub fn dirty_region_quad_count(region: &DirtyRegion) -> u32 {
    u32::from(region.width) * u32::from(region.height)
}

// ---------------------------------------------------------------------------
// Iced Shader widget integration (ADR-005 Shape (a))
// ---------------------------------------------------------------------------

/// Per-frame data handed to the GPU pipeline.
#[derive(Debug, Clone)]
pub struct TerminalPrimitive {
    snapshot: SurfaceSnapshot,
    dirty: Vec<DirtyRegion>,
}

impl TerminalPrimitive {
    /// Creates a new [`TerminalPrimitive`].
    #[must_use]
    pub fn new(snapshot: SurfaceSnapshot, dirty: Vec<DirtyRegion>) -> Self {
        Self { snapshot, dirty }
    }

    /// The surface snapshot to render.
    #[must_use]
    pub fn snapshot(&self) -> &SurfaceSnapshot {
        &self.snapshot
    }

    /// Dirty regions that need repainting.
    #[must_use]
    pub fn dirty(&self) -> &[DirtyRegion] {
        &self.dirty
    }
}

/// GPU-side state: shared cryoglyph atlas + renderer (ADR-004 + ADR-005).
///
/// Created once by Iced on first [`TerminalPrimitive`] encounter (ADR-005 §1).
/// Phase 1.C.3 will wire `prepare`/`draw` to actually render glyphs.
pub struct TerminalPipeline {
    atlas: cryoglyph::TextAtlas,
    #[allow(dead_code)] // Phase 1.C.3 wires prepare()/draw()
    renderer: cryoglyph::TextRenderer,
}

impl iced::widget::shader::Pipeline for TerminalPipeline {
    fn new(
        device: &iced::wgpu::Device,
        queue: &iced::wgpu::Queue,
        format: iced::wgpu::TextureFormat,
    ) -> Self {
        let cache = cryoglyph::Cache::new(device);
        let mut atlas = cryoglyph::TextAtlas::new(device, queue, &cache, format);
        let renderer = cryoglyph::TextRenderer::new(
            &mut atlas,
            device,
            iced::wgpu::MultisampleState::default(),
            None, // no depth stencil for terminal rendering
        );
        Self { atlas, renderer }
    }

    fn trim(&mut self) {
        self.atlas.trim();
    }
}

impl iced::widget::shader::Primitive for TerminalPrimitive {
    type Pipeline = TerminalPipeline;

    fn prepare(
        &self,
        _pipeline: &mut Self::Pipeline,
        _device: &iced::wgpu::Device,
        _queue: &iced::wgpu::Queue,
        _bounds: &iced::Rectangle,
        _viewport: &iced::widget::shader::Viewport,
    ) {
        // Phase 1.C.3: upload dirty cells via pipeline.renderer.prepare()
    }

    fn draw(
        &self,
        _pipeline: &Self::Pipeline,
        _render_pass: &mut iced::wgpu::RenderPass<'_>,
    ) -> bool {
        // Phase 1.C.3: pipeline.renderer.render(render_pass, &pipeline.atlas, ...)
        true
    }
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

    // --- 1.C.2: glyph atlas ceiling ---

    #[test]
    fn atlas_vram_ceiling_is_128_mb() {
        assert_eq!(ATLAS_VRAM_CEILING_BYTES, 128 * 1024 * 1024);
    }

    #[test]
    fn atlas_vram_exceeded_at_and_above_ceiling() {
        assert!(!atlas_vram_exceeded(ATLAS_VRAM_CEILING_BYTES - 1));
        assert!(atlas_vram_exceeded(ATLAS_VRAM_CEILING_BYTES));
        assert!(atlas_vram_exceeded(ATLAS_VRAM_CEILING_BYTES + 1));
    }

    // --- 1.C.1: cell NDC + TerminalPrimitive ---

    #[test]
    fn last_cell_in_grid_reaches_ndc_corner() {
        let cell = cell_quad_ndc(79, 23, 80, 24);
        assert_eq!(cell.bottom_right, [1.0_f32, -1.0_f32]);
    }

    #[test]
    fn adjacent_cells_share_exact_horizontal_edge() {
        let left = cell_quad_ndc(0, 0, 80, 24);
        let right = cell_quad_ndc(1, 0, 80, 24);
        assert_eq!(left.bottom_right[0], right.top_left[0]);
    }

    #[test]
    fn quad_count_matches_sum_of_dirty_regions() {
        let regions = [
            DirtyRegion { col: 0, row: 0, width: 3, height: 4 },
            DirtyRegion { col: 10, row: 5, width: 2, height: 2 },
        ];
        let total: u32 = regions.iter().map(dirty_region_quad_count).sum();
        assert_eq!(total, 16);
    }

    #[test]
    fn terminal_primitive_stores_snapshot_and_dirty_regions() {
        let snapshot = SurfaceSnapshot {
            id: SnapshotId(7),
            cols: 80,
            rows: 24,
            cells: vec![],
        };
        let dirty = vec![DirtyRegion { col: 0, row: 0, width: 80, height: 24 }];
        let prim = TerminalPrimitive::new(snapshot, dirty);
        assert_eq!(prim.snapshot().id, SnapshotId(7));
        assert_eq!(prim.dirty().len(), 1);
        assert_eq!(prim.dirty()[0].width, 80);
    }
}
