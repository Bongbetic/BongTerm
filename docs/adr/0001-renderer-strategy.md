# ADR-001: Renderer Strategy

**Status:** Accepted  
**Date:** 2026-05-27  
**Deciders:** Soubarna Karmakar (sole author)

## Context

BongTerm needs a terminal renderer on Windows 11. Three approaches were evaluated:

- **Approach A:** Use WezTerm's full application (fork wezterm-gui), rebase periodically.
- **Approach B (selected):** Own the renderer (`bongterm-render`) using `wgpu` (D3D12 backend) + `glyphon` (text shaping + glyph atlas). Reuse `wezterm-term` (VT state machine), `wezterm-mux` (pane/tab), and `termwiz` (VT parser) via vendored submodule. BongTerm owns surface types (`SurfaceSnapshot`, `CellRun`, `CursorState`, `DirtyRegion`) — `bongterm-render` consumes these exclusively and never imports wezterm-term/termwiz directly (R1 isolation).
- **Approach C (fallback):** Use vendored `wezterm-gui` renderer directly. Reduces NIH risk but couples BongTerm to upstream rendering decisions.

## Decision

**Approach B — own wgpu + glyphon renderer from MVP-0.**

Rationale:
- Approach A forks too much; divergence compounds over time.
- Approach B reuses WezTerm parser/state (the hard part) while owning rendering (needed for Windows DPI v2, DXGI device-loss recovery, JobObject-aware VRAM enforcement, custom frame pacing, UIA accessibility surface).
- Approach C deferred as fallback only — triggers if Wave 0 spikes (ADR-002 + ADR-003 + ADR-004a) show Approach B latency/VRAM/integration targets cannot be met on reference HW.

**R1 isolation rule (binding):** `bongterm-render` is only permitted to import `bongterm-term`. It must never import `wezterm-term`, `termwiz`, or any vendored WezTerm crate directly. Breaking this rule requires an ADR update.

## Consequences

- Wave 0 spikes S1 (latency), S2 (VRAM), S3a (Iced device integration), S3b (IME) gate the product renderer implementation.
- `bongterm-render` exists as scaffold-only in Phase 0; product implementation begins only after ADR-002, ADR-003, ADR-004a, ADR-004b are all "Accepted".
- If any spike result contradicts Approach B (p99 > 16 ms after two optimization passes, VRAM > 256 MB at 4 panes, integration shape unworkable, IME unworkable), this ADR must be updated to Approach C and `bongterm-render` replaced with vendored `wezterm-gui` renderer.

## Approach C trigger conditions (§7.4 of spec)

| Condition | Threshold |
|---|---|
| p99 keystroke-to-glyph latency | > 16 ms after two optimization passes |
| VRAM at 4-pane steady state | > 256 MB on reference HW (RTX 2050 4 GB) |
| Iced + bongterm-render device integration | No viable shape found in S3a |
| IME composition | CJK input broken on all S3a shapes |

## Reference hardware

Ryzen 5 7535HS / 16 GB RAM / NVIDIA RTX 2050 4 GB VRAM / Radeon 660M iGPU / Win11 24H2.
