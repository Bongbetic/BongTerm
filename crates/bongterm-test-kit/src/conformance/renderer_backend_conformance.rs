//! Conformance suite for [`bongterm_render::RendererBackend`].

use bongterm_render::{DirtyRegion, RendererBackend, SnapshotId, SurfaceSnapshot};

/// Run happy-path conformance checks against any [`RendererBackend`] implementation.
///
/// # Panics
///
/// Panics if any conformance assertion fails, indicating the implementation
/// does not satisfy the port contract.
pub fn run(backend: &impl RendererBackend) {
    let snapshot = SurfaceSnapshot {
        id: SnapshotId(1),
        cols: 80,
        rows: 24,
        cells: vec![0u32; 80 * 24],
    };
    let dirty = [DirtyRegion {
        col: 0,
        row: 0,
        width: 80,
        height: 24,
    }];

    assert!(
        backend.render_frame(&snapshot, &dirty).is_ok(),
        "render_frame must return Ok for a valid snapshot"
    );

    let metrics = backend.collect_metrics();
    assert!(
        metrics.frames_rendered >= 1,
        "collect_metrics must report frames_rendered >= 1 after one render_frame call"
    );
}
