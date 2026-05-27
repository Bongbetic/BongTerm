# ADR-004: Glyph Atlas VRAM Eviction Strategy

**Status:** Pending — written at end of Spike S2  
**Date:** placeholder  
**Deciders:** Soubarna Karmakar

## Context

A shared glyph atlas with LRU eviction must fit within 256 MB VRAM budget at steady-state with 4 open panes on the reference machine (RTX 2050 4 GB total VRAM).

## Spike S2 goal

Harness: `tools/spikes/s2-vram-eviction/`  
Scenario: 4 panes, each running a `htop`-style high-churn output at 60 Hz for 60 s.  
Measure: peak VRAM via DXGI `QueryVideoMemoryInfo`.

## Decision threshold

- **Peak VRAM ≤ 256 MB** → shared atlas with LRU confirmed.
- **Peak VRAM > 256 MB** → investigate per-pane atlas, atlas size caps, or fallback font rasterization.

## Decision

*Pending S2 results. Update with eviction policy and measured VRAM budget.*

## Consequences

*TBD.*
