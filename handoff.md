# BongTerm Phase 1 Handoff — 2026-05-29

## Session summary

Phase 1 implementation **100% complete**. All code tasks done. `[next]` = `1.exit` (wire §6.1 gates into CI).

---

## All Phase 1 commits (since Phase 0 exit)

| Commit | Task | Summary |
|--------|------|---------|
| `8508c11` | 1.D.1 | Pane + tab topology model in `bongterm-mux` |
| `4c53109` | 1.D.2 | Split h/v, resize, focus cycle |
| `d03199f` | —    | orca.md status update |
| `03b1105` | —    | Docs: stale-ref fixes + Phase 2-6 plan files |
| `d8f502e` | 1.A.4b | `SettingsWriter` port + `FileSettingsProvider::write` (atomic tmp→rename). 20 tests. |
| `f590b7b` | 1.D.3 | `LayoutSnapshot` + `LayoutRepo` + `MuxRouter::snapshot/restore`. 67 tests. |
| `c860874` | 1.E.1-4 | OSC 133 FTCS consumer, `Confidence` enum, `BlockBuilder`, `BlockAction`, fixture tests. 32 tests. |
| `d63b18a` | 1.F.1-4 | `ResourceSampler`, `DxgiVramSampler`, `DashboardViewModel`, Windows+stub impl. 25 tests. |
| `5d494a0` | 1.G.1-4 | `SqliteStore` (WAL + all 8 repo traits), sidecar chunk writer (BLAKE3), crash recovery scan, `xtask cleanup-chunks` real impl. 22 tests. |
| `c3f2cbb` | —    | orca.md + handoff.md session 1 update |
| `a61c906` | 1.B.3 (gitlink) | Gitlink `vendor/wezterm` at `5046fc225992db6ba2ef8812743fadfdfe4b184a` (mode 160000). |
| `9913f9a` | 1.B.3 (wire) | `WezTermAdapter::ingest_bytes` → `wezterm_term::Terminal::advance_bytes`. `BongtermConfig` minimal `TerminalConfiguration`. Root workspace excludes `vendor/wezterm`. 11 tests. |
| `339244b` | —    | orca.md + handoff.md phase-complete update |

Full workspace `cargo test` = 0 failures on `master`.

---

## Current state

- **`[next]` in `orca.md`:** `1.exit` — wire §6.1 gates into CI
- **No deferred code tasks.** All Phase 1 implementation done.

---

## Phase 1 exit gates (CI wiring needed)

Per spec §6.1 — must be green × 7 consecutive nightly runs:

| Gate | Description | Where to wire |
|------|-------------|---------------|
| #1 | keystroke-to-glyph p99 ≤ 5 ms | benchmark harness → CI |
| #4 | terminal bytes/s throughput | benchmark harness → CI |
| #5 | RSS ≤ 200 MB steady-state | integration test → CI |
| #6 | VRAM budget compliance | `DxgiVramSampler` → CI |
| #7 | shell integration confidence grading | `BlockBuilder` fixture → CI |
| #8 | block actions available for High/Medium blocks | `BlockAction` test → CI |
| #17 | settings persist across restart | `FileSettingsProvider` roundtrip → CI |
| #28 | layout restore on launch | `LayoutRepo` + `MuxRouter::restore` → CI |
| #29 | resource dashboard shows live values | `DashboardViewModel` → CI |

These are CI checks, not new code. Wire into `.github/workflows/` (skeleton exists).

---

## Key architectural decisions (Phase 1)

- **`LayoutSnapshot` topology-only** — rects + focus indices; per-pane cwd/shell in `WorkspaceSnapshot` (bongterm-app). Module ownership matrix binding.
- **`SqliteStore` uses `unsafe impl Send + Sync`** — `Mutex<Connection>` sound; `Connection` is `!Send` only for thread-local SQLite error state.
- **No FK REFERENCES in `0001_init.sql`** — conformance tests don't pre-insert parent rows.
- **Sidecar frame**: `[u64 monotonic_id][u8;32 blake3][u32 len][payload]`. Hash mismatch = torn write; reader stops cleanly.
- **`CommandBlock.command` always `""`** — PTY input capture deferred. OSC 133 gives prompt boundaries, not typed command.
- **`vendor/wezterm` gitlink** (mode 160000, `5046fc22`). Root workspace `exclude = ["vendor/wezterm"]` prevents nested-workspace conflict.
- **`visible_lines` is `#[cfg(test)]` in wezterm-term** — external consumers use `lines_in_phys_range(0..N)`. Fresh terminal has no scrollback so phys-row 0 = visible row 0.
- **`BongtermConfig`** minimal `TerminalConfiguration`: only `color_palette` required; writer is `Box::new(std::io::sink())` (ConPTY handles input at a higher layer).

---

## Phase 2 plan ready

`docs/superpowers/plans/2026-05-29-bongt-phase2.md` — 17 tasks (agent observability: adapters, transcript writer, approval queue, replay, lifecycle controls, prompt-injection corpus).

---

## Key artifacts

| Artifact | Path |
|----------|------|
| Task authority | `orca.md` |
| Authoritative spec | `docs/PRD/bongterm_prd_v7.md` |
| Design doc (gate numbering) | `docs/superpowers/specs/2026-05-27-bongt-mvp0-design.md` |
| Phase 1 plan | `docs/superpowers/plans/2026-05-28-bongt-phase1.md` |
| Phase 2 plan | `docs/superpowers/plans/2026-05-29-bongt-phase2.md` |
| ADRs (0003–0007, all Accepted) | `docs/adr/` |
| wezterm-term adapter | `crates/bongterm-term/src/adapter.rs` |

---

## Recommended next actions

1. **`1.exit`** — wire §6.1 gates into `.github/workflows/`. Benchmarks #1/#4 need criterion harness; integration tests #17/#28/#29 need fixture runners; resource gates #5/#6 need sampler-driven assertions.

2. **Phase 2** — invoke `superpowers:subagent-driven-development` or `superpowers:executing-plans` against `docs/superpowers/plans/2026-05-29-bongt-phase2.md`.

---

*Generated 2026-05-29. All changes on `master`. No sensitive data.*
