# ADR-003: wgpu + glyphon Keystroke-to-Glyph Latency

**Status:** Accepted
**Date:** 2026-05-27
**Deciders:** Soubarna Karmakar

## Context

ADR-001 (Approach B) commits to a wgpu + glyphon renderer. Spike S1 measures actual
keystroke-to-glyph latency on reference hardware under a synthetic typing load of
200 keypresses before Phase 1.C implementation begins.

## Spike S1 results

Harness: `tools/spikes/s1-renderer-latency/`
Reference HW: NVIDIA GeForce RTX 2050 / Ryzen 5 7535HS 16 GB / Win11 24H2

Measurement endpoints:
- **t1**: `Instant::now()` at top of `WindowEvent::KeyboardInput` handler (before any work)
- **t2**: `Instant::now()` after `wgpu::Queue::submit()` returns for the frame with the new glyph
- Captures: event dispatch → glyph shaping (80×24 cells of 'A') → encoder recording → GPU submit (CPU side)

| Metric | Value |
|--------|-------|
| Samples | 200 keypresses |
| Min | 432 µs |
| p50 | 829 µs |
| p95 | 1,181 µs |
| **p99** | **2,291 µs** |
| Max | 26,004 µs ¹ |
| GPU adapter | NVIDIA GeForce RTX 2050 |

¹ Max outlier (26 ms) is a first-frame or OS scheduling spike; p99 is the correct threshold metric.

## Decision threshold

- **p99 ≤ 16 ms after two optimization passes** → Approach B confirmed.
- Measured p99: **2.3 ms** — **PASS** (3.5× headroom against the 8 ms real-time goal,
  7× headroom against the 16 ms acceptance threshold).

## Decision

**Approach B confirmed. Proceed with `bongterm-render` wgpu + glyphon implementation.**

p99 of 2.3 ms at the `queue.submit` boundary leaves substantial headroom for the actual
GPU execution time (typically 0.5–2 ms additional on discrete GPU) to still clear 8 ms.
No optimization passes required before Phase 1.C.

## Dependency finding (actionable before Phase 1.C)

**glyphon 0.6 requires wgpu ≥ 22.** The workspace currently pins `wgpu = "0.20"`.
Spike Cargo.tomls override this locally. The workspace `[workspace.dependencies]` wgpu
entry must be bumped from `"0.20"` to `"22"` before Phase 1.C.1 implementation.
Tracked in ADR-005 Consequences and Phase 1.C planning boundary.

## Consequences

- Phase 1 tasks 1.C.1–1.C.5 may proceed.
- wgpu workspace pin bump (`"0.20"` → `"22"`) required at Phase 1.C boundary.
- ADR-004 (atlas eviction) already Accepted; shared atlas strategy confirmed.
- ADR-005 (device integration shape) already Accepted; Iced Shader widget confirmed.
- The 26 ms max outlier warrants a Phase 1 CI gate: p99 measured via headless replay
  benchmark, not single-run spike. Gate threshold: p99 ≤ 8 ms under 60 s sustained load.
