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

pub mod device_loss;

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

/// A run of identically-styled text positioned on the grid.
///
/// Renderer-local mirror of `bongterm-term`'s `CellRun` (spec §1.2: the renderer
/// must not import `bongterm-term`). `bongterm-app` maps one to the other.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CellSpan {
    pub row: u16,
    pub col: u16,
    pub text: String,
    /// Foreground colour, `0x00RRGGBB`.
    pub fg: u32,
    /// Background colour, `0x00RRGGBB`.
    pub bg: u32,
    /// Attribute bitfield: bold, italic, underline, blink, reverse, strikethrough.
    pub attrs: u32,
}

/// Cursor position + visibility for the renderer's cursor overlay.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CursorVis {
    pub row: u16,
    pub col: u16,
    pub visible: bool,
}

/// Attribute bit positions for [`CellSpan::attrs`] (mirror of `bongterm-term`).
pub mod attr {
    pub const BOLD: u32 = 1 << 0;
    pub const ITALIC: u32 = 1 << 1;
    pub const UNDERLINE: u32 = 1 << 2;
    pub const BLINK: u32 = 1 << 3;
    pub const REVERSE: u32 = 1 << 4;
    pub const STRIKETHROUGH: u32 = 1 << 5;
}

/// A surface snapshot passed to the renderer: styled text runs + cursor.
#[derive(Debug, Clone)]
pub struct SurfaceSnapshot {
    pub id: SnapshotId,
    pub cols: u16,
    pub rows: u16,
    /// Styled text runs, one per contiguous same-style cell range.
    pub spans: Vec<CellSpan>,
    /// Cursor position + visibility.
    pub cursor: CursorVis,
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

    /// Recreate device-bound renderer resources after device loss.
    fn recover_device_loss(&self) -> Result<(), RenderError> {
        Ok(())
    }
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

/// Build the rich-text span stream (`(text, Attrs)` pairs) for `set_rich_text`.
///
/// Spans are laid out row by row: within a row, runs are emitted in column
/// order with padding spaces inserting any column gap, and each run carries its
/// own foreground colour, weight (bold), and slant (italic). A `\n` separates
/// rows — `cosmic-text` splits the stream into lines on those newlines.
///
/// Strings are returned owned alongside their `Attrs` so the caller can hand
/// `set_rich_text` borrowed slices that outlive the call.
fn build_rich_spans(snap: &SurfaceSnapshot) -> Vec<(String, cryoglyph::Attrs<'static>)> {
    let rows = snap.rows as usize;
    let base = cryoglyph::Attrs::new().family(cryoglyph::Family::Monospace);

    // Bucket runs by row, then sort each row by starting column.
    let mut by_row: Vec<Vec<&CellSpan>> = vec![Vec::new(); rows];
    for span in &snap.spans {
        let r = span.row as usize;
        if r < rows {
            by_row[r].push(span);
        }
    }

    let mut out: Vec<(String, cryoglyph::Attrs<'static>)> = Vec::new();
    for (r, mut runs) in by_row.into_iter().enumerate() {
        runs.sort_by_key(|s| s.col);
        let mut next_col: u16 = 0;
        for run in runs {
            if run.col > next_col {
                let gap = usize::from(run.col - next_col);
                out.push((" ".repeat(gap), base.clone()));
            }
            let (rr, gg, bb) = unpack_rgb(run.fg);
            let weight = if run.attrs & attr::BOLD != 0 {
                cryoglyph::Weight::BOLD
            } else {
                cryoglyph::Weight::NORMAL
            };
            let style = if run.attrs & attr::ITALIC != 0 {
                cryoglyph::Style::Italic
            } else {
                cryoglyph::Style::Normal
            };
            let attrs = base
                .clone()
                .color(cryoglyph::Color::rgb(rr, gg, bb))
                .weight(weight)
                .style(style);
            // Next free column = this run's column + its width. Char count is a
            // close-enough proxy for monospace advance; wide glyphs refine later.
            let width = u16::try_from(run.text.chars().count()).unwrap_or(u16::MAX);
            next_col = run.col.saturating_add(width);
            out.push((run.text.clone(), attrs));
        }
        // Cursor overlay: a block glyph at the cursor cell, emitted into the same
        // layout stream so it aligns for free (no separate quad pass). Drawn only
        // when the cursor sits at or past the row's content end — the dominant
        // prompt-end case — so it never shifts existing glyphs. Mid-line cursor
        // positions are a v1 gap, pending the background quad pass.
        if snap.cursor.visible && usize::from(snap.cursor.row) == r && snap.cursor.col >= next_col {
            let gap = usize::from(snap.cursor.col - next_col);
            if gap > 0 {
                out.push((" ".repeat(gap), base.clone()));
            }
            let (cr, cg, cb) = unpack_rgb(CURSOR_RGB);
            out.push((
                "\u{2588}".to_string(),
                base.clone().color(cryoglyph::Color::rgb(cr, cg, cb)),
            ));
        }
        if r + 1 < rows {
            out.push(("\n".to_string(), base.clone()));
        }
    }
    out
}

/// Block-cursor colour (`0x00RRGGBB`), a light grey visible on a dark surface.
const CURSOR_RGB: u32 = 0x00D0_D0D0;

/// Split a `0x00RRGGBB` colour into its `(r, g, b)` byte components.
#[allow(clippy::cast_possible_truncation)]
const fn unpack_rgb(c: u32) -> (u8, u8, u8) {
    (
        ((c >> 16) & 0xFF) as u8,
        ((c >> 8) & 0xFF) as u8,
        (c & 0xFF) as u8,
    )
}

/// Measure the monospace cell size `(advance_width, line_height)` in **logical**
/// pixels for the renderer's font at `font_size`.
///
/// `bongterm-app` uses this to map a window's logical size to terminal
/// columns/rows (the renderer lays text out with this same font/metrics, so the
/// grid lines up). Creates a one-shot `cosmic-text` `FontSystem` (loads system
/// fonts — tens of ms), so callers should cache the result, not call per frame.
/// No GPU is required.
#[must_use]
pub fn monospace_cell_size(font_size: f32) -> (f32, f32) {
    const PROBE: &str = "MMMMMMMMMMMMMMMMMMMM"; // 20 monospace cells
    const PROBE_CELLS: f32 = 20.0;

    // Must match `TerminalPrimitive::prepare`: line height = font_size * 1.25.
    let line_height = font_size * 1.25;
    let mut font_system = cryoglyph::FontSystem::new();
    let mut buffer = cryoglyph::Buffer::new(
        &mut font_system,
        cryoglyph::Metrics::new(font_size, line_height),
    );
    buffer.set_size(&mut font_system, Some(10_000.0), Some(line_height * 2.0));
    let attrs = cryoglyph::Attrs::new().family(cryoglyph::Family::Monospace);
    buffer.set_text(
        &mut font_system,
        PROBE,
        &attrs,
        cryoglyph::Shaping::Advanced,
        None,
    );
    buffer.shape_until_scroll(&mut font_system, false);

    // Advance = laid-out width of the probe / cell count. Fall back to a typical
    // monospace ratio if the probe somehow produces no layout run.
    let cell_w = buffer
        .layout_runs()
        .next()
        .map_or(font_size * 0.6, |run| run.line_w / PROBE_CELLS);
    (cell_w, line_height)
}

/// Fast startup estimate for monospace cell size in **logical** pixels.
///
/// Real font probing can load system font metadata and is too expensive for the
/// app's boot path. The renderer still performs real shaping when preparing a
/// frame; this estimate is only for the initial grid until later calibration.
#[must_use]
pub const fn startup_monospace_cell_size(font_size: f32) -> (f32, f32) {
    (font_size * 0.6, font_size * 1.25)
}

/// Compute terminal grid dimensions `(cols, rows)` that fit a content area of
/// `width` × `height` **logical** pixels, given a cell size from
/// [`monospace_cell_size`]. Always returns at least `1×1`.
#[must_use]
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
pub fn grid_dims(width: f32, height: f32, cell_w: f32, cell_h: f32) -> (u16, u16) {
    let cols = (width / cell_w).floor().max(1.0);
    let rows = (height / cell_h).floor().max(1.0);
    (
        cols.min(f32::from(u16::MAX)) as u16,
        rows.min(f32::from(u16::MAX)) as u16,
    )
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
        let spans = build_rich_spans(&self.snapshot);

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
        let default_attrs = cryoglyph::Attrs::new().family(cryoglyph::Family::Monospace);
        buffer.set_rich_text(
            &mut pipeline.font_system,
            spans.iter().map(|(s, a)| (s.as_str(), a.clone())),
            &default_attrs,
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
            spans: vec![],
            cursor: CursorVis::default(),
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
            spans: vec![],
            cursor: CursorVis::default(),
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
            spans: vec![],
            cursor: CursorVis::default(),
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
            spans: vec![],
            cursor: CursorVis::default(),
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
            spans: vec![],
            cursor: CursorVis::default(),
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
            spans: vec![],
            cursor: CursorVis::default(),
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
            spans: vec![],
            cursor: CursorVis::default(),
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

    // --- 1.C.x: rich-span layout + cursor overlay ---

    fn spans_text(snap: &SurfaceSnapshot) -> String {
        build_rich_spans(snap)
            .iter()
            .map(|(s, _)| s.as_str())
            .collect()
    }

    fn snap_with_cursor(spans: Vec<CellSpan>, cursor: CursorVis) -> SurfaceSnapshot {
        SurfaceSnapshot {
            id: SnapshotId(1),
            cols: 80,
            rows: 24,
            spans,
            cursor,
        }
    }

    #[test]
    fn cursor_block_injected_at_cursor_cell() {
        let snap = snap_with_cursor(
            vec![],
            CursorVis {
                row: 0,
                col: 3,
                visible: true,
            },
        );
        // Empty row 0 → 3 pad spaces then the block cursor glyph.
        assert!(
            spans_text(&snap).starts_with("   \u{2588}"),
            "expected a block cursor at column 3"
        );
    }

    #[test]
    fn cursor_not_drawn_when_hidden() {
        let snap = snap_with_cursor(
            vec![],
            CursorVis {
                row: 0,
                col: 3,
                visible: false,
            },
        );
        assert!(
            !spans_text(&snap).contains('\u{2588}'),
            "a hidden cursor must not draw a block"
        );
    }

    #[test]
    fn cursor_block_follows_row_content() {
        // "hi" occupies cols 0-1; cursor at col 2 lands the block right after it.
        let snap = snap_with_cursor(
            vec![CellSpan {
                row: 0,
                col: 0,
                text: "hi".into(),
                fg: 0x00FF_FFFF,
                bg: 0,
                attrs: 0,
            }],
            CursorVis {
                row: 0,
                col: 2,
                visible: true,
            },
        );
        assert!(
            spans_text(&snap).starts_with("hi\u{2588}"),
            "cursor block should immediately follow 'hi'"
        );
    }

    #[test]
    fn span_foreground_colour_round_trips_to_unpack() {
        // Guards the fg packing/unpacking the renderer relies on for span colour.
        assert_eq!(unpack_rgb(0x00AB_CDEF), (0xAB, 0xCD, 0xEF));
    }

    // --- 1.C.x: cell metrics + grid dimensions (resize / panes) ---

    #[test]
    fn monospace_cell_size_is_plausible() {
        let (w, h) = monospace_cell_size(14.0);
        assert!(
            (h - 17.5).abs() < 0.001,
            "line height should be font_size*1.25"
        );
        assert!(
            w > 3.0 && w < 20.0,
            "advance for 14px monospace out of range: {w}"
        );
    }

    #[test]
    fn startup_monospace_cell_size_is_deterministic() {
        let (w, h) = startup_monospace_cell_size(14.0);
        assert!((w - 8.4).abs() < 0.001);
        assert!((h - 17.5).abs() < 0.001);
    }

    #[test]
    fn grid_dims_floors_and_clamps_to_one() {
        // 800x600 content with ~8x17.5 cells => 100 cols x 34 rows.
        assert_eq!(grid_dims(800.0, 600.0, 8.0, 17.5), (100, 34));
        // A sub-cell area still yields a usable 1x1 grid.
        assert_eq!(grid_dims(2.0, 2.0, 8.0, 17.5), (1, 1));
    }
}
