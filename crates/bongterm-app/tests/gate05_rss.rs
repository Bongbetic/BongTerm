//! Gate #5 (spec §6.1), **RSS portion**: "Core RSS ≤ 120 MB / 1 pane".
//!
//! Holds one *real* `TerminalSession` (ConPTY child + VT parser + grid) and
//! samples the current process's working set through the **real product
//! sampler** (`bongterm-ledger::CurrentProcessSampler` → `GetProcessMemoryInfo`).
//! This measures the actual terminal core doing its job, not an empty process.
//!
//! Scope (see `docs/phase1-exit-gates.md`):
//! - The **VRAM** portion of gate #5 (≤ 256 MB RTX / ≤ 128 MB iGPU) is BLOCKED:
//!   it needs the real wgpu renderer wired into the app **and** a GPU. Not
//!   asserted here.
//! - "Core RSS" = the BongTerm host process working set (children — cmd/conhost
//!   — are separate PIDs, not counted), matching the spec's "core" definition.

use std::sync::Arc;

use bongterm_app::session::TerminalSession;
use bongterm_ledger::{CurrentProcessSampler, MockVramSampler, ResourceSampler};

/// Spec §6.1 #5 core-RSS budget for a single pane.
const CORE_RSS_BUDGET_BYTES: u64 = 120 * 1024 * 1024;

#[test]
fn core_rss_within_budget_with_one_session() {
    // Spawn a real shell so a real ConPTY child + master handles are held for
    // the lifetime of the measurement (a realistic 1-pane resource footprint).
    let (mut session, reader) =
        TerminalSession::spawn_command("cmd.exe", &[], 80, 24).expect("spawn cmd.exe");

    // Exercise the real VT parser + grid by feeding synthetic output directly,
    // rather than reading the ConPTY master — ConPTY does not reliably signal
    // EOF while the master is open, so a blocking read here would hang. Feeding
    // bytes through `WezTermAdapter` is the same parse/grid path real output
    // takes, just without the unbounded read.
    for line in 0..200u32 {
        session.feed(
            format!("gate05 synthetic output line {line}: the quick brown fox\r\n").as_bytes(),
        );
    }
    let _ = session.snapshot_text();

    // Real sampler over the current process (VRAM mocked unavailable — not in
    // scope for this gate portion).
    let sampler = CurrentProcessSampler::new(Arc::new(MockVramSampler::unavailable()));
    let sample = sampler.take_sample();
    assert_eq!(
        sample.processes.len(),
        1,
        "sampler returns the host process"
    );
    let rss = sample.processes[0].rss_bytes;

    drop(session);
    drop(reader);

    let rss_mb = rss as f64 / 1024.0 / 1024.0;
    eprintln!(
        "gate05: core process RSS with 1 session = {rss} bytes ({rss_mb:.1} MB); budget 120 MB"
    );

    if cfg!(target_os = "windows") {
        assert!(rss > 0, "RSS must be measurable (>0) on Windows");
        assert!(
            rss <= CORE_RSS_BUDGET_BYTES,
            "core RSS {rss_mb:.1} MB exceeds the 120 MB budget (spec §6.1 #5)"
        );
    } else {
        eprintln!("gate05: non-Windows sampler is a stub (rss=0); RSS budget not asserted");
    }
}
