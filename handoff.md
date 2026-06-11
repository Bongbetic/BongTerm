# BongTerm Handoff — live-terminal slice: colour, cursor, event-driven I/O, resize — 2026-06-01

## TL;DR

Interactive session (human at the keyboard running `cargo run -p bongterm-app`
and reporting what rendered). Drove the **integration spine** — the
SHIP-READINESS critical path — forward in verified increments. Every renderer
change below was **visually confirmed by the user**, not committed blind.

Landed on `master` (oldest→newest):

| Commit | What |
|--------|------|
| `d90f0b6` | ci: trigger `ci.yml` on `master` + `workflow_dispatch` (was `main`-only → CI had never run). |
| `e10f8eb` | feat(render): real per-run **colour + attributes** in the live renderer (was codepoint-only grey). ✅ visually confirmed. |
| `25436a3` | feat(render): draw the **cursor** as a block glyph in the cosmic-text stream (aligns free, no quad pass). ✅ visually confirmed. |
| `35cbc94` | docs(1.exit): gate **#6** idle-CPU measured baseline + diagnosis. |
| `860b72b` | feat(app): **event-driven ConPTY I/O** via a per-pane subscription worker (no idle timer). ✅ confirmed working. |
| *(pending)* | feat: **window resize** → cell-metrics → cols/rows → ConPTY reflow. ✅ visually confirmed (fills window + reflows). |

Post-session truth (verify, don't trust): `cargo test --workspace` green;
`cargo clippy --workspace --all-targets --all-features -- -D warnings` exit 0;
`cargo fmt --all --check` clean.

## What changed, by subsystem

- **`bongterm-term`** (`adapter.rs`, `surface.rs`): `current_snapshot` now extracts
  real per-run fg/bg colour + attrs via wezterm `Line::cluster` + palette
  `resolve_fg/bg` (was hardcoded white/black/0). Added `surface::attr` bitfield and
  `WezTermAdapter::resize`. Headless TDD: truecolor fg, bold+underline, start-col,
  resize dims.
- **`bongterm-render`** (`lib.rs`): `SurfaceSnapshot` is now `spans: Vec<CellSpan>`
  + `cursor: CursorVis` (was `cells: Vec<u32>`). `prepare` builds a cosmic-text
  **rich-text** stream with per-span fg colour + weight/slant, and injects a block
  **cursor** glyph at the cursor cell (only at/after row content end — never shifts
  text; mid-line cursor is a documented v1 gap). New `monospace_cell_size(font_size)`
  (measures advance via a one-shot FontSystem, no GPU) and `grid_dims(...)` for
  window→grid mapping. Tests for all.
- **`bongterm-pty`** (`host.rs`): `PtyChild::resize(cols, rows)` (renamed `_master`
  → `master`, now used).
- **`bongterm-app`** (`terminal_app.rs`): **rewrote the I/O to event-driven.** A
  `Subscription::run_with(shell, pane_worker)` worker owns the ConPTY child + a
  blocking reader thread; emits `Message::Output` only when bytes arrive (idle =
  no messages = no repaints). Keystrokes + resize flow back via a `tokio` channel
  (`WorkerCmd::{Input,Resize}`) handed over in `Message::Ready`. The VT parser/grid
  stays in app state (UI thread — need not be `Send`). Window `Resized` events map
  to cols/rows via the cell metrics and reflow both the parser and the PTY.
  `Cargo.toml`: added `tokio`.

## Gate #6 (idle CPU) — measured, NOT yet strict-pass

See `docs/phase1-exit-gates.md` for the full table. Bottom line: idle CPU is
**~0.05% all-core / ~0.6% single-core** (60s, pwsh). **Passes under all-core
normalization, fails under single-core.** The spec doesn't pin the normalization;
the honest target is the strict (single-core ≈ 0) reading — do **not** claim #6
green off the all-core number.

- iced 0.14 is `ControlFlow::Wait` (event-driven), so it is NOT a framework spin.
- Event-driven I/O **improved** the shipped baseline (0.83% → 0.57% single-core)
  and removed output latency, but did not reach ~0. The residual floor is repaints
  driven by the **shell's own periodic output** (suspected pwsh PSReadLine
  animation; **confirm with a `cmd.exe` idle measurement** — cmd doesn't animate).
- **MEASURE FIRST, then fix (do not reverse this):** run the `cmd.exe` idle
  measurement **before** writing any #6 fix. cmd doesn't animate, so if cmd idles
  ~0 the floor is shell-output repaints (→ repaint-suppression is the right fix);
  if cmd is *also* ~0.6% single-core the floor is the reader thread not parking on
  ConPTY (→ repaint-suppression fixes nothing; fix the reader instead). The
  pwsh-animation cause is **suspected, not verified** — building the fix on the
  guess repeats the #6 mistake (the event-driven 100 lines didn't reach ~0 because
  the 2000 ms result already showed a floor). The app + the 60 s PowerShell sampler
  are ready; one measurement decides.
- **Strict-pass fix (only after the measurement):** if shell-output repaints —
  suppress repaints when the visible grid is unchanged (cleanest in the worker:
  move the parser there, de-dup snapshots, send only on grid change; needs wezterm
  `Terminal: Send` — verify). If reader-thread spin — make the ConPTY read block.

## Next steps (priority order)

1. **Split panes — gate #7** (the user's stated next goal). Foundation is in place:
   the PTY worker is already keyed for `run_with`, and resize/cell-metrics exist.
   Plan: app state holds N panes (each: `WezTermAdapter` + snapshot + `WorkerCmd`
   sender), a `bongterm-mux::InMemoryMux` for layout, `active_pane`. `subscription()`
   returns one worker per pane keyed by `(pane_id, shell)`. `Message` gains a pane
   id on `Ready`/`Output`. `view()` lays panes out per mux `Rect` (iced row/column
   of shader widgets). Keybindings: split H/V, focus-next. Per-pane cols/rows from
   the pane rect × cell metrics. **Interactive — verify each step in the GUI.**
   Heads-up: the first step is an all-or-nothing single→N restructure of
   boot/update/view/subscription/Message — no sub-slice of it compiles-and-runs,
   so it wants a fresh context budget. Also: a pane whose shell **exits** currently
   just freezes its last snapshot (fine for single-pane v1); multi-pane needs
   dead-pane handling (e.g. a "pane exited" banner + close/restart).
2. **Resource dashboard — gate #17.** The worker now has `child.pid`; surface it
   (e.g. extend `Message::Ready` with the pid) so `bongterm-ledger` can sample the
   pane's process tree. Build `CurrentProcessSampler::register_pid` **with** this
   wiring (its per-pane PID→pane contract is integration-defined — don't build it
   standalone). Then the dashboard view-model + a panel in the app.
3. **#6 strict-pass** — run the `cmd.exe` isolation measurement **first** (it
   decides whether the fix is repaint-suppression or a reader-thread fix; see
   §"Gate #6"), then apply the indicated fix. Do not build the fix on the
   unverified pwsh-animation guess.
4. **Background quad pass (deferred):** cell backgrounds + reverse-video + a
   quad-based cursor. Gate-irrelevant; needs the advance-measured NDC overlay
   (`monospace_cell_size` already gives the advance). Verify alignment at col 79.
5. **#4 / #5-full / #2 / #3:** cold-start-to-first-frame, full-app RSS + DXGI VRAM,
   keystroke-to-glyph p99, throughput — measurement harnesses (need display/GPU).
6. **Confirm CI for real:** the trigger is fixed (`d90f0b6`); push / open a PR so
   `ci.yml` + `nightly.yml` actually run on `windows-latest`. The 7-nightly clock
   for the Phase-1 exit can't start until they do. **This is the true long-pole.**

## Shape notes / discipline

- Renderer/GUI changes are **verified interactively** (human runs the app), never
  committed blind. Keep this for panes/dashboard.
- Do **not** fake-green the display/GPU gates (#4, #5-full, #6 strict, #7, #17) from
  headless harnesses. They need a real window + a human.
- "Finish" = `1.exit`→`6.exit` + 30-working-day dogfood + external beta + trademark.
  Many sessions; calendar- and human-bound. This session advanced the spine; it did
  not (and could not) finish the product.

## Key artifacts

| Artifact | Path |
|----------|------|
| Task authority | `orca.md` |
| Gate triage + #6 baseline | `docs/phase1-exit-gates.md` |
| Ground-truth audit | `SHIP-READINESS.md` |
| Canonical gate criteria | `docs/superpowers/specs/2026-05-27-bongt-mvp0-design.md` §6.1 |
| CI | `.github/workflows/ci.yml`, `nightly.yml` |

*Generated 2026-06-01. All changes on `master`, not pushed. No sensitive data.*
