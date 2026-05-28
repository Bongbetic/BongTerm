# ADR-008: wgpu Version Correction — 27 (not 22)

**Status:** Accepted
**Date:** 2026-05-28
**Deciders:** Soubarna Karmakar
**Supersedes (partial):** ADR-003 §"Dependency finding", ADR-005 §"Consequences" wgpu pin item

## Context

ADR-003 and ADR-005 both recorded "bump workspace wgpu pin from `0.20` to `22`" based on
the observation that glyphon 0.6 requires wgpu ≥ 22. At the time the spikes ran, the
Iced version and its wgpu requirement were not yet confirmed by `cargo metadata`.

Before starting Phase 1.C.1 the exact wgpu requirement was verified against the locked
dependency tree.

## Findings

| Dependency | wgpu required |
|------------|--------------|
| `iced_wgpu 0.14.0` | `^27.0` (confirmed: `cargo metadata`) |
| `glyphon 0.6.0` | `^22` |
| `glyphon 0.11.0` | `=29.0.0` |
| `cryoglyph 0.1.0` | `=27` |

**`iced_wgpu 0.14.0` re-exports wgpu** at `pub use wgpu;` (src/lib.rs:54 in registry).
**`iced 0.14.0`** re-exports it further at `pub use iced_renderer::wgpu::wgpu;` when the
`wgpu` feature is enabled. Consumers can import `iced_wgpu::wgpu` or `iced::wgpu` for
type-consistent wgpu 27 access without a separate top-level `wgpu` dep.

**glyphon is superseded.** Neither glyphon 0.6 (wgpu 22) nor glyphon 0.11 (wgpu 29) is
compatible with wgpu 27. `cryoglyph 0.1.0` is iced-rs's glyphon fork, uses wgpu 27,
and exposes the same public API (`TextAtlas`, `TextRenderer`, `TextBounds`, `Viewport`,
`TextAtlas::trim()`). It is already present in the locked dep tree via `iced_wgpu`.

## Decision

1. **Workspace wgpu pin**: `"0.20"` → `"27"`. All product crates that need wgpu types
   use the re-export (`iced_wgpu::wgpu`) rather than adding a top-level wgpu dep.
2. **Remove `glyphon` from `[workspace.dependencies]`**. No product crate uses it.
3. **Add `cryoglyph = "0.1"` to `[workspace.dependencies]`** for use in `bongterm-render`
   (Phase 1.C.2 shared glyph atlas per ADR-004).
4. **Delete Wave 0 spike harnesses** (`tools/spikes/s1–s4`) — all ADRs accepted;
   per `Cargo.toml` lifecycle comment they are removed at this boundary.

## Consequences

- Phase 1.C.1 (`bongterm-render` real device): implement `Pipeline` + `Primitive` via
  `iced_wgpu::primitive`; import wgpu types as `iced_wgpu::wgpu::*`.
- Phase 1.C.2 (glyph atlas): use `cryoglyph::{TextAtlas, TextRenderer, Viewport}`.
  ADR-004 strategy (per-frame `trim()`, drop-and-recreate on budget overflow) applies
  unchanged; API is identical.
- ADR-003 §"Dependency finding" ("bump to 22") is superseded by this ADR.
- ADR-005 §"Consequences" wgpu pin item ("22.x") is superseded by this ADR.
- The 3-wgpu-version situation in `Cargo.lock` (0.20.1, 22.1.0, 27.0.1) collapses
  to a single version (27.0.1) once the workspace pin and spike harnesses are removed.
