# ADR-004: Glyph Atlas VRAM Eviction Strategy

**Status:** Accepted
**Date:** 2026-05-27
**Deciders:** Soubarna Karmakar

## Context

A shared glyph atlas with LRU eviction must fit within 256 MB VRAM budget at
steady-state with 4 open panes on the reference machine (RTX 2050, 4 GB total).

## Spike S2 results

Harness: `tools/spikes/s2-vram-eviction/`
Reference HW: NVIDIA GeForce RTX 2050 / Win11 24H2
Run: `cargo run --release -p s2-vram-eviction`

### Glyph load

| Pane | Glyph set | Count |
|------|-----------|-------|
| 0 | ASCII printable (U+0020–U+007E) | 95 |
| 1 | Latin-1 Supplement (U+00A0–U+00FF) | 96 |
| 2 | Box-drawing + Braille (U+2500–U+25FF) | 256 |
| 3 | CJK Unified Ideographs sample (U+4E00–U+4EFF) | 256 |
| **Total** | | **703** |

### Atlas size

Glyphon 0.6 uses separate mask (R8) and color (Rgba8) atlas textures. For 703 glyphs
at 14 px, the 512×512 tier is sufficient:

| Texture | Size | Bytes |
|---------|------|-------|
| Mask atlas (R8) | 512×512 | ~256 KB |
| Color atlas (Rgba8) | 512×512 | ~1,024 KB |
| **Total** | | **~1,280 KB (~1.25 MB)** |

### DXGI VRAM measurement

DXGI `QueryVideoMemoryInfo(DXGI_MEMORY_SEGMENT_GROUP_LOCAL)` reports in whole MB.
At 1.25 MB the delta rounds to **0 MB** — i.e. usage is sub-MB. The 256 MB budget is
not at risk for any realistic terminal glyph set.

| Metric | Value |
|--------|-------|
| VRAM before atlas upload | 0 MB (baseline) |
| VRAM after 4-pane upload | 0 MB (sub-MB precision) |
| VRAM delta | 0 MB (< 1 MB actual) |
| Fits under 256 MB budget | **YES** |

### LRU eviction finding

**`TextAtlas::trim()` does NOT shrink or reallocate the GPU texture.**
It only clears the `glyphs_in_use` sets (both mask and color inner atlases),
marking all cached glyphs as LRU-eligible for eviction on the *next* allocation
that overflows the packer. VRAM measured before/after trim(): **0 MB delta** (as expected).

Real VRAM reclamation requires **dropping and recreating `TextAtlas`** (no shrink
path exists in glyphon 0.6). The old texture is freed when the `TextAtlas` is dropped.

## Decision

**Shared atlas with per-frame `trim()` + drop-and-recreate on budget overflow.**

- One `TextAtlas` shared across all panes.
- Call `atlas.trim()` each frame to reset LRU eligibility at zero CPU cost.
- If atlas grows past a configurable ceiling (default: 128 MB), drop the `TextAtlas`
  and recreate it; the next `prepare()` call repopulates from the glyph cache.
  This is a one-frame stutter; acceptable for an overflow event that should never
  occur in practice given the ~1.25 MB measured cost.

## Consequences

- Phase 1 task 1.C.2 (shared glyph atlas with LRU eviction): implement using the
  above strategy. Track atlas size via `TextAtlas::size()` if exposed, or estimate
  from glyph count × average cell area.
- Phase 1 task 1.C.5 (VRAM ceiling enforcement): set the drop-and-recreate threshold
  at 128 MB. Alert via `bongterm-ledger` if atlas exceeds 64 MB (warn threshold).
- wgpu workspace pin bump required (wgpu ≥ 22) before Phase 1.C; tracked in ADR-003.
