//! Gate #8 — block-detection latency bench (spec §5.4 / §6.1).
//!
//! Spec §5.4 names a hard perf budget:
//! `block_detection_latency_p99 <= 5 ms after command_end marker`.
//!
//! This test drives the **real** detection path through the public API only —
//! it lives in `tests/` precisely so it can see nothing but `pub` items, which
//! proves the "command_end -> detected" transition is observable rather than
//! asserting it. The window measured is exactly: parse the `OSC 133;D;<code>`
//! payload (the `command_end` / FTCS "D" marker), feed it to the detector, and
//! query the confidence label — i.e. everything that happens *after* the marker
//! arrives until the block is finalized with its label.
//!
//! Anti-gaming guards (see `docs/phase1-exit-gates.md`):
//!   * Every iteration asserts the push actually closed a block
//!     (`block.is_some()`), so a no-op loop cannot pass green-by-construction.
//!   * The marker bytes and the resulting block are `black_box`'d so the
//!     optimizer cannot fold the work away (matters under `--release`).
//!   * The realistic prompt prefix (OSC 7 cwd + FTCS A/B/C) is pushed *outside*
//!     the timer, so we time marker consumption, not session setup.

use std::hint::black_box;
use std::time::Instant;

use bongterm_blocks::{BlockBuilder, parse_osc};

/// Number of measured iterations. 10_000 keeps total runtime well under a
/// second (each iteration is sub-microsecond) while making p99 robust: it
/// discards the top 100 samples, so a stray OS scheduler stall lands above p99
/// and cannot flake the assert.
const ITERATIONS: usize = 10_000;

/// Spec §5.4 budget, in nanoseconds (5 ms).
const BUDGET_NS: u128 = 5_000_000;

#[test]
fn gate08_block_detection_latency_p99_under_5ms() {
    let mut samples: Vec<u128> = Vec::with_capacity(ITERATIONS);

    for _ in 0..ITERATIONS {
        // ── Realistic command-block prefix — NOT timed ──────────────────────
        // A High-confidence shell-integration block: OSC 7 (cwd) then the three
        // leading FTCS markers. Mirrors `tests/fixtures/osc/bash_session.txt`.
        let mut builder = BlockBuilder::new();
        let _ = builder.push(parse_osc(b"7;file:///c:/projects/bongt"));
        let _ = builder.push(parse_osc(b"133;A"));
        let _ = builder.push(parse_osc(b"133;B"));
        let _ = builder.push(parse_osc(b"133;C"));

        // ── Timed window: command_end marker -> block finalized + labelled ──
        let start = Instant::now();
        let event = parse_osc(black_box(b"133;D;0"));
        let block = builder.push(event);
        // The confidence label is part of "blocks render with confidence labels"
        // (gate #8), so the label query belongs inside the measured window.
        let confidence = builder.confidence();
        let elapsed = start.elapsed();
        black_box(&block);
        black_box(confidence);

        // Anti-gaming: prove the timed code actually closed a block every loop.
        assert!(
            block.is_some(),
            "command_end marker did not finalize a block — \
             detection path is a no-op, measurement is meaningless"
        );

        samples.push(elapsed.as_nanos());
    }

    samples.sort_unstable();
    // Integer percentile index (avoids float-cast clippy lints under -D warnings).
    let p99_ns = samples[ITERATIONS * 99 / 100];
    let p50_ns = samples[ITERATIONS / 2];
    let max_ns = *samples.last().unwrap();

    eprintln!(
        "gate08 block-detection latency over {ITERATIONS} iterations: \
         p50 = {} ns ({:.3} µs), p99 = {} ns ({:.3} µs), max = {} ns ({:.3} µs); \
         budget p99 <= 5 ms (= {BUDGET_NS} ns)",
        p50_ns,
        p50_ns as f64 / 1000.0,
        p99_ns,
        p99_ns as f64 / 1000.0,
        max_ns,
        max_ns as f64 / 1000.0,
    );

    assert!(
        p99_ns <= BUDGET_NS,
        "block_detection_latency_p99 = {} µs exceeds spec §5.4 budget of 5 ms",
        p99_ns / 1000
    );
}
