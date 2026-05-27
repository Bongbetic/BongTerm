# ADR-003: wgpu + glyphon Keystroke-to-Glyph Latency

**Status:** Pending — written at end of Spike S1  
**Date:** placeholder  
**Deciders:** Soubarna Karmakar

## Context

ADR-001 (Approach B) commits to a wgpu + glyphon renderer. Before implementing it, Spike S1 measures actual keystroke-to-glyph p99 latency on reference hardware under a synthetic typing load.

## Spike S1 goal

Harness: `tools/spikes/s1-renderer-latency/`  
Output: `tools/spikes/s1-renderer-latency/results-<utc>.json` with at minimum:
- `p50_us`, `p95_us`, `p99_us`
- `gpu_adapter`, `driver_version`, `os_version`

## Decision threshold

- **p99 ≤ 16 ms after two optimization passes** → Approach B confirmed. Proceed with `bongterm-render` product implementation.
- **p99 > 16 ms after two optimization passes** → Approach C trigger per ADR-001 §trigger conditions.

## Decision

*Pending S1 results. Update this ADR with measured values and decision.*

## Consequences

*TBD — determined by S1 results.*
