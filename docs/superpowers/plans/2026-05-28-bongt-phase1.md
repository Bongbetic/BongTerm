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

Start with `1.UX.1` and build the full UX contract set (`1.UX.1`-`1.UX.10`) under `docs/ux/`.
