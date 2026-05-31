# BongTerm — Ship-Readiness Assessment (2026-05-31)

> Written by a fresh session in response to "get this app ready to ship."
> Evidence-based ground-truth audit, not a status restatement. Where a claim is
> inferred rather than directly verified, it is marked **[inferred]**.

> **Update 3 (2026-05-31, later session — CI MADE GREEN):** `ci.yml` was red on
> three gates at the start of this session and is now **fully green on all 7
> steps** (stable 1.95). Landed on `master`: `ccef9ca` (restore the
> storage-sqlite repo-conformance coverage that an uncommitted change had deleted
> to dodge `check-deps`, fixed properly via a one-line allow-list entry +
> re-lock), `9c98f06` (stable rustfmt + ~38 behavior-preserving clippy
> `-D warnings` fixes across the workspace), `03b5678` (`cargo deny`: ignore the
> `adler` *unmaintained* advisory + allow `WTFPL` — both transitive via vendored
> wezterm), `41047c4` (stale-path + AGENTS.md doc fixes). Verified: fmt-check
> clean, clippy exit 0, `cargo test --workspace` 343 pass/0 fail/1 ignored,
> `cargo deny check` ok, `check-deps` ok, `cargo build --release --workspace`
> ok, `terminal_session` slice test 1 pass. **Caveats unchanged below:** this
> unblocks CI but does not make a usable product — the GUI *visual* render is
> still unverified (human-only step), and `nightly.yml`'s Phase-1 perf gates
> (#1,#4-8,#17,#28,#29 = task `1.exit`) are still unwired. See `handoff.md`.
>
> **Update 2 (2026-05-31, same session — VERTICAL SLICE LANDED):** The gating
> "is it a working terminal" finding below is now **partially resolved**.
> `cargo run -p bongterm-app` opens a window running a **real shell** (pwsh/cmd).
> - New `bongterm-app::session::TerminalSession` (iced-free, testable) spawns a
>   real ConPTY via `PortablePtyHost`, feeds output through `WezTermAdapter`, and
>   renders a `SurfaceSnapshot`. `WezTermAdapter::current_snapshot` — previously a
>   stub returning empty runs (the real reason nothing could render) — now
>   extracts grid text + cursor.
> - A thin iced shell (`terminal_app.rs`) pumps ConPTY bytes (reader thread →
>   mpsc → `time::every` tick → parser) and maps keystrokes → `write_input`.
> - **Evidence:** headless integration test `tests/terminal_session.rs` proves
>   spawn→write→read→parse→snapshot (asserts a `cmd.exe` `echo hello` lands in the
>   grid) — GREEN; binary builds clean; a bounded launch ran 6s with no crash.
> - **Honestly NOT verified (no display here):** that glyphs visually render and
>   typing is visible in the GUI. The parse/snapshot path is asserted; the on-
>   screen render path compiles + launches but isn't visually asserted — the user
>   should run it. **Cuts (all deliberate v1):** pragmatic iced-`text` grid (not
>   the wgpu `TerminalPipeline`), fixed 80×24 (no resize), no colour/attrs, the
>   `bongterm-ui` shell (tabs/palette/sidebar) is bypassed pending integration.
>
> **Update 1 (2026-05-31, same session):** Phase 2 (agent observability) is now
> **code-complete** — tasks 2.C.3a–2.C.3c, 2.D.1, 2.EXIT landed (commits
> `31a9c0e`→`662e31b`). Both Phase 2 P0 gates are green locally and wired into a
> new `nightly.yml`: **#24** (`cargo run -p xtask -- prompt-injection-corpus` →
> "32 scenarios passed gate #24") and **#15** (`cargo test -p bongterm-agents
> --test gate15` → 3 pass). `cargo test --workspace` is green; `check-deps` ok.
> **The core verdict below is unchanged:** this is still not a working terminal —
> Phase 2 was built on top of the unwired terminal. The vertical slice (§"Honest
> path to ship" step 1) remains the gating work for a usable product, and
> workspace-wide `clippy -D warnings` / `fmt --check` still fail on pre-existing
> debt in other crates (1.exit / hygiene scope).

## Top-line verdict

**Not shippable. BongTerm is not yet a working terminal.** It compiles, all unit
tests pass, and the binary opens a GUI window — but that window cannot display a
shell. The product's subsystems are built and unit-tested *in isolation*; the
integration layer that makes them a terminal does not exist yet.

This contradicts the prior handoff's "Phase 1 100% complete." That statement is
true only in the narrow sense that every Phase 1 *unit* is coded and unit-green.
It is **not** true that Phase 1 delivers a usable terminal — its own exit gates
(keystroke-to-glyph latency, live resource dashboard) are currently
*unmeasurable* because nothing is wired end-to-end.

## The one fact that reframes everything: the app is not wired

Dependency closure of the shipped binary (verified from `Cargo.toml` manifests):

```
bongterm-app   ->  bongterm-ui, bongterm-diagnostics
bongterm-ui    ->  bongterm-settings, iced
```

That is the *entire* runtime graph. The following crates — i.e. the actual
product — are **not** in the running app's dependency graph at all:

- `bongterm-pty`     (real ConPTY host) — **zero consumers anywhere**
- `bongterm-term`    (wezterm VT parser adapter) — consumed only by `test-kit`
- `bongterm-render`  (wgpu renderer) — consumed only by `test-kit`
- `bongterm-mux`, `bongterm-blocks`, `bongterm-ledger`, `bongterm-agents`,
  `bongterm-mcp`, `bongterm-security`, `bongterm-storage-sqlite`, … — none
  reachable from `bongterm-app`.

`bongterm-ui::run_shell()` is a real Iced application, but its `view()` renders
static placeholder panels (`shell_panel("Agents","collapsed")`, etc.). The
`terminal-surface` region is a text label. No PTY is spawned, no bytes are read,
no parser is fed, no glyphs are drawn from shell output, and keystrokes are not
routed to a child process. (`grep` for `PortablePtyHost|spawn|advance_bytes|
ingest_bytes` in `bongterm-ui/src` and `bongterm-app/src` → 0 matches.)

**Every product crate is an island reachable only by `cargo test`, never by
`cargo run`.** The parts are on the workbench; the engine is not in the car.

## What is genuinely done (real, not mocked, unit-tested)

These are real and pass tests — they just aren't connected:

- **ConPTY host** (`bongterm-pty::PortablePtyHost`): real `portable-pty` spawn;
  tests prove spawn-ok, nonzero pid, writer accepts input.
- **VT parser adapter** (`bongterm-term`): `WezTermAdapter::ingest_bytes` wired
  to vendored `wezterm_term::Terminal::advance_bytes` (commit `9913f9a`).
- **SQLite WAL store** (`bongterm-storage-sqlite`): all 8 repo traits, migrations,
  append-only sidecar chunks (BLAKE3 frame), crash-recovery scan.
- **OSC-133 command blocks** (`bongterm-blocks`): FTCS consumer, confidence model,
  block actions, fixture tests (bash/pwsh).
- **Resource ledger** (`bongterm-ledger`): sampler, DXGI VRAM sampler, dashboard
  view-model.
- **Agent observability** (`bongterm-agents`, Phase 2): Claude Code + Codex
  adapters (discover/classify), transcript sink, git-porcelain file-change
  tracker, approval queue with `EnforcementLevel`, replay builder, lifecycle
  state machine. All unit-green.
- **Settings / keybindings / mux topology / agent sidebar view-model**: present
  and tested.

## What is mocked, stubbed, or unverified on real hardware

- **Renderer device-loss, VRAM ceiling** — validated via *mocks*
  (`MockRendererBackend`), not a real DXGI device-removed event. **[inferred
  from handoff + graph god-nodes: 4 of 8 are `Mock*`]**
- **Renderer actually drawing glyphs to a window** — not demonstrated
  end-to-end. wgpu latency was measured in a Phase 0 *spike* (ADR-003), not in
  the app.
- **Phase 1 exit gates** (#1 keystroke-to-glyph p99 ≤5 ms, #4 throughput, #5 RSS,
  #6 VRAM, #29 live dashboard) — **not measurable today**; require the wiring
  above to exist first. `1.exit` (CI gate wiring) is unstarted.
- **CJK IME round-trip** — harness exists; live test deferred to Phase 5.

## Defects found this session

1. **Repo-move stale-build phantom failure (root-caused, fixed).** The repo was
   moved `C:\Users\souba\Documents\Projects\BongT` → `D:\Programming\Bongbetic\
   BongT`. `bongterm-blocks` fixture tests load via `env!("CARGO_MANIFEST_DIR")`,
   which was baked into the cached test binary at the old C: path → 5 tests
   failed with `ERROR_PATH_NOT_FOUND`. `cargo clean -p bongterm-blocks` + rebuild
   → **32/32 pass.** Recommend a one-time full `cargo clean` so all crates
   re-bake the new path. **The workspace is genuinely green after this.**
2. **Stale absolute paths in docs.** `CLAUDE.md`, `handoff.md`, `orca.md`, and the
   memory dir reference the old `C:\…\Documents\Projects\BongT` path. The
   resume/handoff protocol will mis-target. Needs a path sweep.
3. **Storage-sqlite conformance-coverage regression (uncommitted).** Working tree
   deletes the `bongterm-test-kit` dev-dep + 3 repo-conformance tests from
   `bongterm-storage-sqlite` to silence the `check-deps` dep-direction violation.
   `check-deps` scans `dev-dependencies` (`check_deps.rs:41`), so an in-crate
   `tests/` dir wouldn't help. Proper fix: host the storage conformance harness
   in `bongterm-test-kit` (which already depends on `-term`/`-render`) and run it
   against `SqliteStore` there — keeps coverage, keeps dep-direction legal.
   Either finish that or revert; don't ship the silent coverage loss.

## Honest path to "ship" (re-sequenced)

The current `orca.md` is building Phase 2 (agents) **on top of a terminal that
cannot display a shell.** That ordering is backwards for shipping. Corrected
spine:

0. **One-time hygiene:** full `cargo clean`; doc path sweep; resolve the
   storage-sqlite conformance item.
1. **VERTICAL SLICE (the true next step):** wire `app → ui → {pty, term, render}`
   so the window spawns a real shell, pumps `ConPTY bytes → parser → grid →
   renderer → glyphs`, and routes keystrokes back. This is what turns a pile of
   tested parts into a terminal. Until `cargo run -p bongterm-app` shows a live
   shell, nothing in Phases 2-6 is observable by a user.
2. **Make Phase 1 exit gates real & green** (#1,#4-8,#17,#28,#29) — now
   measurable because the pipe exists. Wire them into CI (`1.exit`).
3. **Re-integrate the already-built subsystems into the live shell** — blocks,
   ledger/dashboard, mux panes/tabs, settings, then the Phase 2 agent sidebar.
   Most code exists; it needs connecting, not writing.
4. **Then resume orca.md Phases 2→6** (agents already ~90% coded, MCP, secrets,
   accessibility, signed MSIX) and the long-pole non-code gates: 30 working-days
   solo dogfood + external beta + trademark search.

## Effort realism

"Ship" = Phases 1.exit→6.exit **plus** 30 working-days dogfood + external beta +
trademark/brand review. This is **many sessions, not one**, and the long-pole
items are calendar-bound (dogfood window) and human-bound (beta, legal), not
just code. No single session makes this app shippable. The highest-leverage
thing achievable now is step 1 (the vertical slice) — it converts the project
from "tested parts" to "a thing you can actually use," and unblocks every gate.

---
*Generated 2026-05-31. Evidence: dependency-manifest audit, `cargo test`
ground-truth run, source inspection of `run_shell`/`view`/`PortablePtyHost`.*
