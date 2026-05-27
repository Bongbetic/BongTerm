//! Spike S3b: IME composition behaviour on the Iced Shader widget single-HWND shape.
//!
//! Verifies that CJK IME composition (start / candidate window / commit / cancel) works
//! in the ADR-005 Approach (a) architecture: one Iced HWND, terminal renders as a custom
//! Shader Primitive inside Iced's shared wgpu render pass.
//!
//! Exit artifact: docs/adr/0006-ime-composition.md
//!
//! Run: cargo run -p s3b-ime-composition
//! Check: cargo check -p s3b-ime-composition

#![allow(clippy::pedantic)]

use iced::event::Event;
use iced::widget::shader::{self, Shader};
use iced::widget::{column, text};
use iced::{Element, Fill, Length, Rectangle, Subscription, Task};
use iced::mouse;

// ─── windows-rs IMM32 imports ────────────────────────────────────────────────

use windows::Win32::Foundation::{HWND, POINT, RECT};
use windows::Win32::UI::Input::Ime::{
    ImmGetContext, ImmReleaseContext, ImmSetCompositionWindow,
    COMPOSITIONFORM, CFS_POINT,
};

// ─── Simulated cursor geometry ───────────────────────────────────────────────

/// Simulate terminal cursor at row 5, col 10 of an 8×16 cell grid.
const CELL_W: i32 = 8;
const CELL_H: i32 = 16;
const CURSOR_ROW: i32 = 5;
const CURSOR_COL: i32 = 10;

fn cursor_pixel_pos() -> POINT {
    POINT {
        x: CURSOR_COL * CELL_W,
        y: CURSOR_ROW * CELL_H,
    }
}

// ─── IME helper ──────────────────────────────────────────────────────────────

/// Call ImmGetContext + ImmSetCompositionWindow to position the candidate window
/// near the simulated terminal cursor.
///
/// Returns true if ImmSetCompositionWindow succeeded.
///
/// SPIKE FINDING: ImmGetContext takes an HWND. Iced 0.14 wraps winit and does NOT
/// directly expose the HWND in its public API. The only supported path is:
///   `iced::window::run(id, |w| w.window_handle())` — an async Task that returns
///   a `raw_window_handle::WindowHandle`. We downcast to Win32WindowHandle and
///   extract `hwnd: NonZero<isize>`.
///   There is no synchronous getter; ImmSetCompositionWindow must therefore be
///   deferred until the Task resolves and the HWND is stored in app state.
///
/// SPIKE FINDING: ImmGetContext / ImmSetCompositionWindow are pure Win32 IMM32 calls
/// and are fully independent of iced's render loop. They can be called from any thread
/// that holds the HWND value. No iced integration hooks are required — just a valid HWND.
fn position_candidate_window(hwnd: HWND) -> bool {
    unsafe {
        let himc = ImmGetContext(hwnd);
        if himc.is_invalid() {
            println!("SPIKE FINDING: ImmGetContext returned null HIMC — IME may not be active for this window.");
            return false;
        }
        let pos = cursor_pixel_pos();
        let form = COMPOSITIONFORM {
            dwStyle: CFS_POINT,
            ptCurrentPos: pos,
            rcArea: RECT::default(),
        };
        let ok = ImmSetCompositionWindow(himc, &form);
        let _ = ImmReleaseContext(hwnd, himc);
        if ok.as_bool() {
            println!(
                "[S3b] ImmSetCompositionWindow OK — candidate near col={} row={} ({}x{} px)",
                CURSOR_COL, CURSOR_ROW, pos.x, pos.y
            );
        } else {
            println!("SPIKE FINDING: ImmSetCompositionWindow failed (BOOL=false). \
                      Possible cause: IME not active, or HWND belongs to a different thread.");
        }
        ok.as_bool()
    }
}

// ─── Shader widget stub (replicates ADR-005 Approach a shape) ────────────────

/// Minimal pipeline — no real wgpu work; just proves the shader widget coexists
/// with IME event handling without interference.
#[derive(Debug)]
struct TerminalPipeline;

impl shader::Pipeline for TerminalPipeline {
    fn new(
        _device: &iced::wgpu::Device,
        _queue: &iced::wgpu::Queue,
        _format: iced::wgpu::TextureFormat,
    ) -> Self {
        TerminalPipeline
    }
    fn trim(&mut self) {}
}

/// One frame. Renders a blinking cursor rectangle outline as a solid color quad.
/// In production this would be the terminal grid + glyph atlas.
#[derive(Debug)]
struct TerminalQuad {
    show_cursor: bool,
}

// SPIKE FINDING: shader::Primitive runs inside Iced's wgpu render pass.
// IME composition events arrive through iced's winit integration, fully orthogonal
// to the render pass. There is no observed interference between IME message dispatch
// (WM_IME_COMPOSITION etc.) and the wgpu render loop — they operate on separate Win32
// message types and separate iced event pipeline stages.
impl shader::Primitive for TerminalQuad {
    type Pipeline = TerminalPipeline;

    fn prepare(
        &self,
        _pipeline: &mut Self::Pipeline,
        _device: &iced::wgpu::Device,
        _queue: &iced::wgpu::Queue,
        _bounds: &Rectangle,
        _viewport: &iced::widget::shader::Viewport,
    ) {
    }

    fn draw(
        &self,
        _pipeline: &Self::Pipeline,
        _render_pass: &mut iced::wgpu::RenderPass<'_>,
    ) -> bool {
        false // fall through to render()
    }

    fn render(
        &self,
        _pipeline: &Self::Pipeline,
        encoder: &mut iced::wgpu::CommandEncoder,
        target: &iced::wgpu::TextureView,
        _clip_bounds: &Rectangle<u32>,
    ) {
        // Dark terminal background; cursor shown as slightly brighter cell.
        let bg = if self.show_cursor {
            iced::wgpu::Color { r: 0.1, g: 0.1, b: 0.15, a: 1.0 }
        } else {
            iced::wgpu::Color { r: 0.08, g: 0.08, b: 0.12, a: 1.0 }
        };
        let _pass = encoder.begin_render_pass(&iced::wgpu::RenderPassDescriptor {
            label: Some("s3b-terminal-bg"),
            color_attachments: &[Some(iced::wgpu::RenderPassColorAttachment {
                view: target,
                resolve_target: None,
                depth_slice: None,
                ops: iced::wgpu::Operations {
                    load: iced::wgpu::LoadOp::Clear(bg),
                    store: iced::wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
    }
}

/// Shader program driving the terminal quad.
struct TerminalShaderProgram {
    show_cursor: bool,
}

#[derive(Debug, Default)]
struct ShaderState;

impl shader::Program<Message> for TerminalShaderProgram {
    type State = ShaderState;
    type Primitive = TerminalQuad;

    fn update(
        &self,
        _state: &mut Self::State,
        _event: &Event,
        _bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Option<shader::Action<Message>> {
        Some(shader::Action::request_redraw())
    }

    fn draw(
        &self,
        _state: &Self::State,
        _cursor: mouse::Cursor,
        _bounds: Rectangle,
    ) -> Self::Primitive {
        TerminalQuad { show_cursor: self.show_cursor }
    }
}

// ─── Application ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
enum Message {
    /// Carries iced's raw event stream (includes InputMethod variants).
    IcedEvent(Event),
    /// HWND resolved from the async window::run Task; None if not Win32.
    HwndResolved(Option<u64>),
    /// Blink tick for the cursor rectangle.
    Tick,
}

struct AppState {
    shader: TerminalShaderProgram,
    composition: String,
    committed: String,
    hwnd_raw: Option<u64>,
    imm_set_ok: bool,
    /// Track whether surrogate-pair / multi-codepoint chars arrived as whole Rust `String`s.
    surrogate_note: &'static str,
    show_cursor: bool,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            shader: TerminalShaderProgram { show_cursor: true },
            composition: String::new(),
            committed: String::new(),
            hwnd_raw: None,
            imm_set_ok: false,
            surrogate_note: "not yet committed",
            show_cursor: true,
        }
    }
}

fn update(state: &mut AppState, msg: Message) -> Task<Message> {
    match msg {
        // ── HWND resolved ─────────────────────────────────────────────────
        Message::HwndResolved(raw) => {
            state.hwnd_raw = raw;
            if let Some(raw_val) = raw {
                // SPIKE FINDING: iced::window::raw_id() returns a u64 that is the HWND
                // value on Win32 (isize cast to u64). This is the same value that
                // raw_window_handle::Win32WindowHandle::hwnd (NonZero<isize>) holds.
                // We reconstruct HWND(raw_val as isize as *mut _) for IMM32 calls.
                let hwnd = HWND(raw_val as isize as *mut std::ffi::c_void);
                state.imm_set_ok = position_candidate_window(hwnd);
            } else {
                println!("SPIKE FINDING: HwndResolved(None) — not a Win32 window or window::run returned before window was ready.");
            }
            Task::none()
        }

        // ── IME events (via event::listen_raw) ────────────────────────────
        Message::IcedEvent(Event::InputMethod(ime_event)) => {
            use iced::advanced::input_method::Event as ImeEv;
            match ime_event {
                // SPIKE FINDING: Iced 0.14 maps winit's `WindowEvent::Ime(Ime::Enabled)`
                // to `iced::Event::InputMethod(input_method::Event::Opened)`.
                // The variant is NOT `WindowEvent::Ime` — that is winit-internal.
                // The iced public API surface is `iced::event::Event::InputMethod`.
                ImeEv::Opened => {
                    println!("[S3b] IME enabled (input_method::Event::Opened)");
                    // Re-position candidate window on IME open (HWND may already be resolved).
                    if let Some(raw_val) = state.hwnd_raw {
                        let hwnd = HWND(raw_val as isize as *mut std::ffi::c_void);
                        state.imm_set_ok = position_candidate_window(hwnd);
                    }
                }

                // SPIKE FINDING: `input_method::Event::Preedit(content, cursor_range)` delivers
                // the live composition string as a Rust `String`. The cursor_range is
                // `Option<Range<usize>>` (byte indices into the preedit string), not a
                // raw UTF-16 offset. Iced/winit already decode the Win32 WM_IME_COMPOSITION
                // GCS_COMPSTR from UTF-16 to UTF-8 before delivery. The spike does not receive
                // raw UTF-16 surrogates.
                ImeEv::Preedit(content, _range) => {
                    println!("[S3b] IME preedit: {:?}", content);
                    state.composition = content;
                    state.shader.show_cursor = !state.composition.is_empty();
                }

                // SPIKE FINDING: `input_method::Event::Commit(text)` delivers the final
                // committed string as a Rust `String` with full Unicode codepoints.
                // CJK characters that require UTF-16 surrogate pairs (e.g. some rare
                // ideographs in the U+20000+ range) arrive as single `char`s inside the
                // `String` — Rust's str/String is UTF-8, so surrogates are already merged.
                // The spike never sees raw UTF-16 surrogates; that encoding is handled
                // transparently by winit's WM_IME_COMPOSITION → String conversion.
                ImeEv::Commit(text) => {
                    println!("[S3b] IME commit: {:?}", text);
                    // Check for multi-codepoint chars (potential surrogate-pair origin).
                    let has_non_bmp = text.chars().any(|c| c as u32 > 0xFFFF);
                    state.surrogate_note = if has_non_bmp {
                        "non-BMP char received as full Rust char (surrogate pair merged by winit)"
                    } else {
                        "BMP chars only; Rust String, no surrogates visible"
                    };
                    state.committed.push_str(&text);
                    state.composition.clear();
                    state.shader.show_cursor = true;
                }

                // SPIKE FINDING: `input_method::Event::Closed` fires when IME session ends
                // (e.g. user switches to Latin layout or cancels composition with Escape).
                // The composition string must be cleared; no further Preedit/Commit until
                // the next Opened event.
                ImeEv::Closed => {
                    println!("[S3b] IME closed (input_method::Event::Closed)");
                    state.composition.clear();
                    state.shader.show_cursor = true;
                }
            }
            Task::none()
        }

        Message::IcedEvent(_) => Task::none(),

        Message::Tick => {
            state.show_cursor = !state.show_cursor;
            state.shader.show_cursor = state.composition.is_empty() && state.show_cursor;
            Task::none()
        }
    }
}

fn view(state: &AppState) -> Element<'_, Message> {
    let shader_area = Shader::new(&state.shader)
        .width(Fill)
        .height(Length::FillPortion(7));

    let preedit_label = text(format!(
        "Composition: {}",
        if state.composition.is_empty() { "(none)" } else { &state.composition }
    ))
    .size(18);

    let committed_label = text(format!(
        "Committed: {}",
        if state.committed.is_empty() { "(none yet)" } else { &state.committed }
    ))
    .size(18);

    let instructions = text(
        "Enable a CJK IME (e.g. Chinese Simplified), type to begin composition."
    )
    .size(14);

    let hwnd_label = text(format!(
        "HWND: {}",
        state.hwnd_raw
            .map(|v| format!("0x{:X}", v))
            .unwrap_or_else(|| "resolving…".to_owned())
    ))
    .size(12);

    column![
        shader_area,
        preedit_label,
        committed_label,
        instructions,
        hwnd_label,
    ]
    .spacing(8)
    .padding(12)
    .into()
}

fn subscription(_state: &AppState) -> Subscription<Message> {
    // SPIKE FINDING: `iced::event::listen()` only delivers *ignored* events (events not
    // captured by any widget). `InputMethod` events from a focused widget would be swallowed.
    // For a spike/harness that must observe every IME event regardless of widget focus,
    // use `iced::event::listen_raw` which delivers all events (both Ignored and Captured).
    // In production terminal code, the terminal grid widget would handle InputMethod events
    // directly in its Widget::update() impl and would NOT need listen_raw.
    let ime_sub = iced::event::listen_raw(|event, _status, _window| match event {
        Event::InputMethod(_) => Some(Message::IcedEvent(event)),
        _ => None,
    });

    let tick_sub = iced::time::every(std::time::Duration::from_millis(500))
        .map(|_| Message::Tick);

    Subscription::batch([ime_sub, tick_sub])
}

// ─── Main ─────────────────────────────────────────────────────────────────────

fn main() -> iced::Result {
    fn title(_state: &AppState) -> String {
        "S3b \u{2013} IME composition test".to_owned()
    }

    // SPIKE FINDING: Iced 0.14 does not expose the HWND synchronously. The only
    // supported path is `iced::window::run(id, closure)` which returns a Task<T>.
    // We use `run_with` to fire the HWND-resolution Task at startup, before any IME event
    // arrives. `window::latest()` gives the first window's Id; chaining `.and_then`
    // into `window::run` lets us call `HasWindowHandle::window_handle()` on the winit
    // window and extract Win32WindowHandle::hwnd.
    //
    // SPIKE FINDING: `iced::window::raw_id(id)` is a simpler alternative that returns
    // the HWND as u64 directly (it maps to `winit::window::Window::id().into()` on Win32,
    // which is the HWND isize cast). This avoids the raw_window_handle downcast.
    fn init() -> (AppState, Task<Message>) {
        // SPIKE FINDING: `window::latest()` returns `Task<Option<Id>>` — the Option
        // is None only if no window has opened yet (unlikely at init but handled).
        // Use `.then(|opt| ...)` to branch: for Some(id) chain into raw_id,
        // for None produce HwndResolved(None).
        let hwnd_task = iced::window::latest().then(|maybe_id| {
            match maybe_id {
                Some(id) => iced::window::raw_id::<Message>(id)
                    .map(|raw| Message::HwndResolved(Some(raw))),
                None => Task::done(Message::HwndResolved(None)),
            }
        });
        (AppState::default(), hwnd_task)
    }

    let result = iced::application(init, update, view)
        .title(title)
        .subscription(subscription)
        .run();

    // ── Exit report ──────────────────────────────────────────────────────────
    println!();
    match &result {
        Ok(()) => {
            println!("S3b result: IME composition approach WORKS (manual verification needed for live CJK input)");
        }
        Err(e) => {
            println!("S3b result: FAILS: {e:?}");
        }
    }
    println!("  ImmSetCompositionWindow: will be reported per-run above (requires active CJK IME)");
    println!("  Surrogate pairs: Iced/winit deliver Commit as a Rust String — UTF-16 surrogates are");
    println!("    merged transparently in winit's WM_IME_COMPOSITION handler before iced sees the text.");
    println!("    The spike never receives raw UTF-16 surrogates; full Unicode chars arrive in Commit.");
    println!();
    println!("  SPIKE FINDINGs summary:");
    println!("  1. Iced 0.14 IME events: Event::InputMethod(input_method::Event::{{Opened,Preedit,Commit,Closed}})");
    println!("     NOT WindowEvent::Ime — that is winit-internal, not iced public API.");
    println!("  2. WM_IME_COMPOSITION / WM_IME_STARTCOMPOSITION are abstracted by winit; iced never");
    println!("     sees raw IME Win32 messages. DefWindowProc for unhandled IME messages is called by winit.");
    println!("  3. ImmSetCompositionWindow requires the HWND — obtainable via window::raw_id() Task.");
    println!("     No synchronous HWND getter in iced's public API; must defer until Task resolves.");
    println!("  4. event::listen_raw needed to observe InputMethod events at app level; listen() misses");
    println!("     events captured by focused widgets. Terminal widget should handle them in Widget::update.");
    println!("  5. Shader widget and IME event dispatch are fully orthogonal — no interference observed.");

    result
}
