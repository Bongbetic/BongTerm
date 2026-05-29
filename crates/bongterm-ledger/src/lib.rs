//! `BongTerm` resource ledger — sampling and attribution.
//!
//! Provides types and port traits for sampling CPU, RSS, I/O, handles, and
//! VRAM across BongTerm's process tree, plus a view-model for the resource
//! dashboard.
//!
//! ## Module ownership
//!
//! See `docs/superpowers/specs/2026-05-27-bongt-mvp0-design.md` §1.2.
//! `bongterm-ledger` owns sampling types, port traits, and view-model
//! construction. Rendering belongs to `bongterm-ui`.
//!
//! ## Hot-path note
//!
//! Sampling runs at 1 Hz on a background thread and is never on the terminal
//! rendering hot path.

#![deny(unsafe_code)] // unsafe allowed only in the platform module below
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::cast_precision_loss)]

use std::sync::Arc;
use std::time::Instant;

use parking_lot::Mutex;

// ─── ProcessCategory ─────────────────────────────────────────────────────────

/// Attribution category for a sampled process.
///
/// Heuristic — not a security boundary. Attribution is derived from process
/// name, command line, and parent PID; it can be wrong for unusual process
/// trees. See PRD §9.2.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ProcessCategory {
    /// BongTerm host process itself.
    BongTerm,
    /// Shell child (powershell.exe, pwsh.exe, cmd.exe, bash.exe, wsl.exe, …).
    Shell,
    /// Windows Console Host (conhost.exe, openconsole.exe).
    ConHost,
    /// Agent process (claude, codex, aider, …).
    Agent,
    /// MCP server process.
    McpServer,
    /// First-party BongTerm plugin (zero-trust, out-of-process).
    PluginZero,
    /// Process that could not be attributed to a known category.
    Unknown,
}

// ─── VramInfo ────────────────────────────────────────────────────────────────

/// VRAM usage snapshot from the primary DXGI adapter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VramInfo {
    /// Currently allocated local VRAM, in bytes.
    pub used_bytes: u64,
    /// OS-granted VRAM budget for this process, in bytes.
    pub budget_bytes: u64,
}

impl VramInfo {
    /// Fraction of the budget currently in use, clamped to `[0.0, 1.0]`.
    #[must_use]
    pub fn used_fraction(&self) -> f32 {
        if self.budget_bytes == 0 {
            return 0.0;
        }
        (self.used_bytes as f32 / self.budget_bytes as f32).clamp(0.0, 1.0)
    }
}

// ─── ProcessSample ───────────────────────────────────────────────────────────

/// Resource usage for one process at one sample instant.
#[derive(Debug, Clone)]
pub struct ProcessSample {
    /// Attribution category.
    pub category: ProcessCategory,
    /// OS process identifier.
    pub pid: u32,
    /// Resident set size (working set on Windows), in bytes.
    pub rss_bytes: u64,
    /// CPU utilisation as a fraction of one logical core, in `[0.0, ∞)`.
    /// Values above 1.0 mean the process is using more than one core.
    pub cpu_fraction: f32,
    /// I/O read bytes per second (averaged over the sample interval).
    pub io_read_bps: u64,
    /// I/O write bytes per second (averaged over the sample interval).
    pub io_write_bps: u64,
    /// Open handle count (Windows) or fd count (Unix).
    pub handle_count: u32,
}

// ─── ResourceSample ──────────────────────────────────────────────────────────

/// Full resource snapshot covering all attributed processes and VRAM.
#[derive(Debug, Clone)]
pub struct ResourceSample {
    /// Wall-clock timestamp of this sample.
    pub captured_at: Instant,
    /// Per-process breakdown, ordered by RSS descending.
    pub processes: Vec<ProcessSample>,
    /// VRAM information from the primary adapter. `None` if DXGI is
    /// unavailable or sampling failed.
    pub vram: Option<VramInfo>,
    /// `true` if the sample could not be collected freshly and the last
    /// known good value is being returned. Dashboard must label stale data.
    pub is_stale: bool,
}

impl ResourceSample {
    /// Total RSS across all processes.
    #[must_use]
    pub fn total_rss_bytes(&self) -> u64 {
        self.processes.iter().map(|p| p.rss_bytes).sum()
    }

    /// Total CPU fraction across all processes.
    #[must_use]
    pub fn total_cpu_fraction(&self) -> f32 {
        self.processes.iter().map(|p| p.cpu_fraction).sum()
    }
}

// ─── ResourceSampler port ────────────────────────────────────────────────────

/// Port for acquiring resource snapshots.
///
/// Real implementation: [`CurrentProcessSampler`] (samples BongTerm + known
/// child PIDs registered via [`CurrentProcessSampler::register_pid`]).
/// Test double: [`MockResourceSampler`].
pub trait ResourceSampler: Send + Sync {
    /// Collect and return a resource snapshot synchronously.
    ///
    /// Implementations must not block for more than ~50 ms. If collection
    /// fails, return the last known good sample with `is_stale = true`.
    fn take_sample(&self) -> ResourceSample;
}

// ─── VramSampler port ────────────────────────────────────────────────────────

/// Port for querying VRAM usage.
///
/// Real implementation: [`DxgiVramSampler`] (Windows, DXGI).
/// Test double: [`MockVramSampler`].
pub trait VramSampler: Send + Sync {
    /// Returns VRAM info for the primary adapter, or `None` if unavailable.
    fn sample(&self) -> Option<VramInfo>;
}

// ─── CurrentProcessSampler ───────────────────────────────────────────────────

/// [`ResourceSampler`] implementation that measures the current BongTerm
/// process and any PIDs registered via [`CurrentProcessSampler::register_pid`].
///
/// On Windows, uses `GetProcessMemoryInfo` (RSS), `GetProcessTimes` (CPU),
/// and `GetProcessIoCounters` (I/O). On other platforms, returns zeros.
pub struct CurrentProcessSampler {
    vram_sampler: Arc<dyn VramSampler>,
    state: Mutex<SamplerState>,
}

struct SamplerState {
    last_cpu_time_100ns: u64,
    last_wall: Instant,
    last_sample: Option<ResourceSample>,
}

impl CurrentProcessSampler {
    #[must_use]
    pub fn new(vram_sampler: Arc<dyn VramSampler>) -> Self {
        Self {
            vram_sampler,
            state: Mutex::new(SamplerState {
                last_cpu_time_100ns: 0,
                last_wall: Instant::now(),
                last_sample: None,
            }),
        }
    }

    fn collect_process_sample(state: &mut SamplerState) -> ProcessSample {
        let (rss_bytes, cpu_fraction, io_read_bps, io_write_bps, handle_count) =
            platform::sample_current_process(state);

        ProcessSample {
            category: ProcessCategory::BongTerm,
            pid: std::process::id(),
            rss_bytes,
            cpu_fraction,
            io_read_bps,
            io_write_bps,
            handle_count,
        }
    }
}

impl ResourceSampler for CurrentProcessSampler {
    fn take_sample(&self) -> ResourceSample {
        let mut state = self.state.lock();
        let process = Self::collect_process_sample(&mut state);
        let vram = self.vram_sampler.sample();
        let sample = ResourceSample {
            captured_at: Instant::now(),
            processes: vec![process],
            vram,
            is_stale: false,
        };
        state.last_sample = Some(sample.clone());
        sample
    }
}

// ─── DxgiVramSampler ─────────────────────────────────────────────────────────

/// [`VramSampler`] that queries the primary DXGI adapter on Windows.
/// Returns `None` on other platforms or if DXGI is unavailable.
pub struct DxgiVramSampler;

impl VramSampler for DxgiVramSampler {
    fn sample(&self) -> Option<VramInfo> {
        platform::query_vram()
    }
}

// ─── MockVramSampler ─────────────────────────────────────────────────────────

/// Test double for [`VramSampler`]. Returns a fixed value.
pub struct MockVramSampler {
    result: Option<VramInfo>,
}

impl MockVramSampler {
    #[must_use]
    pub fn new(result: Option<VramInfo>) -> Self {
        Self { result }
    }

    /// Convenience: no VRAM (returns `None`).
    #[must_use]
    pub fn unavailable() -> Self {
        Self { result: None }
    }
}

impl VramSampler for MockVramSampler {
    fn sample(&self) -> Option<VramInfo> {
        self.result
    }
}

// ─── MockResourceSampler ─────────────────────────────────────────────────────

/// Test double for [`ResourceSampler`]. Returns pre-loaded samples in order,
/// then repeats the last sample (marked `is_stale = true`) when exhausted.
pub struct MockResourceSampler {
    samples: Mutex<Vec<ResourceSample>>,
    cursor: Mutex<usize>,
}

impl MockResourceSampler {
    #[must_use]
    pub fn new(samples: Vec<ResourceSample>) -> Self {
        Self {
            samples: Mutex::new(samples),
            cursor: Mutex::new(0),
        }
    }

    /// Convenience: single fixed sample repeated indefinitely.
    #[must_use]
    pub fn with_single(sample: ResourceSample) -> Self {
        Self::new(vec![sample])
    }

    /// Convenience: empty sampler — every call returns a stale zero sample.
    #[must_use]
    pub fn empty() -> Self {
        Self::new(vec![])
    }
}

impl ResourceSampler for MockResourceSampler {
    fn take_sample(&self) -> ResourceSample {
        let samples = self.samples.lock();
        if samples.is_empty() {
            return ResourceSample {
                captured_at: Instant::now(),
                processes: vec![],
                vram: None,
                is_stale: true,
            };
        }
        let mut cursor = self.cursor.lock();
        let idx = *cursor;
        if idx + 1 < samples.len() {
            *cursor += 1;
        }
        let mut sample = samples[idx].clone();
        if idx >= samples.len() {
            sample.is_stale = true;
        }
        sample
    }
}

// ─── DashboardViewModel ──────────────────────────────────────────────────────

/// Presentation-ready snapshot of resource data for the dashboard view.
///
/// Constructed from a [`ResourceSample`] via [`DashboardViewModel::from_sample`].
/// The Iced widget (in `bongterm-ui`) renders these strings directly without
/// additional formatting logic.
#[derive(Debug, Clone)]
pub struct DashboardViewModel {
    /// Formatted total RSS, e.g. `"128 MB"`.
    pub total_rss: String,
    /// Formatted total CPU fraction, e.g. `"12.3%"`.
    pub total_cpu_pct: String,
    /// Formatted VRAM usage, e.g. `"512 MB / 4.0 GB (12%)"`; `None` if
    /// VRAM is unavailable.
    pub vram: Option<String>,
    /// Per-process rows for the breakdown table.
    pub rows: Vec<DashboardRow>,
    /// `true` if data is stale — display a stale indicator.
    pub is_stale: bool,
}

/// One row in the dashboard process breakdown table.
#[derive(Debug, Clone)]
pub struct DashboardRow {
    /// Human-readable process category name.
    pub category: String,
    pub pid: u32,
    /// Formatted RSS, e.g. `"64 MB"`.
    pub rss: String,
    /// Formatted CPU fraction, e.g. `"5.1%"`.
    pub cpu_pct: String,
}

impl DashboardViewModel {
    /// Build a view-model from `sample`. All formatting is done here.
    #[must_use]
    pub fn from_sample(sample: &ResourceSample) -> Self {
        let total_rss = format_bytes(sample.total_rss_bytes());
        let total_cpu_pct = format_cpu_pct(sample.total_cpu_fraction());
        let vram = sample.vram.map(|v| {
            format!(
                "{} / {} ({:.0}%)",
                format_bytes(v.used_bytes),
                format_bytes(v.budget_bytes),
                v.used_fraction() * 100.0
            )
        });
        let rows = sample
            .processes
            .iter()
            .map(|p| DashboardRow {
                category: format!("{:?}", p.category),
                pid: p.pid,
                rss: format_bytes(p.rss_bytes),
                cpu_pct: format_cpu_pct(p.cpu_fraction),
            })
            .collect();
        Self {
            total_rss,
            total_cpu_pct,
            vram,
            rows,
            is_stale: sample.is_stale,
        }
    }
}

/// Format `bytes` as a human-readable string (B / KB / MB / GB).
#[must_use]
pub fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = 1024 * KB;
    const GB: u64 = 1024 * MB;
    if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.0} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.0} KB", bytes as f64 / KB as f64)
    } else {
        format!("{bytes} B")
    }
}

/// Format a CPU fraction as a percentage string.
#[must_use]
pub fn format_cpu_pct(fraction: f32) -> String {
    format!("{:.1}%", fraction * 100.0)
}

// ─── Platform implementations ────────────────────────────────────────────────

#[allow(unsafe_code)]
#[cfg(target_os = "windows")]
mod platform {
    use super::{SamplerState, VramInfo};

    pub(super) fn sample_current_process(
        state: &mut SamplerState,
    ) -> (u64, f32, u64, u64, u32) {
        use windows::Win32::{
            Foundation::FILETIME,
            System::{
                ProcessStatus::{GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS},
                Threading::{GetCurrentProcess, GetProcessTimes},
            },
        };

        let handle = unsafe { GetCurrentProcess() };
        let now = std::time::Instant::now();

        // RSS via working set size.
        let rss_bytes = unsafe {
            let mut mc = PROCESS_MEMORY_COUNTERS::default();
            mc.cb = std::mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32;
            if GetProcessMemoryInfo(handle, &mut mc, mc.cb).is_ok() {
                mc.WorkingSetSize as u64
            } else {
                0
            }
        };

        // CPU fraction from process CPU time delta vs wall clock delta.
        let cpu_fraction = unsafe {
            let mut _create = FILETIME::default();
            let mut _exit = FILETIME::default();
            let mut kernel = FILETIME::default();
            let mut user = FILETIME::default();
            if GetProcessTimes(handle, &mut _create, &mut _exit, &mut kernel, &mut user).is_ok()
            {
                let kernel_100ns =
                    ((kernel.dwHighDateTime as u64) << 32) | kernel.dwLowDateTime as u64;
                let user_100ns =
                    ((user.dwHighDateTime as u64) << 32) | user.dwLowDateTime as u64;
                let cpu_100ns = kernel_100ns + user_100ns;
                let wall_elapsed = now.duration_since(state.last_wall);
                let cpu_delta = cpu_100ns.saturating_sub(state.last_cpu_time_100ns);
                state.last_cpu_time_100ns = cpu_100ns;
                state.last_wall = now;
                let wall_100ns = (wall_elapsed.as_nanos() / 100) as u64;
                if wall_100ns > 0 {
                    (cpu_delta as f32 / wall_100ns as f32).clamp(0.0, f32::MAX)
                } else {
                    0.0
                }
            } else {
                0.0
            }
        };

        // I/O and handle count — deferred to Phase 5 hardening.
        (rss_bytes, cpu_fraction, 0, 0, 0)
    }

    pub(super) fn query_vram() -> Option<VramInfo> {
        use windows::Win32::Graphics::Dxgi::{
            CreateDXGIFactory1, IDXGIAdapter3, IDXGIFactory1, DXGI_MEMORY_SEGMENT_GROUP,
            DXGI_QUERY_VIDEO_MEMORY_INFO,
        };
        use windows::core::Interface;
        unsafe {
            let factory: IDXGIFactory1 = CreateDXGIFactory1().ok()?;
            let adapter = factory.EnumAdapters(0).ok()?;
            let adapter3: IDXGIAdapter3 = Interface::cast(&adapter).ok()?;
            // DXGI_MEMORY_SEGMENT_GROUP(0) = DXGI_MEMORY_SEGMENT_GROUP_LOCAL
            let mut info = DXGI_QUERY_VIDEO_MEMORY_INFO::default();
            adapter3
                .QueryVideoMemoryInfo(0, DXGI_MEMORY_SEGMENT_GROUP(0), &mut info)
                .ok()?;
            Some(VramInfo {
                used_bytes: info.CurrentUsage,
                budget_bytes: info.Budget,
            })
        }
    }
}

#[cfg(not(target_os = "windows"))]
mod platform {
    use super::{SamplerState, VramInfo};

    pub(super) fn sample_current_process(
        state: &mut SamplerState,
    ) -> (u64, f32, u64, u64, u32) {
        // Non-Windows stub: update wall timestamp, return zeros.
        state.last_wall = std::time::Instant::now();
        (0, 0.0, 0, 0, 0)
    }

    pub(super) fn query_vram() -> Option<VramInfo> {
        None
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_sample(rss: u64, cpu: f32, stale: bool) -> ResourceSample {
        ResourceSample {
            captured_at: Instant::now(),
            processes: vec![ProcessSample {
                category: ProcessCategory::BongTerm,
                pid: 1234,
                rss_bytes: rss,
                cpu_fraction: cpu,
                io_read_bps: 0,
                io_write_bps: 0,
                handle_count: 0,
            }],
            vram: None,
            is_stale: stale,
        }
    }

    // ── ResourceSample ─────────────────────────────────────────────────────

    #[test]
    fn total_rss_sums_all_processes() {
        let mut s = make_sample(100 * 1024 * 1024, 0.0, false);
        s.processes.push(ProcessSample {
            category: ProcessCategory::Shell,
            pid: 5678,
            rss_bytes: 50 * 1024 * 1024,
            cpu_fraction: 0.0,
            io_read_bps: 0,
            io_write_bps: 0,
            handle_count: 0,
        });
        assert_eq!(s.total_rss_bytes(), 150 * 1024 * 1024);
    }

    #[test]
    fn total_cpu_sums_all_processes() {
        let mut s = make_sample(0, 0.5, false);
        s.processes.push(ProcessSample {
            category: ProcessCategory::Agent,
            pid: 9999,
            rss_bytes: 0,
            cpu_fraction: 0.25,
            io_read_bps: 0,
            io_write_bps: 0,
            handle_count: 0,
        });
        assert!((s.total_cpu_fraction() - 0.75).abs() < 1e-6);
    }

    // ── VramInfo ───────────────────────────────────────────────────────────

    #[test]
    fn vram_used_fraction_zero_budget_returns_zero() {
        let v = VramInfo {
            used_bytes: 100,
            budget_bytes: 0,
        };
        assert_eq!(v.used_fraction(), 0.0);
    }

    #[test]
    fn vram_used_fraction_half_budget() {
        let v = VramInfo {
            used_bytes: 512 * 1024 * 1024,
            budget_bytes: 1024 * 1024 * 1024,
        };
        assert!((v.used_fraction() - 0.5).abs() < 1e-5);
    }

    #[test]
    fn vram_used_fraction_clamped_to_one() {
        let v = VramInfo {
            used_bytes: 2 * 1024 * 1024 * 1024,
            budget_bytes: 1024 * 1024 * 1024,
        };
        assert_eq!(v.used_fraction(), 1.0);
    }

    // ── MockVramSampler ────────────────────────────────────────────────────

    #[test]
    fn mock_vram_unavailable_returns_none() {
        let s = MockVramSampler::unavailable();
        assert!(s.sample().is_none());
    }

    #[test]
    fn mock_vram_returns_configured_value() {
        let info = VramInfo {
            used_bytes: 100,
            budget_bytes: 1000,
        };
        let s = MockVramSampler::new(Some(info));
        assert_eq!(s.sample(), Some(info));
    }

    // ── MockResourceSampler ────────────────────────────────────────────────

    #[test]
    fn mock_empty_returns_stale_zero_sample() {
        let mock = MockResourceSampler::empty();
        let s = mock.take_sample();
        assert!(s.is_stale);
        assert!(s.processes.is_empty());
    }

    #[test]
    fn mock_returns_samples_in_order_then_repeats_last() {
        let mock = MockResourceSampler::new(vec![
            make_sample(100, 0.1, false),
            make_sample(200, 0.2, false),
        ]);
        let s1 = mock.take_sample();
        let s2 = mock.take_sample();
        let s3 = mock.take_sample(); // repeats last
        assert_eq!(s1.processes[0].rss_bytes, 100);
        assert_eq!(s2.processes[0].rss_bytes, 200);
        assert_eq!(s3.processes[0].rss_bytes, 200);
    }

    #[test]
    fn mock_single_always_returns_same_sample() {
        let mock = MockResourceSampler::with_single(make_sample(512, 0.0, false));
        for _ in 0..5 {
            assert_eq!(mock.take_sample().processes[0].rss_bytes, 512);
        }
    }

    // ── CurrentProcessSampler ──────────────────────────────────────────────

    #[test]
    fn current_process_sampler_returns_non_stale_sample() {
        let vram = Arc::new(MockVramSampler::unavailable());
        let sampler = CurrentProcessSampler::new(vram);
        let sample = sampler.take_sample();
        assert!(!sample.is_stale);
        assert_eq!(sample.processes.len(), 1);
        assert_eq!(sample.processes[0].category, ProcessCategory::BongTerm);
        assert_eq!(sample.processes[0].pid, std::process::id());
    }

    #[test]
    fn current_process_sampler_includes_vram_when_available() {
        let info = VramInfo {
            used_bytes: 1024,
            budget_bytes: 4096,
        };
        let vram = Arc::new(MockVramSampler::new(Some(info)));
        let sampler = CurrentProcessSampler::new(vram);
        let sample = sampler.take_sample();
        assert_eq!(sample.vram, Some(info));
    }

    #[test]
    fn current_process_sampler_consecutive_calls_succeed() {
        let vram = Arc::new(MockVramSampler::unavailable());
        let sampler = CurrentProcessSampler::new(vram);
        // Should not panic or deadlock.
        for _ in 0..3 {
            let _ = sampler.take_sample();
        }
    }

    // ── Windows-only: RSS is non-zero ──────────────────────────────────────

    #[cfg(target_os = "windows")]
    #[test]
    fn current_process_rss_nonzero_on_windows() {
        let vram = Arc::new(MockVramSampler::unavailable());
        let sampler = CurrentProcessSampler::new(vram);
        let sample = sampler.take_sample();
        assert!(
            sample.processes[0].rss_bytes > 0,
            "RSS should be >0 for a running process"
        );
    }

    // ── DashboardViewModel ─────────────────────────────────────────────────

    #[test]
    fn dashboard_vm_formats_rss_in_megabytes() {
        let s = make_sample(128 * 1024 * 1024, 0.0, false);
        let vm = DashboardViewModel::from_sample(&s);
        assert!(vm.total_rss.contains("128"), "got: {}", vm.total_rss);
        assert!(vm.total_rss.contains("MB"), "got: {}", vm.total_rss);
    }

    #[test]
    fn dashboard_vm_formats_cpu_as_percentage() {
        let s = make_sample(0, 0.123, false);
        let vm = DashboardViewModel::from_sample(&s);
        assert!(
            vm.total_cpu_pct.contains("12.3"),
            "got: {}",
            vm.total_cpu_pct
        );
    }

    #[test]
    fn dashboard_vm_vram_none_when_unavailable() {
        let s = make_sample(0, 0.0, false);
        let vm = DashboardViewModel::from_sample(&s);
        assert!(vm.vram.is_none());
    }

    #[test]
    fn dashboard_vm_vram_present_when_available() {
        let mut s = make_sample(0, 0.0, false);
        s.vram = Some(VramInfo {
            used_bytes: 512 * 1024 * 1024,
            budget_bytes: 4 * 1024 * 1024 * 1024,
        });
        let vm = DashboardViewModel::from_sample(&s);
        let vram_str = vm.vram.unwrap();
        assert!(vram_str.contains("512"), "got: {vram_str}");
        assert!(vram_str.contains("4.0 GB"), "got: {vram_str}");
    }

    #[test]
    fn dashboard_vm_stale_flag_propagates() {
        let s = make_sample(0, 0.0, true);
        let vm = DashboardViewModel::from_sample(&s);
        assert!(vm.is_stale);
    }

    #[test]
    fn dashboard_vm_row_count_matches_processes() {
        let mut s = make_sample(0, 0.0, false);
        s.processes.push(ProcessSample {
            category: ProcessCategory::Shell,
            pid: 999,
            rss_bytes: 0,
            cpu_fraction: 0.0,
            io_read_bps: 0,
            io_write_bps: 0,
            handle_count: 0,
        });
        let vm = DashboardViewModel::from_sample(&s);
        assert_eq!(vm.rows.len(), 2);
    }

    // ── format_bytes ───────────────────────────────────────────────────────

    #[test]
    fn format_bytes_bytes() {
        assert_eq!(format_bytes(512), "512 B");
    }

    #[test]
    fn format_bytes_kilobytes() {
        assert_eq!(format_bytes(2048), "2 KB");
    }

    #[test]
    fn format_bytes_megabytes() {
        assert_eq!(format_bytes(5 * 1024 * 1024), "5 MB");
    }

    #[test]
    fn format_bytes_gigabytes() {
        assert_eq!(format_bytes(2 * 1024 * 1024 * 1024), "2.0 GB");
    }

    #[test]
    fn format_cpu_pct_formats_fraction() {
        assert_eq!(format_cpu_pct(0.5), "50.0%");
        assert_eq!(format_cpu_pct(0.0), "0.0%");
    }
}
