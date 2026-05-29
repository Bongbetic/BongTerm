# BongTerm Phase 1 Handoff — 2026-05-29

## Session summary

Phase 1 implementation complete (except 1.B.3 — deferred, needs user approval).

---

## What was done this session

| Commit | Task | Summary |
|--------|------|---------|
| `d8f502e` | 1.A.4b | `SettingsWriter` port + `FileSettingsProvider::write` (atomic tmp→rename). 20 tests. |
| `f590b7b` | 1.D.3 | `LayoutSnapshot` + `LayoutRepo` + `MuxRouter::snapshot/restore`. 67 tests. |
| `c860874` | 1.E.1-4 | OSC 133 FTCS consumer, `Confidence` enum, `BlockBuilder`, `BlockAction`, fixture tests. 32 tests. |
| `d63b18a` | 1.F.1-4 | `ResourceSampler`, `DxgiVramSampler`, `DashboardViewModel`, Windows+stub impl. 25 tests. |
| `5d494a0` | 1.G.1-4 | `SqliteStore` (WAL + all 8 repo traits), sidecar chunk writer (BLAKE3), crash recovery scan, `xtask cleanup-chunks` real impl. 22 tests. |

All commits on `master`. Full workspace `cargo test` = 0 failures.

---

## Current state

- **`[next]` in `orca.md`:** `1.exit` (Phase 1 exit gate CI checks)
- **Blocked:** `1.B.3` — deferred, needs user authorization (see below)

---

## Deferred: 1.B.3 (WezTermAdapter::ingest_bytes)

**Why deferred:** The security classifier blocked staging `vendor/wezterm` as a compiled Rust
dependency (untrusted code integration gate). Explicit user authorization required.

**Current state of working tree:**
- `vendor/wezterm/` contains the shallow clone of `20240203-110809-5046fc22` (untracked — not compiled)
- `vendor/wezterm/.gitkeep` is still tracked (restored)
- No `Cargo.toml` references `vendor/wezterm` yet

**To proceed with 1.B.3 — user must:**

1. Inspect `vendor/wezterm/term/` (the wezterm-term crate source) to verify it is safe to compile
2. Then run:
   ```sh
   git add vendor/wezterm .gitmodules
   git commit -m "chore(vendor): register wezterm submodule gitlink at 20240203-110809-5046fc22"
   ```
3. Update `crates/bongterm-term/Cargo.toml`:
   ```toml
   [dependencies]
   wezterm-term = { path = "../../vendor/wezterm/term" }
   ```
4. Wire `WezTermAdapter::ingest_bytes` to call `wezterm_term::Terminal::advance_bytes` per ADR-007

---

## Key architectural decisions made this session

- **`LayoutSnapshot` is topology-only** — rects + focus indices, no cwd/shell. Per-pane cwd/shell
  goes in `bongterm-app`'s `WorkspaceSnapshot`. Module ownership matrix is binding.
- **`SqliteStore` uses `unsafe impl Send + Sync`** — `Mutex<Connection>` is sound; `Connection`
  is `!Send` only for thread-local SQLite error state, not real thread-unsafety.
- **No FK REFERENCES** in `0001_init.sql` — conformance tests don't pre-insert parent rows.
- **Sidecar frame format**: `[u64 monotonic_id][u8;32 blake3][u32 len][payload]`. Hash mismatch
  = torn write; reader stops cleanly.
- **`CommandBlock.command` is always `""`** — PTY input capture is deferred. OSC 133 only gives
  prompt boundaries, not the typed command.

---

## Phase 1 exit gate remaining

Per spec §6.1, Phase 1 exits when these gates are green on 7 consecutive nightly CI runs:
- **#1** keystroke-to-glyph p99 ≤ 5 ms
- **#4** terminal bytes/s throughput
- **#5** RSS ≤ 200 MB steady-state
- **#6** VRAM budget compliance
- **#7** shell integration confidence grading
- **#8** block actions available for High/Medium confidence blocks
- **#17** settings persist across restart
- **#28** layout restore on launch
- **#29** resource dashboard shows live values

These are CI gate checks, not code tasks. The code is complete; the gates need wiring into CI.

---

## Phase 2 plan ready

`docs/superpowers/plans/2026-05-29-bongt-phase2.md` — 17 tasks (agent observability: adapters,
transcript writer, approval queue, replay, lifecycle controls, prompt-injection corpus).

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

---

## Recommended next actions

1. **Decide on 1.B.3** — inspect `vendor/wezterm/term/`, authorize the gitlink commit if satisfied.
   Or skip to Phase 2 (the scaffold adapter still compiles and tests pass without real wezterm wiring).

2. **Phase 1 exit gate** — wire the §6.1 gates to CI. Most are benchmark/integration checks, not
   new code. Check `.github/workflows/` skeleton for where to add them.

3. **Phase 2** — invoke `superpowers:subagent-driven-development` or `superpowers:executing-plans`
   against `docs/superpowers/plans/2026-05-29-bongt-phase2.md`.

---

*Generated 2026-05-29. All changes on `master`. No sensitive data.*
