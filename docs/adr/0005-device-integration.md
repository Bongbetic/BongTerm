# ADR-005: Iced + bongterm-render Device Integration Shape

**Status:** Pending — written at end of Spike S3a  
**Date:** placeholder  
**Deciders:** Soubarna Karmakar

## Context

`bongterm-render` (wgpu) and `bongterm-ui` (Iced 0.14) each need a wgpu device. Three integration shapes are under evaluation:

| Shape | Description | Key risk |
|---|---|---|
| **(a) Iced Shader widget** | `bongterm-render` renders into a texture; Iced composites it via a custom shader widget | Texture readback cost; API stability of Iced's shader widget |
| **(b) Multi-window HWND** | bongterm-render owns separate native HWND per pane; Iced UI owns its own window | Window management complexity; z-order and focus issues |
| **(c) Render-to-texture** | bongterm-render draws into a shared wgpu surface; Iced draws UI chrome on top | Device sharing protocol; synchronization |

## Spike S3a goal

Harness: `tools/spikes/s3a-device-integration/`  
Prove: the chosen shape can render 4 terminal panes at 60 Hz without tearing or device-loss on reference HW.

## Decision

*Pending S3a results. Update with selected shape and rationale.*

## Consequences

Phase 1 task 1.A.2 (Iced shell) and 1.C.1 (bongterm-render real device) depend on this decision.  
ADR-006 (IME) depends on this decision.
