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

/// Warn-via-ledger threshold for atlas VRAM (64 MB per ADR-004).
pub const ATLAS_VRAM_WARN_BYTES: u64 = 64 * 1024 * 1024;

/// Returns `true` if estimated atlas VRAM meets or exceeds the ceiling.
/// Caller must drop and recreate the atlas when this returns true.
#[must_use]
pub fn atlas_vram_exceeded(current_bytes: u64) -> bool {
    current_bytes >= ATLAS_VRAM_CEILING_BYTES
}

/// Returns `true` if estimated atlas VRAM meets or exceeds the warn threshold.
/// Caller should emit a ledger alert; rendering continues.
#[must_use]
pub fn atlas_vram_warn_exceeded(current_bytes: u64) -> bool {
    current_bytes >= ATLAS_VRAM_WARN_BYTES
}

// ---------------------------------------------------------------------------
// Frame pacing (ADR-005 backpressure contract)
// ---------------------------------------------------------------------------

struct PendingFrame {
    snapshot_id: SnapshotId,
    regions: Vec<DirtyRegion>,
}

/// Single-slot backpressure buffer for dirty regions.
///
/// Parser submits dirty regions; render side takes them once per frame.
/// N submits between takes coalesce into one slot — no unbounded queue.
/// Caller is responsible for external synchronization across threads.
pub struct FramePacer {
    pending: Option<PendingFrame>,
}

impl FramePacer {
    /// Creates a new idle [`FramePacer`].
    #[must_use]
    pub fn new() -> Self {
        Self { pending: None }
    }

    /// Submits dirty regions for a snapshot.
    /// If a frame is already pending, regions are merged and `snapshot_id` advances.
    pub fn submit(&mut self, snapshot_id: SnapshotId, regions: &[DirtyRegion]) {
        match &mut self.pending {
            None => {
                self.pending = Some(PendingFrame {
                    snapshot_id,
                    regions: regions.to_vec(), // Phase 1.C.X: pool this allocation
                });
            }
            Some(pending) => {
                pending.snapshot_id = snapshot_id;
                pending.regions.extend_from_slice(regions);
            }
        }
    }

    /// Takes the pending frame for rendering. Returns `None` if idle.
    pub fn take(&mut self) -> Option<(SnapshotId, Vec<DirtyRegion>)> {
        self.pending.take().map(|f| (f.snapshot_id, f.regions))
    }

    /// Returns `true` if no frame is pending (render side is caught up).
    #[must_use]
    pub fn is_idle(&self) -> bool {
        self.pending.is_none()
    }
}

impl Default for FramePacer {
    fn default() -> Self {
        Self::new()
    }
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
// Takes `&DirtyRegion` so it composes directly with iterator adapters such as
// `regions.iter().map(dirty_region_quad_count)`; passing by value would break
// that call shape for a trivially-copyable 8-byte struct.
#[allow(clippy::trivially_copy_pass_by_ref)]
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
/// On DXGI `DEVICE_REMOVED`, Iced calls [`Storage::clear`] and invokes `new` again
/// with a fresh device — this IS the device-loss recovery path (1.C.4).
/// Phase 1.C.3 will wire `prepare`/`draw` to actually render glyphs.
pub struct TerminalPipeline {
    atlas: cryoglyph::TextAtlas,
    renderer: cryoglyph::TextRenderer,
    font_system: cryoglyph::FontSystem,
    swash_cache: cryoglyph::SwashCache,
    viewport: cryoglyph::Viewport,
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
        let viewport = cryoglyph::Viewport::new(device, &cache);
        Self {
            atlas,
            renderer,
            font_system: cryoglyph::FontSystem::new(),
            swash_cache: cryoglyph::SwashCache::new(),
            viewport,
        }
    }

    fn trim(&mut self) {
        self.atlas.trim();
    }
}

/// Lay the snapshot's row-major codepoints out as one `\n`-joined string.
///
/// Cell `0` and control codepoints render as a space; trailing spaces per row
/// are trimmed so the shaper does not pad runs. This is the scaffold's lossy
/// text view (no colour/attributes yet) — enough to put glyphs on screen.
#[must_use]
fn snapshot_to_text(snap: &SurfaceSnapshot) -> String {
    let cols = snap.cols as usize;
    let rows = snap.rows as usize;
    let mut out = String::with_capacity((cols + 1) * rows);
    for r in 0..rows {
        let mut line = String::with_capacity(cols);
        for c in 0..cols {
            let cp = snap.cells.get(r * cols + c).copied().unwrap_or(0);
            let ch = char::from_u32(cp).filter(|c| !c.is_control());
            line.push(ch.unwrap_or(' '));
        }
        out.push_str(line.trim_end());
        if r + 1 < rows {
            out.push('\n');
        }
    }
    out
}

impl iced::widget::shader::Primitive for TerminalPrimitive {
    type Pipeline = TerminalPipeline;

    // Casts to/from f32/i32 are intrinsic to pixel-space layout math here; the
    // values (window-bounded pixel coords) are far inside the lossy ranges.
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_precision_loss,
        clippy::cast_sign_loss,
        clippy::cast_possible_wrap
    )]
    fn prepare(
        &self,
        pipeline: &mut Self::Pipeline,
        device: &iced::wgpu::Device,
        queue: &iced::wgpu::Queue,
        bounds: &iced::Rectangle,
        viewport: &iced::widget::shader::Viewport,
    ) {
        let text = snapshot_to_text(&self.snapshot);

        // The shader render pass covers the whole physical surface; positions and
        // the cryoglyph viewport are in physical pixels, so scale logical bounds.
        let scale = viewport.scale_factor();
        let physical = viewport.physical_size();
        pipeline.viewport.update(
            queue,
            cryoglyph::Resolution {
                width: physical.width,
                height: physical.height,
            },
        );

        // Logical-pixel metrics; the TextArea scale lifts them to physical px.
        let font_size = 14.0_f32;
        let line_height = font_size * 1.25;
        let mut buffer = cryoglyph::Buffer::new(
            &mut pipeline.font_system,
            cryoglyph::Metrics::new(font_size, line_height),
        );
        buffer.set_wrap(&mut pipeline.font_system, cryoglyph::Wrap::None);
        buffer.set_size(
            &mut pipeline.font_system,
            Some(bounds.width.max(1.0)),
            Some(bounds.height.max(1.0)),
        );
        let attrs = cryoglyph::Attrs::new().family(cryoglyph::Family::Monospace);
        buffer.set_text(
            &mut pipeline.font_system,
            &text,
            &attrs,
            cryoglyph::Shaping::Advanced,
            None,
        );
        buffer.shape_until_scroll(&mut pipeline.font_system, false);

        let left = bounds.x * scale;
        let top = bounds.y * scale;
        let area = cryoglyph::TextArea {
            buffer: &buffer,
            left,
            top,
            scale,
            bounds: cryoglyph::TextBounds {
                left: left as i32,
                top: top as i32,
                right: ((bounds.x + bounds.width) * scale) as i32,
                bottom: ((bounds.y + bounds.height) * scale) as i32,
            },
            default_color: cryoglyph::Color::rgb(0xCC, 0xCC, 0xCC),
        };

        // cryoglyph's prepare records its staging-belt copies into an encoder; the
        // Iced shader API gives us no encoder, so create one and submit it here so
        // the glyph vertices are uploaded before draw().
        let mut encoder = device.create_command_encoder(&iced::wgpu::CommandEncoderDescriptor {
            label: Some("bongterm-text-prepare"),
        });
        if let Err(e) = pipeline.renderer.prepare(
            device,
            queue,
            &mut encoder,
            &mut pipeline.font_system,
            &mut pipeline.atlas,
            &pipeline.viewport,
            [area],
            &mut pipeline.swash_cache,
        ) {
            // Atlas-full or shaping error: skip this frame's text rather than
            // panic; the renderer recovers on the next frame.
            eprintln!("bongterm-render: text prepare failed: {e:?}");
        }
        queue.submit(Some(encoder.finish()));
    }

    fn draw(
        &self,
        pipeline: &Self::Pipeline,
        render_pass: &mut iced::wgpu::RenderPass<'_>,
    ) -> bool {
        if let Err(e) = pipeline
            .renderer
            .render(&pipeline.atlas, &pipeline.viewport, render_pass)
        {
            eprintln!("bongterm-render: text render failed: {e:?}");
        }
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
    vram_used_bytes: u64,
    device_lost: bool,
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

    /// Simulates a DXGI `DEVICE_REMOVED` event for testing device-loss recovery.
    ///
    /// # Panics
    ///
    /// Panics if the internal mutex is poisoned.
    pub fn force_device_loss(&self) {
        self.state.lock().unwrap().device_lost = true;
    }

    /// Sets mock VRAM usage returned by `collect_metrics`.
    ///
    /// # Panics
    ///
    /// Panics if the internal mutex is poisoned.
    pub fn set_mock_vram_used(&self, bytes: u64) {
        self.state.lock().unwrap().vram_used_bytes = bytes;
    }
}

impl Default for MockRendererBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl RendererBackend for MockRendererBackend {
    /// # Panics
    ///
    /// Panics if the internal mutex is poisoned.
    fn upload_glyphs(&self, _font_key: &FontKey, _glyphs: &[GlyphData]) -> Result<(), RenderError> {
        if self.state.lock().unwrap().device_lost {
            return Err(RenderError::DeviceLost);
        }
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
        if s.device_lost {
            return Err(RenderError::DeviceLost);
        }
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
            vram_used_bytes: s.vram_used_bytes,
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

    // Exact comparison is the point of this test: cell_quad_ndc is constructed so
    // the last cell's edge lands on exactly ±1.0 with no floating-point drift.
    #[allow(clippy::float_cmp)]
    #[test]
    fn last_cell_in_grid_reaches_ndc_corner() {
        let cell = cell_quad_ndc(79, 23, 80, 24);
        assert_eq!(cell.bottom_right, [1.0_f32, -1.0_f32]);
    }

    // Exact comparison is intended: adjacent cells must share a bit-identical
    // edge coordinate (n*2/cols) so quads tile with no seam.
    #[allow(clippy::float_cmp)]
    #[test]
    fn adjacent_cells_share_exact_horizontal_edge() {
        let left = cell_quad_ndc(0, 0, 80, 24);
        let right = cell_quad_ndc(1, 0, 80, 24);
        assert_eq!(left.bottom_right[0], right.top_left[0]);
    }

    #[test]
    fn quad_count_matches_sum_of_dirty_regions() {
        let regions = [
            DirtyRegion {
                col: 0,
                row: 0,
                width: 3,
                height: 4,
            },
            DirtyRegion {
                col: 10,
                row: 5,
                width: 2,
                height: 2,
            },
        ];
        let total: u32 = regions.iter().map(dirty_region_quad_count).sum();
        assert_eq!(total, 16);
    }

    // --- 1.C.5: VRAM ceiling enforcement ---

    #[test]
    fn atlas_vram_warn_threshold_is_64_mb() {
        assert_eq!(ATLAS_VRAM_WARN_BYTES, 64 * 1024 * 1024);
    }

    #[test]
    fn warn_threshold_not_exceeded_below_64_mb() {
        assert!(!atlas_vram_warn_exceeded(ATLAS_VRAM_WARN_BYTES - 1));
    }

    #[test]
    fn warn_threshold_exceeded_at_and_above_64_mb() {
        assert!(atlas_vram_warn_exceeded(ATLAS_VRAM_WARN_BYTES));
        assert!(atlas_vram_warn_exceeded(ATLAS_VRAM_WARN_BYTES + 1));
    }

    // Intentional constant assertion: documents and guards the ADR-004 invariant
    // that the warn threshold stays strictly below the drop-and-recreate ceiling.
    #[allow(clippy::assertions_on_constants)]
    #[test]
    fn warn_threshold_is_below_ceiling() {
        assert!(ATLAS_VRAM_WARN_BYTES < ATLAS_VRAM_CEILING_BYTES);
    }

    #[test]
    fn mock_reports_configured_vram_usage() {
        let mock = MockRendererBackend::new();
        mock.set_mock_vram_used(70 * 1024 * 1024);
        assert_eq!(mock.collect_metrics().vram_used_bytes, 70 * 1024 * 1024);
    }

    #[test]
    fn mock_vram_usage_triggers_warn_predicate() {
        let mock = MockRendererBackend::new();
        mock.set_mock_vram_used(ATLAS_VRAM_WARN_BYTES);
        assert!(atlas_vram_warn_exceeded(
            mock.collect_metrics().vram_used_bytes
        ));
    }

    #[test]
    fn mock_vram_usage_triggers_ceiling_predicate() {
        let mock = MockRendererBackend::new();
        mock.set_mock_vram_used(ATLAS_VRAM_CEILING_BYTES);
        assert!(atlas_vram_exceeded(mock.collect_metrics().vram_used_bytes));
    }

    // --- 1.C.4: device-loss recovery ---

    #[test]
    fn mock_returns_device_lost_after_forced_loss() {
        let mock = MockRendererBackend::new();
        mock.force_device_loss();
        let snapshot = SurfaceSnapshot {
            id: SnapshotId(1),
            cols: 80,
            rows: 24,
            cells: vec![],
        };
        let err = mock.render_frame(&snapshot, &[]).unwrap_err();
        assert!(matches!(err, RenderError::DeviceLost));
    }

    #[test]
    fn device_lost_does_not_increment_frame_count() {
        let mock = MockRendererBackend::new();
        let snapshot = SurfaceSnapshot {
            id: SnapshotId(1),
            cols: 80,
            rows: 24,
            cells: vec![],
        };
        mock.render_frame(&snapshot, &[]).unwrap();
        mock.force_device_loss();
        let _ = mock.render_frame(&snapshot, &[]);
        assert_eq!(mock.frames_rendered(), 1);
    }

    #[test]
    fn upload_glyphs_fails_after_device_loss() {
        let mock = MockRendererBackend::new();
        mock.force_device_loss();
        let font = FontKey {
            family: "Mono".into(),
            weight: 400,
            italic: false,
        };
        let err = mock.upload_glyphs(&font, &[]).unwrap_err();
        assert!(matches!(err, RenderError::DeviceLost));
    }

    #[test]
    fn frame_pacer_state_survives_device_loss() {
        let mut pacer = FramePacer::new();
        let region = DirtyRegion {
            col: 0,
            row: 0,
            width: 80,
            height: 24,
        };
        pacer.submit(SnapshotId(5), &[region]);

        let mock = MockRendererBackend::new();
        mock.force_device_loss();

        // recovery: new backend instance
        let fresh = MockRendererBackend::new();
        let snapshot = SurfaceSnapshot {
            id: SnapshotId(5),
            cols: 80,
            rows: 24,
            cells: vec![],
        };
        let (_, regions) = pacer.take().unwrap();
        fresh.render_frame(&snapshot, &regions).unwrap();
        assert_eq!(fresh.frames_rendered(), 1);
    }

    // --- 1.C.3: frame pacing ---

    #[test]
    fn new_pacer_is_idle() {
        let pacer = FramePacer::new();
        assert!(pacer.is_idle());
    }

    #[test]
    fn submit_marks_pacer_pending() {
        let mut pacer = FramePacer::new();
        pacer.submit(
            SnapshotId(1),
            &[DirtyRegion {
                col: 0,
                row: 0,
                width: 10,
                height: 5,
            }],
        );
        assert!(!pacer.is_idle());
    }

    #[test]
    fn take_returns_submitted_regions_and_clears_pending() {
        let mut pacer = FramePacer::new();
        let region = DirtyRegion {
            col: 0,
            row: 0,
            width: 10,
            height: 5,
        };
        pacer.submit(SnapshotId(3), &[region]);
        let (id, regions) = pacer.take().unwrap();
        assert_eq!(id, SnapshotId(3));
        assert_eq!(regions.len(), 1);
        assert_eq!(regions[0], region);
        assert!(pacer.is_idle());
    }

    #[test]
    fn take_from_idle_returns_none() {
        let mut pacer = FramePacer::new();
        assert!(pacer.take().is_none());
    }

    #[test]
    fn coalesce_preserves_all_regions_from_missed_frames() {
        let mut pacer = FramePacer::new();
        let r1 = DirtyRegion {
            col: 0,
            row: 0,
            width: 10,
            height: 1,
        };
        let r2 = DirtyRegion {
            col: 0,
            row: 5,
            width: 10,
            height: 2,
        };
        pacer.submit(SnapshotId(1), &[r1]);
        pacer.submit(SnapshotId(2), &[r2]);
        let (_, regions) = pacer.take().unwrap();
        assert_eq!(regions.len(), 2);
        assert!(regions.contains(&r1));
        assert!(regions.contains(&r2));
    }

    #[test]
    fn latest_snapshot_id_wins_after_coalescing() {
        let mut pacer = FramePacer::new();
        pacer.submit(
            SnapshotId(1),
            &[DirtyRegion {
                col: 0,
                row: 0,
                width: 1,
                height: 1,
            }],
        );
        pacer.submit(
            SnapshotId(99),
            &[DirtyRegion {
                col: 1,
                row: 0,
                width: 1,
                height: 1,
            }],
        );
        let (id, _) = pacer.take().unwrap();
        assert_eq!(id, SnapshotId(99));
    }

    #[test]
    fn n_submits_produce_single_pending_slot() {
        let mut pacer = FramePacer::new();
        for i in 0..5 {
            pacer.submit(
                SnapshotId(i),
                &[DirtyRegion {
                    col: 0,
                    row: 0,
                    width: 1,
                    height: 1,
                }],
            );
        }
        assert!(pacer.take().is_some());
        assert!(pacer.is_idle());
        assert!(pacer.take().is_none());
    }

    #[test]
    fn terminal_primitive_stores_snapshot_and_dirty_regions() {
        let snapshot = SurfaceSnapshot {
            id: SnapshotId(7),
            cols: 80,
            rows: 24,
            cells: vec![],
        };
        let dirty = vec![DirtyRegion {
            col: 0,
            row: 0,
            width: 80,
            height: 24,
        }];
        let prim = TerminalPrimitive::new(snapshot, dirty);
        assert_eq!(prim.snapshot().id, SnapshotId(7));
        assert_eq!(prim.dirty().len(), 1);
        assert_eq!(prim.dirty()[0].width, 80);
    }
}
