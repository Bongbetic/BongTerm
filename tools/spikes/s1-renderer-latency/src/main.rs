// S1 Measurement endpoints:
//   t1: Instant::now() at the top of the WindowEvent::KeyboardInput arm (before any work)
//   t2: Instant::now() after wgpu Queue::submit() returns for the frame containing the new glyph
// This captures: event dispatch -> glyph shaping -> encoder recording -> submit.
// GPU execution time is NOT included (submit is async on GPU side).
// Goal: t2 - t1 p99 < 8 ms on reference HW (Ryzen 5 7535HS / RTX 2050).

#![allow(clippy::pedantic)]

use anyhow::Context as AnyhowContext;
use glyphon::{
    Attrs, Buffer, Cache, Color, Family, FontSystem, Metrics, Resolution, Shaping, SwashCache,
    TextArea, TextAtlas, TextBounds, TextRenderer, Viewport,
};
use std::sync::Arc;
use std::time::Instant;
use tracing::info;
use wgpu::{
    CommandEncoderDescriptor, CompositeAlphaMode, DeviceDescriptor, Features, Instance,
    InstanceDescriptor, Limits, LoadOp, Operations, PresentMode, RenderPassColorAttachment,
    RenderPassDescriptor, RequestAdapterOptions, StoreOp, SurfaceConfiguration, TextureUsages,
    TextureViewDescriptor,
};
use winit::{
    application::ApplicationHandler,
    event::{ElementState, KeyEvent, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::{Window, WindowId},
};

// Terminal cell geometry: 8 px wide, 16 px tall -> 120*8 x 40*16 = 960x640
const CELL_W: u32 = 8;
const CELL_H: u32 = 16;
const WIN_W: u32 = 120 * CELL_W; // 960
const WIN_H: u32 = 40 * CELL_H;  // 640

// Measurement grid for the glyph load: 80 cols x 24 rows of 'A'
const GLYPH_COLS: usize = 80;
const GLYPH_ROWS: usize = 24;

const MAX_SAMPLES: usize = 200;

// ---------------------------------------------------------------------------
// GPU + glyphon state, created lazily in resumed()
// ---------------------------------------------------------------------------
struct GpuState {
    // Kept alive so the surface lifetime remains valid; not read directly.
    #[allow(dead_code)]
    window: Arc<Window>,
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface_config: SurfaceConfiguration,
    font_system: FontSystem,
    swash_cache: SwashCache,
    viewport: Viewport,
    atlas: TextAtlas,
    text_renderer: TextRenderer,
    buffer: Buffer,
    // Monotonically incrementing counter to vary text each keypress
    glyph_counter: u64,
}

// ---------------------------------------------------------------------------
// Top-level application
// ---------------------------------------------------------------------------
#[derive(Default)]
struct App {
    state: Option<GpuState>,
    samples: Vec<u64>, // µs per sample
}

impl App {
    fn print_stats(&self) {
        if self.samples.is_empty() {
            println!("No samples collected.");
            return;
        }
        let mut sorted = self.samples.clone();
        sorted.sort_unstable();
        let n = sorted.len();
        let idx = |pct: usize| sorted[(n * pct / 100).min(n - 1)];
        println!();
        println!("=== S1 Keystroke-to-glyph latency (submit, not GPU fence) ===");
        println!("  Samples : {}", n);
        println!("  Min     : {} µs", sorted[0]);
        println!("  p50     : {} µs", idx(50));
        println!("  p95     : {} µs", idx(95));
        println!("  p99     : {} µs", idx(99));
        println!("  Max     : {} µs", sorted[n - 1]);
        println!("  Goal    : p99 < 8000 µs  (8 ms)");
        let p99 = idx(99);
        if p99 < 8000 {
            println!("  Result  : PASS ({} µs < 8000 µs)", p99);
        } else {
            println!("  Result  : FAIL ({} µs >= 8000 µs)", p99);
        }
    }

    fn init_gpu(&mut self, event_loop: &ActiveEventLoop) -> anyhow::Result<()> {
        // --- Window ---
        let attrs = Window::default_attributes()
            .with_title("S1 – keystroke-to-glyph latency spike")
            .with_inner_size(winit::dpi::PhysicalSize::new(WIN_W, WIN_H));
        let window = Arc::new(
            event_loop
                .create_window(attrs)
                .context("create window")?,
        );

        // --- wgpu instance + surface ---
        let instance = Instance::new(InstanceDescriptor::default());

        // SAFETY: the surface holds an Arc<Window>, so the window outlives it.
        let surface = instance
            .create_surface(Arc::clone(&window))
            .context("create surface")?;

        // --- adapter + device (block on async) ---
        let rt = tokio::runtime::Runtime::new().context("tokio runtime")?;
        let (adapter, device, queue) = rt.block_on(async {
            let adapter = instance
                .request_adapter(&RequestAdapterOptions {
                    compatible_surface: Some(&surface),
                    ..Default::default()
                })
                .await
                .context("request adapter")?;

            let (device, queue) = adapter
                .request_device(
                    &DeviceDescriptor {
                        label: Some("s1-device"),
                        required_features: Features::empty(),
                        required_limits: Limits::default(),
                        memory_hints: Default::default(),
                    },
                    None,
                )
                .await
                .context("request device")?;

            anyhow::Ok((adapter, device, queue))
        })?;

        info!("Adapter: {:?}", adapter.get_info());

        // --- Surface configuration (no-vsync for unbiased latency measurement) ---
        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(surface_caps.formats[0]);

        let present_mode = if surface_caps
            .present_modes
            .contains(&PresentMode::Immediate)
        {
            PresentMode::Immediate
        } else {
            PresentMode::AutoNoVsync
        };

        let surface_config = SurfaceConfiguration {
            usage: TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: WIN_W,
            height: WIN_H,
            present_mode,
            alpha_mode: CompositeAlphaMode::Opaque,
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &surface_config);

        // --- glyphon stack ---
        let cache = Cache::new(&device);
        let viewport = Viewport::new(&device, &cache);
        let mut atlas = TextAtlas::new(&device, &queue, &cache, surface_format);
        let text_renderer = TextRenderer::new(
            &mut atlas,
            &device,
            wgpu::MultisampleState::default(),
            None,
        );

        let mut font_system = FontSystem::new();
        let swash_cache = SwashCache::new();

        // Build initial Buffer: 80 cols x 24 rows of 'A' in Consolas / Courier New
        let font_size = CELL_H as f32;
        let line_height = font_size * 1.2;
        let mut buffer = Buffer::new(&mut font_system, Metrics::new(font_size, line_height));
        buffer.set_size(&mut font_system, Some(WIN_W as f32), Some(WIN_H as f32));

        let text: String = {
            let row: String = "A".repeat(GLYPH_COLS);
            std::iter::repeat(row)
                .take(GLYPH_ROWS)
                .collect::<Vec<_>>()
                .join("\n")
        };
        buffer.set_text(
            &mut font_system,
            &text,
            Attrs::new().family(Family::Name("Consolas")),
            Shaping::Basic,
        );
        buffer.shape_until_scroll(&mut font_system, false);

        self.state = Some(GpuState {
            window,
            surface,
            device,
            queue,
            surface_config,
            font_system,
            swash_cache,
            viewport,
            atlas,
            text_renderer,
            buffer,
            glyph_counter: 0,
        });

        println!("S1: Press keys to sample latency. Close window or press 200 keys to finish.");
        Ok(())
    }

    fn on_key_pressed(&mut self, event_loop: &ActiveEventLoop) {
        // t1: captured before any work (measurement start)
        let t1 = Instant::now();

        let s = match self.state.as_mut() {
            Some(s) => s,
            None => return,
        };

        s.glyph_counter = s.glyph_counter.wrapping_add(1);

        // Vary one character so the shaper actually does work each keypress
        let ch = char::from_u32(b'A' as u32 + (s.glyph_counter % 26) as u32).unwrap_or('A');
        let text: String = {
            let row: String = ch.to_string().repeat(GLYPH_COLS);
            std::iter::repeat(row)
                .take(GLYPH_ROWS)
                .collect::<Vec<_>>()
                .join("\n")
        };
        s.buffer.set_text(
            &mut s.font_system,
            &text,
            Attrs::new().family(Family::Name("Consolas")),
            Shaping::Basic,
        );
        s.buffer.shape_until_scroll(&mut s.font_system, false);

        // Acquire surface texture
        let output = match s.surface.get_current_texture() {
            Ok(t) => t,
            Err(e) => {
                tracing::warn!("surface error: {:?}", e);
                return;
            }
        };
        let view = output
            .texture
            .create_view(&TextureViewDescriptor::default());

        // Update viewport resolution (idempotent if unchanged)
        s.viewport.update(
            &s.queue,
            Resolution {
                width: s.surface_config.width,
                height: s.surface_config.height,
            },
        );

        // Prepare: shape + upload glyph vertices
        let text_area = TextArea {
            buffer: &s.buffer,
            left: 0.0,
            top: 0.0,
            scale: 1.0,
            bounds: TextBounds {
                left: 0,
                top: 0,
                right: s.surface_config.width as i32,
                bottom: s.surface_config.height as i32,
            },
            default_color: Color::rgb(0xFF, 0xFF, 0xFF),
            custom_glyphs: &[],
        };

        s.text_renderer
            .prepare(
                &s.device,
                &s.queue,
                &mut s.font_system,
                &mut s.atlas,
                &s.viewport,
                [text_area],
                &mut s.swash_cache,
            )
            .expect("glyphon prepare");

        // Encoder + render pass
        let mut encoder =
            s.device
                .create_command_encoder(&CommandEncoderDescriptor {
                    label: Some("s1-encoder"),
                });
        {
            let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("s1-render-pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(wgpu::Color {
                            r: 0.02,
                            g: 0.02,
                            b: 0.02,
                            a: 1.0,
                        }),
                        store: StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            s.text_renderer
                .render(&s.atlas, &s.viewport, &mut pass)
                .expect("glyphon render");
        }

        // Submit — t2 is captured after this returns
        s.queue.submit(std::iter::once(encoder.finish()));

        // t2: after submit (CPU side; GPU is still working asynchronously)
        let t2 = Instant::now();

        output.present();

        // Trim the atlas periodically to avoid unbounded growth
        s.atlas.trim();

        let elapsed_us = t2.duration_since(t1).as_micros() as u64;
        self.samples.push(elapsed_us);

        let n = self.samples.len();
        if n % 20 == 0 {
            info!("Sample {}/{}: {} µs", n, MAX_SAMPLES, elapsed_us);
        }

        if n >= MAX_SAMPLES {
            self.print_stats();
            event_loop.exit();
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.state.is_some() {
            return; // already initialised (e.g., second resume on mobile — guard for safety)
        }
        match self.init_gpu(event_loop) {
            Ok(()) => info!("GPU state initialised"),
            Err(e) => {
                eprintln!("Fatal: {e:#}");
                event_loop.exit();
            }
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        state: ElementState::Pressed,
                        ..
                    },
                ..
            } => {
                self.on_key_pressed(event_loop);
            }
            WindowEvent::CloseRequested => {
                self.print_stats();
                event_loop.exit();
            }
            WindowEvent::Resized(new_size) => {
                if let Some(s) = self.state.as_mut() {
                    s.surface_config.width = new_size.width.max(1);
                    s.surface_config.height = new_size.height.max(1);
                    s.surface.configure(&s.device, &s.surface_config);
                }
            }
            _ => {}
        }
    }

    fn exiting(&mut self, _event_loop: &ActiveEventLoop) {
        // Print stats if not already printed (e.g., OS kill without CloseRequested)
        if !self.samples.is_empty() {
            self.print_stats();
        }
    }
}

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("s1_renderer_latency=info".parse().unwrap()),
        )
        .init();

    let event_loop = EventLoop::new().context("create event loop")?;
    event_loop.set_control_flow(ControlFlow::Wait);

    let mut app = App::default();
    event_loop.run_app(&mut app).context("run event loop")?;

    Ok(())
}
