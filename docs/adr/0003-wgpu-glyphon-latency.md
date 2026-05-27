# ADR-003: wgpu + glyphon Keystroke-to-Glyph Latency

**Status:** Pending measured results — run `cargo run --release -p s1-renderer-latency`
**Date:** 2026-05-27
**Deciders:** Soubarna Karmakar

## Context

ADR-001 (Approach B) commits to a wgpu + glyphon renderer. Before Phase 1.C
implementation begins, Spike S1 measures actual keystroke-to-glyph latency on
reference hardware under a synthetic typing load of 200 keypresses.

## Spike S1 goal

Harness: `tools/spikes/s1-renderer-latency/`
Run: `cargo run --release -p s1-renderer-latency` (requires display; press keys to sample)

Measurement endpoints:
- **t1**: `Instant::now()` at top of `WindowEvent::KeyboardInput` handler (before any work)
- **t2**: `Instant::now()` after `wgpu::Queue::submit()` returns for the frame containing the new glyph
- This captures: event dispatch → glyph shaping → encoder recording → GPU submit (CPU side; GPU async).

## Decision threshold

- **p99 ≤ 16 ms after two optimization passes** → Approach B confirmed. Proceed with
  `bongterm-render` product implementation in Phase 1.C.
- **p99 > 16 ms after two optimization passes** → Approach C fallback per ADR-001
  §trigger conditions.

## Measured results (reference HW: Ryzen 5 7535HS / RTX 2050 4 GB / Win11 24H2)

| Metric | Value |
|--------|-------|
| Samples | 200 keypresses |
| p50 | *TBD — run harness and fill in* |
| p95 | *TBD* |
| p99 | *TBD* |
| Max | *TBD* |
| GPU adapter | *TBD* |

## Dependency finding (actionable before Phase 1.C)

**glyphon 0.6 requires wgpu ≥ 22.** The workspace currently pins `wgpu = "0.20"`.
The spike Cargo.tomls override to `wgpu = { version = "22" }` to compile.
The workspace `[workspace.dependencies]` wgpu entry must be bumped from `"0.20"` to
`"22"` before `bongterm-render` Phase 1.C implementation. Deferred to the Phase 1.C
planning boundary per ADR-005 Consequences.

## Decision

*Pending measured p99 value. Update Status to Accepted once harness results are
filled in above.*

## Consequences

If accepted:
- Phase 1.C tasks (1.C.1–1.C.5) may proceed.
- wgpu workspace pin bump required at 1.C boundary.
- ADR-004 (atlas eviction) must be accepted before 1.C.2 (shared atlas with LRU).
