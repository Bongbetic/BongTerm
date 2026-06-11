# BongTerm Phase 1 Execution Plan (Usable Terminal)

Date: 2026-05-28
Source: `docs/superpowers/specs/2026-05-27-bongt-mvp0-design.md` (§6.1 gates #1, #4-#8, #17, #28, #29)
Status: Active

## Goal

Deliver a usable terminal slice with real ConPTY + render path, panes/tabs, shell blocks, resource dashboard, and SQLite persistence. Exit only when §6.1 gates #1, #4-#8, #17, #28, #29 are green for 7 consecutive nightly runs.

## Scope Locks

1. Keep architecture boundaries from Phase 0 intact (no cross-domain coupling into hot path).
2. Reuse existing crate ownership and trait contracts; extend implementations, not ownership.
3. Phase 1 includes no new agent/MCP feature work beyond what is required for listed gates.
4. Treat observability, recovery, and resource budgets as first-class Phase 1 requirements, not cleanup work.

## AnythingLLM Engineer Workspace Additions

The `engineer` workspace adds these planning constraints for Phase 1 and future re-plans:

1. Prefer a risk-first tracer slice before breadth: single pane -> ConPTY -> `WezTermAdapter` -> renderer -> minimal persistence/logging.
2. Make contracts explicit at every boundary: PTY input/output, terminal surface snapshots, renderer input, pane lifecycle, OSC block events, SQLite writes.
3. Add measurable budgets to each slice, especially renderer frame pacing, ConPTY throughput/backpressure, SQLite write latency, and dashboard staleness.
4. Keep subsystems substitutable: renderer, PTY host, persistence, and shell integration remain behind BongTerm-owned traits.
5. Add structured logs and failure recovery paths as acceptance criteria for ConPTY, renderer, and SQLite work.
6. Use T-shaped integration: prove one deep end-to-end path before expanding panes/tabs, OSC actions, and dashboards.

## Ordered Work Plan

1. UX Contract (`docs/ux/`) completion:
- `1.UX.1` through `1.UX.10` from `orca.md` must exist before implementation starts.

2. Tracer slice:
- Implement the smallest user-visible terminal path first: app shell, one tab, one pane, default shell spawn, PTY read, `WezTermAdapter::ingest_bytes`, surface snapshot, renderer display, structured logs.
- This slice may use temporary minimal UI affordances only if the architecture contracts are production-shaped and tests cover the contracts.

3. Foundation slice:
- `1.A.1` settings JSON5 + schema + last-known-good fallback.
- `1.A.2` Iced shell main window integration per ADR-005.
- `1.A.3` command palette + keyboard map.
- `1.A.4` first-launch onboarding.

4. Terminal data path slice:
- `1.B.1` real ConPTY child spawn.
- `1.B.2` PTY reader task + ring buffer with backpressure.
- `1.B.3` real `WezTermAdapter::ingest_bytes` wiring (after submodule gitlink fix).
- `1.B.4` backpressure tests (slow renderer and slow transcript).
- `1.B.5` per-pane surface + dirty-region emission.

5. Renderer slice:
- Pre-step: bump workspace `wgpu` pin from `0.20` to `22`.
- `1.C.1` real wgpu device + swap chain per ADR-005.
- `1.C.2` shared glyph atlas with LRU eviction per ADR-004.
- `1.C.3` frame pacing with backpressure.
- `1.C.4` device-loss recovery.
- `1.C.5` VRAM ceiling enforcement.

6. Multiplexing slice:
- `1.D.1` pane/tab model over vendored `wezterm-mux`.
- `1.D.2` split/resize/focus-cycle.
- `1.D.3` layout save/restore.

7. Shell-integration and blocks slice:
- `1.E.1` OSC consumer in `bongterm-blocks`.
- `1.E.2` confidence model (High/Medium/Low/Unsupported).
- `1.E.3` block boundary detection with `tests/fixtures/osc/`.
- `1.E.4` block actions (copy/rerun/attach/save snippet).

8. Resource visibility slice:
- `1.F.1` resource dashboard view.
- `1.F.2` `bongterm-ledger` 1Hz sampler.
- `1.F.3` DXGI VRAM sampling.
- `1.F.4` per-process attribution categories.

9. Persistence slice:
- `1.G.1` SQLite WAL + migration runner + `0001_init.sql`.
- `1.G.2` sidecar chunk writer (blake3, monotonic IDs, retention).
- `1.G.3` startup crash recovery scan.
- `1.G.4` `xtask cleanup-chunks` implementation.

## Test and Gate Strategy

1. Keep tests aligned to risk:
- Unit tests for parser/surface/ledger/storage invariants.
- Integration tests for PTY→term→render path and persistence recovery.
- Conformance tests where contracts already exist.
- Contract tests for PTY, renderer input, pane lifecycle, OSC block events, and SQLite repository behavior.

2. Nightly gate rule:
- All Phase 1 gate checks pass for 7 consecutive nightly runs before phase exit.

3. Acceptance additions:
- ConPTY path has no data loss under sustained synthetic output and reports degraded/backpressured state instead of freezing.
- Renderer displays expected ANSI fixture output and logs frame pacing/device-loss recovery events.
- SQLite WAL/chunk writes are bounded and recover from interrupted write fixtures.
- Dashboard labels stale/degraded measurements rather than presenting stale facts as live facts.

4. Tracking:
- Use `orca.md` as task authority.
- Remove completed tasks in place and advance `[next]` marker one task at a time.

## Immediate Next Action

### Corrective reopen: `1.R.1` runtime shell correction

Context: local gate tests were green, but the running binary regressed to the
temporary one-pane `terminal_app` path and bypasses the PRD shell chrome in
`bongterm-ui`.

Files:
- `crates/bongterm-app/src/lib.rs`
- `crates/bongterm-app/src/main.rs`
- `crates/bongterm-app/src/terminal_app.rs`
- `crates/bongterm-app/src/shell_app.rs`
- `crates/bongterm-app/tests/shell_app.rs`
- `crates/bongterm-ui/src/lib.rs`

RED:
- Add a test proving `bongterm-app` exposes a composed app that boots both the
  `bongterm-ui` shell regions and the live terminal runtime.

GREEN:
- Add a composition-root app in `bongterm-app` that owns `BongTermShell` plus
  the existing `TerminalApp`.
- Make the binary entrypoint run the composed app.
- Add a `bongterm-ui` shell view path that accepts a caller-provided terminal
  surface element instead of rendering the placeholder terminal label.

Acceptance:
- `cargo test -p bongterm-app shell_app`
- `cargo test -p bongterm-ui`
- `cargo run -p bongterm-app` opens a window titled `BongTerm - workspace` with
  shell chrome and a live terminal surface.

### Corrective reopen: `1.R.2` panel data correction

Status: completed on 2026-06-11.

Context: `1.R.1` restored the PRD shell chrome around the live terminal, but
the side panels still render placeholder labels: `Agents collapsed` and
`Resources collapsed`.

Files:
- `crates/bongterm-app/Cargo.toml`
- `crates/bongterm-app/src/shell_app.rs`
- `crates/bongterm-app/tests/shell_app.rs`
- `crates/bongterm-ui/src/lib.rs`

RED:
- Add a test proving `BongTermApp` exposes real panel view-model snapshots:
  an agent-sidebar snapshot from `bongterm-ui::agent_sidebar::AgentSidebarVm`
  and a resource-dashboard snapshot with at least the BongTerm host process.

GREEN:
- Add UI-local resource dashboard DTOs to `bongterm-ui` so the UI does not
  depend on `bongterm-ledger`.
- Render `AgentSidebarVm::view()` in the agent panel.
- Render the UI-local resource dashboard VM in the resource panel.
- Translate `bongterm-ledger::DashboardViewModel` into the UI-local DTO in
  `bongterm-app`.

Acceptance:
- `cargo test -p bongterm-app --test shell_app`
- `cargo test -p bongterm-ui`
- `cargo clippy -p bongterm-app -p bongterm-ui --all-targets --all-features -- -D warnings`
- `cargo run -p bongterm-app` opens the composed shell with panel data instead
  of placeholder panel labels.

Stop after `1.R.2` was honored. Later PRD feature actions remain separate.

### Corrective reopen: `1.R.3` resize/layout correction

Status 2026-06-11: COMPLETE. The composed app now routes window resize through
shell-owned center-pane layout metrics and resizes terminal PTY/parser/grid
state from terminal surface bounds instead of the full window.

Context: `1.R.2` renders live side-panel data, but the terminal runtime still
uses the top-level window resize event to compute PTY/grid dimensions. With
shell chrome and side panels visible, the terminal grid must be sized from the
actual center-pane bounds, not the whole window.

Files:
- `crates/bongterm-app/src/shell_app.rs`
- `crates/bongterm-app/src/terminal_app.rs`
- `crates/bongterm-app/tests/shell_app.rs`
- `crates/bongterm-ui/src/lib.rs`

RED:
- Add a test proving the composed app converts a shell content/window size into
  terminal center-pane grid dimensions after subtracting shell chrome, padding,
  and side-panel widths.

GREEN:
- Move terminal resize calculation behind an app/shell-owned layout helper that
  receives the terminal center-pane dimensions.
- Route resize messages so the terminal parser and PTY receive dimensions based
  on the center pane only.
- Keep panel rendering and `bongterm-ui` DTO boundaries unchanged.

Acceptance:
- `cargo test -p bongterm-app --test shell_app`
- `cargo test -p bongterm-ui`
- `cargo clippy -p bongterm-app -p bongterm-ui --all-targets --all-features -- -D warnings`
- Resizing the running app keeps terminal text/grid aligned inside the center
  pane without side-panel overlap or excessive blank columns.

`1.R.3` local verification:
- RED: `cargo test -p bongterm-app --test shell_app` failed with missing
  `terminal_surface_size_for_window` and `AppMessage::WindowResized`.
- GREEN: `cargo test -p bongterm-app --test shell_app` — pass, 3 tests.
- `cargo test -p bongterm-ui` — pass, 46 tests.
- `cargo clippy -p bongterm-app -p bongterm-ui --all-targets --all-features -- -D warnings` — pass.
- `cargo build -p bongterm-app` — pass.
- `cargo fmt --all -- --check` — pass.
- `git diff --check` — pass.
- Manual resize smoke opened `target\debug\bongterm-app.exe`, resized to
  `900x600` and `1200x720`, and remained responsive.

Release pipeline mode supersedes the old stop-after-session instruction only
after status docs are updated and blocker checks pass. Later PRD feature actions
remain blocked by their release gates.
