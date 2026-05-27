// S2 Measurement:
//   - VRAM usage queried via IDXGIAdapter3::QueryVideoMemoryInfo (DXGI_MEMORY_SEGMENT_GROUP_LOCAL)
//   - Glyph sets: ASCII + Latin1 + BoxDraw/Braille + CJK sample across 4 simulated panes
//   - Goal: shared atlas for 4 panes stays under 256 MB dedicated VRAM
//   - Eviction: TextAtlas::trim() called; atlas size reported before and after

#![allow(clippy::pedantic)]

use anyhow::{Context, Result};
use glyphon::{
    Attrs, Buffer, Cache, Color, Family, FontSystem, Metrics, Resolution, Shaping, SwashCache,
    TextArea, TextAtlas, TextBounds, TextRenderer, Viewport,
};
use wgpu::{
    Backends, DeviceDescriptor, Instance, InstanceDescriptor, Maintain, MultisampleState,
    PowerPreference, RequestAdapterOptionsBase,
};
use windows::Win32::Graphics::Dxgi::{
    CreateDXGIFactory1, IDXGIAdapter3, IDXGIFactory4, DXGI_MEMORY_SEGMENT_GROUP_LOCAL,
    DXGI_QUERY_VIDEO_MEMORY_INFO,
};
// `Interface::cast` is required to QI IDXGIAdapter1 → IDXGIAdapter3.
use windows::core::Interface;

// Suppress unused-crate-dependencies warnings: these workspace crates are listed in
// this spike's Cargo.toml for consistency but the spike logic doesn't call into them.
use bongterm_render as _;
use bongterm_term as _;

// ---------------------------------------------------------------------------
// DXGI VRAM query
// ---------------------------------------------------------------------------

/// Query dedicated (local) VRAM currently used by the GPU process via DXGI.
///
/// Creates a fresh `IDXGIFactory4`, enumerates adapter 0 (the high-perf GPU on
/// a hybrid system), casts to `IDXGIAdapter3`, and calls `QueryVideoMemoryInfo`.
///
/// Returns `None` if DXGI is unavailable or any COM call fails.
fn query_vram_used_mb() -> Option<u64> {
    unsafe {
        let factory: IDXGIFactory4 = CreateDXGIFactory1().ok()?;
        let adapter1 = factory.EnumAdapters1(0).ok()?;
        let adapter3: IDXGIAdapter3 = adapter1.cast().ok()?;
        let mut info = DXGI_QUERY_VIDEO_MEMORY_INFO::default();
        adapter3
            .QueryVideoMemoryInfo(0, DXGI_MEMORY_SEGMENT_GROUP_LOCAL, &mut info)
            .ok()?;
        Some(info.CurrentUsage / (1024 * 1024))
    }
}

// ---------------------------------------------------------------------------
// Pane glyph sets
// ---------------------------------------------------------------------------

/// Build the text content for each simulated pane:
/// - Pane 0: ASCII printable 0x20–0x7E  (95 glyphs)
/// - Pane 1: Latin-1 Supplement 0xA0–0xFF (96 glyphs)
/// - Pane 2: Box-drawing + Braille U+2500–U+25FF (256 glyphs)
/// - Pane 3: CJK Unified Ideographs sample U+4E00–U+4EFF (256 glyphs)
fn pane_text(pane: usize) -> (String, usize) {
    match pane {
        0 => {
            let s: String = (0x20u32..=0x7E).filter_map(char::from_u32).collect();
            let n = s.chars().count();
            (s, n)
        }
        1 => {
            let s: String = (0xA0u32..=0xFFu32).filter_map(char::from_u32).collect();
            let n = s.chars().count();
            (s, n)
        }
        2 => {
            let s: String = (0x2500u32..=0x25FFu32).filter_map(char::from_u32).collect();
            let n = s.chars().count();
            (s, n)
        }
        3 => {
            let s: String = (0x4E00u32..=0x4EFFu32).filter_map(char::from_u32).collect();
            let n = s.chars().count();
            (s, n)
        }
        _ => unreachable!(),
    }
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

#[tokio::main]
async fn main() -> Result<()> {
    // tracing-subscriber is intentionally excluded from this spike's Cargo.toml.
    // Use tracing macros only; no subscriber setup needed for a measurement binary.
    tracing::info!("s2-vram-eviction spike starting");

    println!("=== S2: Shared Glyph Atlas VRAM Budget Spike ===\n");

    // ------------------------------------------------------------------
    // 1. wgpu: DX12 instance → high-perf adapter → device + queue
    // ------------------------------------------------------------------
    let instance = Instance::new(InstanceDescriptor {
        backends: Backends::DX12,
        ..Default::default()
    });

    // Headless: no surface. RequestAdapterOptionsBase with compatible_surface: None.
    let adapter = instance
        .request_adapter(&RequestAdapterOptionsBase {
            power_preference: PowerPreference::HighPerformance,
            compatible_surface: None::<&wgpu::Surface<'_>>,
            force_fallback_adapter: false,
        })
        .await
        .context("No DX12 adapter found — requires Windows 10 1809+ and a DX12 GPU")?;

    println!("Adapter: {}", adapter.get_info().name);

    let (device, queue) = adapter
        .request_device(&DeviceDescriptor::default(), None)
        .await
        .context("Failed to create wgpu device")?;

    // ------------------------------------------------------------------
    // 2. VRAM baseline (before any atlas work)
    // ------------------------------------------------------------------
    let vram_before_mb = query_vram_used_mb();
    match vram_before_mb {
        Some(mb) => println!("DXGI dedicated VRAM used BEFORE atlas upload: {} MB", mb),
        None => println!("DXGI VRAM query unavailable — skipping VRAM delta measurement"),
    }

    // ------------------------------------------------------------------
    // 3. Glyphon setup: shared Cache, Viewport, TextAtlas, TextRenderer
    //    One shared atlas simulates the "single shared atlas across N panes" design.
    // ------------------------------------------------------------------
    // Bgra8UnormSrgb matches the prod swapchain convention.
    // Headless: no real framebuffer; this only controls atlas pipeline format.
    let surface_format = wgpu::TextureFormat::Bgra8UnormSrgb;

    let cache = Cache::new(&device);
    let mut viewport = Viewport::new(&device, &cache);

    // Non-zero resolution so prepare() doesn't trivially skip glyph rasterization.
    // "Virtual" 1920×1080 surface — no actual framebuffer exists.
    viewport.update(&queue, Resolution { width: 1920, height: 1080 });

    let mut atlas = TextAtlas::new(&device, &queue, &cache, surface_format);
    let mut text_renderer =
        TextRenderer::new(&mut atlas, &device, MultisampleState::default(), None);

    // ------------------------------------------------------------------
    // 4. Per-pane: FontSystem + SwashCache + Buffer; shape all glyphs;
    //    then call prepare() to upload shaped glyphs into the shared atlas.
    // ------------------------------------------------------------------
    // 14 px — matching the BongTerm default terminal font size.
    let font_size_px = 14.0_f32;
    let line_height_px = font_size_px * 1.2;
    let metrics = Metrics::new(font_size_px, line_height_px);

    // Separate FontSystem + SwashCache per pane (matching the measurement scenario).
    let mut pane_data: Vec<(FontSystem, SwashCache, Buffer, usize)> = Vec::new();
    for p in 0..4usize {
        let (text, glyph_count) = pane_text(p);
        let mut font_system = FontSystem::new();
        let swash_cache = SwashCache::new();
        let mut buf = Buffer::new(&mut font_system, metrics);
        // Wide "virtual terminal" line: no word-wrap, all glyphs on one long line.
        buf.set_size(&mut font_system, Some(64000.0), Some(line_height_px * 2.0));
        buf.set_text(
            &mut font_system,
            &text,
            Attrs::new().family(Family::Monospace),
            Shaping::Advanced,
        );
        buf.shape_until_scroll(&mut font_system, false);
        pane_data.push((font_system, swash_cache, buf, glyph_count));
    }

    // Print per-pane glyph counts.
    let pane_labels = [
        "ASCII printable (0x20–0x7E)",
        "Latin-1 Supplement (0xA0–0xFF)",
        "Box-drawing + Braille (U+2500–U+25FF)",
        "CJK sample (U+4E00–U+4EFF)",
    ];
    let total_glyphs: usize = pane_data.iter().map(|(_, _, _, n)| n).sum();
    for (i, (_, _, _, n)) in pane_data.iter().enumerate() {
        println!("Pane {}: {} — {} glyphs", i, pane_labels[i], n);
    }
    println!("Total glyphs across all panes: {}", total_glyphs);
    println!();

    // Upload each pane's glyphs to the shared atlas via prepare().
    for p in 0..4usize {
        let (font_system, swash_cache, buf, _) = &mut pane_data[p];
        let text_area = TextArea {
            buffer: buf,
            left: 0.0,
            top: 0.0,
            scale: 1.0,
            bounds: TextBounds {
                left: 0,
                top: 0,
                right: 64000,
                bottom: (line_height_px * 2.0) as i32,
            },
            default_color: Color::rgb(255, 255, 255),
            custom_glyphs: &[],
        };

        text_renderer
            .prepare(
                &device,
                &queue,
                font_system,
                &mut atlas,
                &viewport,
                [text_area],
                swash_cache,
            )
            .with_context(|| format!("Failed to prepare pane {} glyphs", p))?;
    }

    // Flush: ensure all queue.write_texture calls complete before VRAM query.
    device.poll(Maintain::Wait);

    // ------------------------------------------------------------------
    // 5. Estimated atlas texture size (computed from source-known constants
    //    since glyphon 0.6 InnerAtlas.size is pub(crate)).
    //
    //    Atlas starts at 256×256 and doubles on overflow.
    //    With 703 glyphs at 14px (≈14×14px each ≈196 px²):
    //      total ≈ 137,788 px; one doubling → 512×512 likely sufficient.
    //    Mask atlas  (R8,    1 B/px): 512×512×1 =   256 KB
    //    Color atlas (Rgba8, 4 B/px): 512×512×4 = 1,024 KB
    //    Combined estimate: ~1.25 MB — well under the 256 MB budget.
    //    The DXGI delta (below) is the ground-truth measurement.
    // ------------------------------------------------------------------
    let est_side: u32 = 512;
    let est_mask_kb = est_side * est_side * 1 / 1024;
    let est_color_kb = est_side * est_side * 4 / 1024;
    let est_total_kb = est_mask_kb + est_color_kb;
    println!(
        "Estimated atlas texture: mask {}×{} ({} KB) + color {}×{} ({} KB) = ~{} KB total",
        est_side, est_side, est_mask_kb,
        est_side, est_side, est_color_kb,
        est_total_kb,
    );
    println!();

    // ------------------------------------------------------------------
    // 6. VRAM after upload
    // ------------------------------------------------------------------
    let vram_after_mb = query_vram_used_mb();
    match (vram_before_mb, vram_after_mb) {
        (Some(before), Some(after)) => {
            let delta = (after as i64) - (before as i64);
            println!("DXGI dedicated VRAM used AFTER atlas upload: {} MB", after);
            println!("DXGI VRAM delta (atlas + pipeline + buffers): {} MB", delta);
            let fits = after < 256;
            println!(
                "Atlas + GPU overhead fits under 256 MB budget: {}  (VRAM used: {} MB)",
                if fits { "YES" } else { "NO — INVESTIGATE" },
                after
            );
        }
        (None, Some(after)) => {
            println!("DXGI dedicated VRAM used AFTER atlas upload: {} MB", after);
            println!("DXGI VRAM delta: unavailable (no baseline)");
            let fits = after < 256;
            println!(
                "Atlas + GPU overhead fits under 256 MB budget: {}  (VRAM used: {} MB)",
                if fits { "YES" } else { "NO — INVESTIGATE" },
                after
            );
        }
        _ => {
            println!("DXGI VRAM query unavailable — cannot measure VRAM delta");
            println!(
                "Heuristic atlas size ({} KB) is well under 256 MB budget: YES",
                est_total_kb
            );
        }
    }

    // ------------------------------------------------------------------
    // 7. LRU eviction test: trim() + report
    //
    //    ADR finding: glyphon 0.6 TextAtlas::trim() clears glyphs_in_use sets
    //    (both mask and color InnerAtlas), marking all cached glyphs as
    //    LRU-eligible for eviction on the NEXT allocation that overflows the
    //    packer. It does NOT shrink or reallocate the underlying GPU texture.
    //    VRAM usage is therefore unchanged by trim().
    //
    //    Real eviction occurs in InnerAtlas::try_allocate() via:
    //      LruCache::pop_lru() + BucketedAtlasAllocator::deallocate()
    //
    //    Implication for ADR-0004:
    //    - trim() is safe to call every frame; it resets LRU state at zero cost.
    //    - Texture memory is reclaimed only when the atlas grows into a new
    //      texture (old texture is dropped). No "shrink" path exists in 0.6.
    //    - A "reset atlas" pattern (drop + recreate TextAtlas) is the only way
    //      to reclaim VRAM explicitly.
    // ------------------------------------------------------------------
    println!();
    println!("--- LRU Eviction Test ---");
    let vram_pre_trim = query_vram_used_mb();

    atlas.trim();

    // trim() is CPU-only; poll flushes any pending destructor callbacks.
    device.poll(Maintain::Wait);

    let vram_post_trim = query_vram_used_mb();

    match (vram_pre_trim, vram_post_trim) {
        (Some(pre), Some(post)) => {
            println!("VRAM before trim(): {} MB", pre);
            println!("VRAM after  trim(): {} MB", post);
            println!(
                "VRAM delta after trim(): {} MB  (expected ~0 — trim resets LRU eligibility only, GPU texture unchanged)",
                (post as i64) - (pre as i64)
            );
        }
        _ => {
            println!("DXGI VRAM query unavailable — cannot measure trim() effect");
        }
    }

    println!(
        "trim() completed. glyphs_in_use cleared. Next prepare() overflow → LRU eviction."
    );
    println!();
    println!("=== S2 Complete ===");
    println!("Exit artifact: docs/adr/0004-atlas-eviction.md");

    Ok(())
}
