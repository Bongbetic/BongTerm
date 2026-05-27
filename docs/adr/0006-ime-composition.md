# ADR-006: IME Composition on Terminal Surface

**Status:** Accepted (live CJK round-trip deferred to Phase 5 acceptance gate §6.1 #21)
**Date:** 2026-05-27
**Deciders:** Soubarna Karmakar

## Context

Windows IME (Input Method Editor) for CJK input requires the terminal to support:
- Candidate window positioned near the terminal cursor.
- Compose / cancel / commit events delivered to the terminal input handler.
- Surrogate pairs and grapheme clusters handled without corruption.

The ADR-005 (Iced Shader widget, single-HWND) decision determines the integration path.

## Spike S3b results

Harness: `tools/spikes/s3b-ime-composition/`
Run: `cargo run -p s3b-ime-composition` (requires CJK IME enabled in Windows Settings)
Reference HW: Win11 24H2 / Ryzen 5 7535HS

### API findings (statically verified from iced 0.14 + winit + windows-rs source)

1. **Iced 0.14 IME event type:** `Event::InputMethod(input_method::Event)` with variants
   `Opened`, `Preedit(String, Option<(usize, usize)>)`, `Commit(String)`, `Closed`.
   Not `WindowEvent::Ime` — that is winit-internal; iced re-exposes it with renamed types.

2. **Win32 IME messages abstracted by winit:** `WM_IME_STARTCOMPOSITION`,
   `WM_IME_COMPOSITION`, `WM_IME_ENDCOMPOSITION` are handled inside winit's WNDPROC.
   `DefWindowProc` is called for unhandled variants. Iced code never receives raw
   IME Win32 messages; no custom WNDPROC override is needed.

3. **HWND access:** Iced's public API has no synchronous HWND getter. The supported path:
   `iced::window::run(id, |w| w.window_handle())` → `raw_window_handle::Win32WindowHandle`
   → `hwnd: NonZero<isize>`. This is an async `Task`; `ImmSetCompositionWindow` must be
   deferred until the Task resolves. Store HWND in app state on first `Opened` event.

4. **IME + shader widget orthogonality:** `WM_IME_*` messages and wgpu render passes
   operate on separate Win32 message types and separate Iced pipeline stages. No
   interference observed between IME event dispatch and the render loop.

5. **Surrogate pairs:** `Event::InputMethod::Commit(String)` delivers a Rust `String`
   (UTF-8). Winit converts `GCS_COMPSTR` from UTF-16 to UTF-8 before Iced sees it.
   Raw surrogates never reach application code. Emoji and CJK Extension characters
   serialize as multi-byte UTF-8 sequences, handled correctly by Rust `String` + `char`.

6. **Candidate window positioning:** `ImmGetContext(hwnd)` + `ImmSetCompositionWindow`
   with `CFS_POINT` positions the OS candidate window at pixel coordinates computed from
   the terminal cursor cell position (col × cell_width, row × cell_height). Iced's widget
   bounds must be queried from the layout engine for accurate positioning; the spike uses
   a fixed offset from window top-left.

### Runtime test cases

| Test case | Expected behavior | Status |
|-----------|------------------|--------|
| Basic compose → commit | `Preedit` events during typing; `Commit` on selection | Phase 5 gate |
| Cancel mid-composition (Escape) | `Preedit("")` then `Closed` | Phase 5 gate |
| Surrogate pair (e.g. 𠀋 U+2000B) | `Commit("𠀋")` — 4-byte UTF-8, no split | **Confirmed by design** ¹ |
| Grapheme cluster with combining mark | `Commit("ñ")` — NFC from winit | Phase 5 gate |
| Candidate window near cursor | Window appears at (col*8, row*16) px | Phase 5 gate |
| ImmSetCompositionWindow | No active IME during S3b run; call path verified correct | Phase 5 gate |

¹ winit converts GCS_COMPSTR from UTF-16 to UTF-8 before iced sees it; confirmed by S3b static analysis
  and SPIKE FINDING #5: "iced/winit deliver Commit as a Rust String — UTF-16 surrogates are merged
  transparently in winit's WM_IME_COMPOSITION handler before iced sees the text."

## Decision

**IME composition handled via `Event::InputMethod` + `ImmSetCompositionWindow` on async HWND.**

Production wiring (Phase 5.A.2):
- Terminal grid widget subscribes to `Event::InputMethod` via `event::listen_raw`.
- On `Opened`: schedule `window::raw_id` Task; store HWND; call `ImmSetCompositionWindow`.
- On `Preedit(text, cursor_range)`: render composition underline in the terminal grid
  overlay; do not commit to the terminal buffer.
- On `Commit(text)`: push text into the PTY input stream as UTF-8 bytes.
- On `Closed`: clear preedit overlay.
- Update cursor position sent to `ImmSetCompositionWindow` on every cursor move.

## Consequences

- Phase 5 task: IME wired to Phase 1 renderer shape. Acceptance gate §6.1 #21 (CJK
  input functional) is gated by a real CJK round-trip test.
- Preedit rendering (underline decoration in the terminal grid) requires a dedicated
  render path in `bongterm-render` (not just the glyph atlas) — the composition string
  overlays glyphs that have not been committed yet.
- No custom WNDPROC or `WM_IME_*` handling needed in BongTerm — winit + Iced handle it.

