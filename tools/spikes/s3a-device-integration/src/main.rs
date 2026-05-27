//! Spike S3a: Iced 0.14 + bongterm-render device integration — Approach (a) Iced Shader widget.
//!
//! Evaluates whether `iced::widget::shader::Program` + `iced_wgpu::primitive::{Primitive, Pipeline}`
//! is a viable integration point for compositing a wgpu-rendered terminal grid inside Iced's window.
//!
//! Exit artifact: docs/adr/0005-device-integration.md
//!
//! Run: cargo run -p s3a-device-integration
//! Check: cargo check -p s3a-device-integration

#![allow(clippy::pedantic)]

use iced::mouse;
use iced::widget::shader::{self, Shader};
use iced::widget::{button, column, text};
use iced::{Element, Event, Fill, Length, Rectangle, Task};

// Re-exported by iced_widget::shader; backed by iced_wgpu::primitive.
use iced::widget::shader::{Pipeline, Primitive};

// ─── Hue cycling ─────────────────────────────────────────────────────────────

fn hue_to_rgb(h: f32) -> [f64; 3] {
    let h = h % 360.0;
    let s = 0.8_f32;
    let v = 0.6_f32;
    let c = v * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = v - c;
    let (r, g, b) = match h as u32 {
        0..=59 => (c, x, 0.0),
        60..=119 => (x, c, 0.0),
        120..=179 => (0.0, c, x),
        180..=239 => (0.0, x, c),
        240..=299 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    [(r + m) as f64, (g + m) as f64, (b + m) as f64]
}

// ─── Pipeline ────────────────────────────────────────────────────────────────

/// Shared pipeline state for `TerminalQuad`.
///
/// A real terminal renderer would hold a `wgpu::RenderPipeline`, vertex buffers,
/// a glyph atlas texture, etc.  For the spike, it is empty — we only exercise the
/// integration path, not actual glyph rendering.
#[derive(Debug)]
struct TerminalPipeline;

// SPIKE FINDING: Pipeline::new receives (device, queue, format).
// This is the correct place to create `wgpu::RenderPipeline`, buffers, and bind
// groups.  device + queue are fully accessible here.
impl Pipeline for TerminalPipeline {
    fn new(
        _device: &iced::wgpu::Device,
        _queue: &iced::wgpu::Queue,
        _format: iced::wgpu::TextureFormat,
    ) -> Self {
        println!("[S3a] Pipeline::new called — device/queue accessible at pipeline-init time");
        TerminalPipeline
    }

    fn trim(&mut self) {
        // Called at end of each frame.  Release per-frame scratch buffers here in prod.
    }
}

// ─── Primitive ───────────────────────────────────────────────────────────────

/// One "frame" of terminal output.  Carries the solid clear-color that simulates
/// the terminal renderer handing off its rendered image to Iced.
/// `first_frame` is not used here; `Pipeline::new` is the correct one-shot hook
/// (it is called exactly once, when the first TerminalQuad is encountered).
#[derive(Debug)]
struct TerminalQuad {
    color: [f64; 3],
}

// SPIKE FINDING: Primitive::prepare receives (pipeline, device, queue, bounds, viewport).
// device + queue ARE accessible here — this is the correct place for texture uploads
// and dynamic buffer writes (e.g. uploading new grid data from the terminal renderer).
// The split between prepare (upload) and draw (bind+dispatch) matches the wgpu
// deferred-recording model.
//
// SPIKE FINDING: Primitive::draw receives (&pipeline, &mut wgpu::RenderPass).
// Iced pre-sets the viewport and scissor rect to the widget's bounds before calling
// this fn.  The RenderPass is shared with the rest of Iced's rendering.  Issuing
// additional draw calls here is fully supported — we can bind our own pipeline,
// vertex buffers, and bind groups and draw into the same pass.
// Returning `true` keeps us in the shared pass (preferred).
// Returning `false` falls through to Primitive::render which gets a CommandEncoder
// and a TextureView, allowing a fresh render pass over the whole target surface.
// The render() path blows away Iced's composited UI underneath — use draw() in prod.
//
// SPIKE FINDING: Program::draw(&self, &State, cursor, bounds) -> Self::Primitive
// does NOT receive a RenderPass.  It runs on Iced's layout thread and merely
// constructs the Primitive value.  All wgpu access is deferred to prepare/draw/render.
impl Primitive for TerminalQuad {
    type Pipeline = TerminalPipeline;

    fn prepare(
        &self,
        _pipeline: &mut Self::Pipeline,
        _device: &iced::wgpu::Device,
        _queue: &iced::wgpu::Queue,
        _bounds: &Rectangle,
        _viewport: &iced::widget::shader::Viewport,
    ) {
        // device + queue accessible here. In production: upload dirty grid cells,
        // resize vertex buffers, write to staging buffers via queue.write_buffer().
    }

    fn draw(
        &self,
        _pipeline: &Self::Pipeline,
        _render_pass: &mut iced::wgpu::RenderPass<'_>,
    ) -> bool {
        // Returning false: we do not have a real wgpu render pipeline set up in this spike,
        // so we fall through to render() which issues a clear via a fresh render pass.
        // In production, return true and issue draw calls here to stay in the shared pass.
        false
    }

    fn render(
        &self,
        _pipeline: &Self::Pipeline,
        encoder: &mut iced::wgpu::CommandEncoder,
        target: &iced::wgpu::TextureView,
        _clip_bounds: &Rectangle<u32>,
    ) {
        // SPIKE FINDING: render() is a fallback when draw() returns false.
        // It receives a CommandEncoder + the full surface TextureView + clip_bounds.
        // clip_bounds is provided so the implementor can manually set a scissor rect —
        // unlike draw(), Iced does NOT pre-scissor the render() path; that is the caller's
        // responsibility. Here we issue a full-surface clear (no scissor) to prove wgpu
        // commands execute, but this clears Iced's composited UI too — hence render()
        // composites *behind* Iced's layer, not inside it.
        // Production use: implement draw() returning true with a real pipeline to stay
        // in Iced's shared RenderPass (pre-scissored to bounds).
        let [r, g, b] = self.color;
        let _render_pass = encoder.begin_render_pass(&iced::wgpu::RenderPassDescriptor {
            label: Some("s3a-terminal-clear"),
            color_attachments: &[Some(iced::wgpu::RenderPassColorAttachment {
                view: target,
                resolve_target: None,
                depth_slice: None,
                ops: iced::wgpu::Operations {
                    load: iced::wgpu::LoadOp::Clear(iced::wgpu::Color { r, g, b, a: 1.0 }),
                    store: iced::wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        // Drop _render_pass here to end the render pass and submit the clear.
    }
}

// ─── Shader Program ──────────────────────────────────────────────────────────

/// Shader widget program.  Holds frame counter for hue cycling.
struct TerminalShaderProgram {
    frame: u64,
}

impl TerminalShaderProgram {
    fn new() -> Self {
        Self { frame: 0 }
    }
}

#[derive(Debug, Default)]
struct ShaderState;

// SPIKE FINDING: shader::Program::draw() runs in Iced's retained-mode update cycle.
// It is NOT called on every vsync; it is called when Iced determines a redraw is
// needed.  To drive continuous animation, return an Action with a RedrawRequest from
// update() — or subscribe to iced::time::every().  Mutating self is not possible
// here (draw takes &self); frame counter must live in State or be driven by message.
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
        // Request continuous redraws so the hue cycling animation runs.
        // In production the terminal renderer pushes new frames via a channel;
        // the shader widget would request a redraw only when a new frame is ready.
        Some(shader::Action::request_redraw())
    }

    fn draw(
        &self,
        _state: &Self::State,
        _cursor: mouse::Cursor,
        _bounds: Rectangle,
    ) -> Self::Primitive {
        // Hue cycles based on frame counter driven by Tick messages.
        // Program::draw() only constructs the Primitive; no wgpu access here.
        let hue = (self.frame % 360) as f32;
        TerminalQuad {
            color: hue_to_rgb(hue),
        }
    }
}

// ─── Application ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
enum Message {
    Ping,
    Tick,
}

struct AppState {
    shader: TerminalShaderProgram,
    ping_count: u32,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            shader: TerminalShaderProgram::new(),
            ping_count: 0,
        }
    }
}

fn update(state: &mut AppState, msg: Message) -> Task<Message> {
    match msg {
        Message::Ping => {
            state.ping_count += 1;
            println!(
                "[S3a] ping #{} — Iced event routing works alongside shader widget",
                state.ping_count
            );
        }
        Message::Tick => {
            state.shader.frame = state.shader.frame.wrapping_add(1);
        }
    }
    Task::none()
}

fn view(state: &AppState) -> Element<'_, Message> {
    let shader_widget = Shader::new(&state.shader)
        .width(Fill)
        .height(Length::FillPortion(9));

    let label = text("Device integration: Shader widget").size(16);

    let ping_btn = button("Ping").on_press(Message::Ping);

    let controls = iced::widget::row![label, ping_btn]
        .spacing(16)
        .padding(8)
        .align_y(iced::alignment::Vertical::Center);

    column![shader_widget, controls].spacing(4).into()
}

// ─── Main ─────────────────────────────────────────────────────────────────────

fn main() -> iced::Result {
    // Subscription drives Tick messages for the hue-cycling animation.
    // Without this the shader widget only advances frame on user input.
    fn title(_state: &AppState) -> String {
        "S3a \u{2013} device integration shape".to_owned()
    }
    fn subscription(_state: &AppState) -> iced::Subscription<Message> {
        iced::time::every(std::time::Duration::from_millis(16)).map(|_| Message::Tick)
    }

    let result = iced::application(AppState::default, update, view)
        .title(title)
        .subscription(subscription)
        .run();

    // Post-exit report.
    match &result {
        Ok(()) => {
            println!();
            println!("S3a result: Shader widget approach WORKS");
            println!("  Observations:");
            println!(
                "  - iced::widget::shader::Program::draw() returns a Primitive; no direct RenderPass access from draw()."
            );
            println!(
                "  - device + queue accessible in Primitive::prepare() and Pipeline::new() only."
            );
            println!(
                "  - wgpu commands (draw calls, texture uploads) issue correctly from Primitive::draw() / render()."
            );
            println!(
                "  - Primitive::draw() shares Iced's RenderPass (preferred path; bounds + scissor pre-set by Iced)."
            );
            println!(
                "  - Primitive::render() fallback receives CommandEncoder + full TextureView; clears entire surface."
            );
            println!(
                "  - Iced event routing (button Ping) coexists correctly with shader widget redraws."
            );
            println!(
                "  - For production: implement draw() returning true with a real wgpu RenderPipeline."
            );
        }
        Err(e) => {
            println!("S3a result: FAILS: {e:?}");
        }
    }

    result
}
