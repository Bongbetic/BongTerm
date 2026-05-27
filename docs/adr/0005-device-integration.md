# ADR-005: Iced + bongterm-render Device Integration Shape

**Status:** Accepted
**Date:** 2026-05-27
**Deciders:** Soubarna Karmakar

## Context

`bongterm-render` (wgpu) and `bongterm-ui` (Iced 0.14) both need access to a wgpu
device and surface. Three integration shapes were evaluated:

| Shape | Description |
|---|---|
| **(a) Iced Shader widget** | Terminal renders into Iced's existing wgpu render pass via `iced::widget::shader::Program` + custom `Primitive` |
| **(b) Multi-window HWND** | `bongterm-render` owns a separate native HWND per pane; Iced owns its own HWND |
| **(c) Render-to-texture** | `bongterm-render` draws off-screen; Iced blits the texture via its own renderer |

## Spike S3a results

Harness: `tools/spikes/s3a-device-integration/`
Run: `cargo run -p s3a-device-integration`

### API findings (statically verified, compile confirmed)

1. **`Pipeline::new(device, queue, format)`** — called exactly once when the first
   `TerminalQuad` primitive is encountered. `device` + `queue` are fully accessible here.
   Correct place to create `wgpu::RenderPipeline`, vertex buffers, glyph atlas texture,
   and bind groups.

2. **`Primitive::prepare(pipeline, device, queue, bounds, viewport)`** — called every
   frame before draw. `device` + `queue` accessible. Correct place for per-frame uploads
   (dirty grid cells, staging buffer writes via `queue.write_buffer`).

3. **`Primitive::draw(&pipeline, &mut wgpu::RenderPass) -> bool`** — receives Iced's
   shared `RenderPass` with viewport and scissor rect pre-set to widget bounds.
   Returning `true` stays in Iced's pass (preferred production path).
   Additional draw calls, pipeline binds, and vertex buffer binds all work here.

4. **`Primitive::render(&mut CommandEncoder, &TextureView, clip_bounds)`** — fallback
   when `draw()` returns `false`. Receives a `CommandEncoder` + full surface
   `TextureView`. Scissor is NOT pre-applied; caller must set it from `clip_bounds`.
   This path issues commands that render *behind* Iced's UI layer, not composited inside
   it. Not suitable for production terminal rendering.

5. **`Program::draw(&self, state, cursor, bounds)`** — runs on Iced's retained-mode
   update cycle. Constructs `Self::Primitive` only. No wgpu access. Not called every
   vsync; driven by `shader::Action::request_redraw()` from `Program::update()`.

## Decision

**Adopt Approach (a): Iced Shader widget.**

Production path: implement `Primitive::draw()` returning `true`, issuing wgpu draw calls
from within Iced's shared `RenderPass`. The render pipeline, glyph atlas, and vertex
buffers live in `Pipeline`. Per-frame uploads happen in `Primitive::prepare()`.

## Rejected alternatives

**Approach (b) Multi-window HWND:** Adds Win32 z-order, focus, and DPI management per
pane. Iced's layout engine cannot reason about foreign HWNDs. Deferred to Phase 6
investigation only if single-HWND compositing proves insufficient for > 8 panes.

**Approach (c) Render-to-texture:** Duplicates the compositing work Iced's Shader widget
already provides. Adds a texture readback / blit step with no latency or quality benefit
on a DX12/wgpu pipeline that can share the render pass natively.

## Consequences

- Phase 1 task 1.A.2 (Iced shell): wire the Iced application with `iced::application()`.
- Phase 1 task 1.C.1 (`bongterm-render` real device): implement `Pipeline` and
  `Primitive` in `bongterm-render`; the `RendererBackend` port trait wraps them.
- ADR-006 (IME composition) must target the single-HWND shape — IME candidate window is
  parented to the Iced HWND.
- wgpu dependency: **workspace pin must be bumped from 0.20 to 22.x before Phase 1.C**.
  glyphon 0.6 requires wgpu ≥ 22 (confirmed by S1/S2 spike compilation). Update
  `[workspace.dependencies]` in the root `Cargo.toml` at the Phase 1.C boundary.
