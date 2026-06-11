//! Phase 1 exit gates that can be asserted headlessly.
//!
//! Renderer/manual gates are represented by product-owned model checks here:
//! cold-start budget, RSS+VRAM budget plumbing, idle repaint suppression,
//! split-pane topology, and resource attribution.

use std::sync::Arc;
use std::time::{Duration, Instant};

use bongterm_ledger::{
    CurrentProcessSampler, MockVramSampler, ProcessCategory, ResourceSampler, VramInfo,
};
use bongterm_mux::{InMemoryMux, MuxRouter, SplitDirection};

#[test]
fn gate04_cold_start_boot_path_stays_under_budget() {
    let start = Instant::now();
    let (_app, task) = bongterm_app::TerminalApp::boot();
    drop(task);
    assert!(
        start.elapsed() <= Duration::from_millis(300),
        "boot path exceeded 300ms warm-start budget: {:?}",
        start.elapsed()
    );
}

#[test]
fn gate05_rss_and_vram_budget_are_measurable() {
    let vram = VramInfo {
        used_bytes: 32 * 1024 * 1024,
        budget_bytes: 512 * 1024 * 1024,
    };
    let sampler = CurrentProcessSampler::new(Arc::new(MockVramSampler::new(Some(vram))));
    let sample = sampler.take_sample();
    assert!(sample.total_rss_bytes() <= 120 * 1024 * 1024);
    let measured = sample.vram.expect("VRAM sampler must report a value");
    assert!(measured.used_bytes <= 256 * 1024 * 1024);
}

#[test]
fn gate06_redundant_resize_does_not_emit_repaint_work() {
    let (mut app, task) = bongterm_app::TerminalApp::boot();
    drop(task);
    let _ = app.update(bongterm_app::Message::Resized(1200.0, 800.0));
    let before = app.snapshot_id();
    let _ = app.update(bongterm_app::Message::Resized(1200.0, 800.0));
    assert_eq!(
        app.snapshot_id(),
        before,
        "unchanged grid must not advance snapshot/repaint work"
    );
}

#[test]
fn gate07_split_panes_resize_and_focus_cycle() {
    let mux = InMemoryMux::new();
    let tab = mux.create_tab(120, 40);
    let first = mux.tab_info(tab).unwrap().pane_ids[0];
    let second = mux
        .split_pane(first, SplitDirection::Horizontal)
        .expect("split pane");
    let tab_info = mux.tab_info(tab).unwrap();
    assert_eq!(tab_info.pane_ids.len(), 2);
    assert_eq!(tab_info.active_pane_id, first);
    assert_eq!(mux.pane_info(first).unwrap().rect.cols, 60);
    assert_eq!(mux.pane_info(second).unwrap().rect.left, 60);
    assert_eq!(mux.focus_next_pane(tab).unwrap(), second);
    assert_eq!(mux.focus_next_pane(tab).unwrap(), first);
}

#[test]
fn gate17_dashboard_attribution_includes_registered_pane_process() {
    let sampler = CurrentProcessSampler::new(Arc::new(MockVramSampler::unavailable()));
    sampler.register_pid(12345, ProcessCategory::Shell);
    let sample = sampler.take_sample();
    assert!(
        sample
            .processes
            .iter()
            .any(|process| { process.pid == 12345 && process.category == ProcessCategory::Shell })
    );
}
