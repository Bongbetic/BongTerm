# Phase 1 Exit (`1.exit`) — Gate Triage & Status

**Date:** 2026-05-31 · **HEAD at audit:** `974c7ed` · **Build:** `cargo test --workspace`
= **343 passed / 0 failed / 1 ignored** (`CARGO_EXIT=0`, 45 binaries) — verified green
this session, not trusted from docs.

This document is the detailed breakdown of orca task **`1.exit`**. orca stays the
task authority; this is the working state for the nine Phase-1 exit gates.

> **STATUS (2026-05-31, commits `b81eaf0`→`2e0947e`):** the **measurable subset is
> DONE** — built, verified with real numbers, wired into `nightly.yml`, committed.
> Post-session: `cargo test --workspace` = 350 pass / 0 fail; fmt + clippy
> (`--workspace -D warnings`) clean.
>
> | Gate | Result | CI step |
> |---|---|---|
> | #1 shell-smoke | ✅ DONE — PASS 4/6 (CMD, WinPS, PS7, SSH); skip-log Git Bash, WSL | `cargo test -p bongterm-app --test gate01_shell_smoke` |
> | #5 RSS | ⚠ **PARTIAL** — headless engine-core lower-bound **9.8 MB** (no window/wgpu/render loop; does NOT verify the full-app 120 MB budget) | `cargo test -p bongterm-app --test gate05_rss` |
> | #8 blocks | ✅ DONE — fixture corpus green + detection p99 **500 ns** ≤ 5 ms | `cargo test -p bongterm-blocks` |
> | #28 settings | ✅ DONE — backup + Safe Mode + v1→v2 migration built + tested | `cargo test -p bongterm-settings --test gate28_settings_recovery` |
> | #29 storage | ✅ DONE — torn / checksum / corrupt-DB recovery | `cargo test -p bongterm-storage-sqlite --test gate29_storage_recovery_suite` |
>
> **Still BLOCKED** (integration + GPU/display + human visual — do not fake): #4,
> **#5 full-app RSS + VRAM**, #6, #7, #17. The #5-RSS tripwire above is a floor on
> the engine, not the gate — re-measure full-app RSS when the renderer is wired.
> These are the next session's "wire renderer/mux/ledger into the app" work.
> `1.exit` overall stays open until these land **and** the nightly is green for 7
> consecutive runs on real CI.

---

## Gate-number correction (IMPORTANT — orca labels were wrong)

orca.md's inline labels for `1.exit` were **off-by-one** against the canonical
numbering. The canonical source is the design spec §6.1
(`docs/superpowers/specs/2026-05-27-bongt-mvp0-design.md`), which drives gate
numbering. The Phase-1 exit set (orca line 129) is **{1, 4, 5, 6, 7, 8, 17, 28, 29}**,
mapped through §6.1 criteria:

| # | §6.1 criterion | orca's WRONG inline label |
|---|---|---|
| 1 | Shell profiles launch (PS7, WinPS, CMD, Git Bash, WSL, SSH) | ~~keystroke-to-glyph p99~~ |
| 4 | Cold startup ≤300 ms; shell-integration ready ≤800 ms | ~~throughput~~ |
| 5 | RSS ≤120 MB core / ≤25 MB per-pane; VRAM ≤256 MB (RTX) / ≤128 MB (iGPU) | ~~VRAM~~ |
| 6 | Idle CPU ≤0.1 % / 1 pane / 60 s; ≤0.4 % / 4 panes | — |
| 7 | Tabs, split panes (H+V), resize, focus cycle | — |
| 8 | Command blocks + confidence labels (PS, Bash/WSL); CMD heuristic | — |
| 17 | Resource dashboard attribution per pane/session | ~~live dashboard~~ |
| 28 | Settings load + validation-failure + backup + Safe-Mode fallback | — |
| 29 | SQLite WAL + sidecar recovery (torn / checksum / corrupt; no fabrication) | — |

The set **excludes #2 (keystroke-to-glyph p99) and #3 (stream throughput)** — the two
gates that need the as-yet-unwired wgpu renderer. That exclusion corroborates the
mapping: Phase 1's exit set is exactly the subset measurable without the renderer.

**Phase-enumeration hole (flagged for a future session):** §6.1 **#2, #3** (renderer
perf) and **#27** (no open P0/P1 correctness defect) are in **no phase's** exit set.
#2/#3 must land when the real renderer is wired into the app; #27 is a release-review
gate. Do not let them fall through.

---

## Triage — measurable now vs blocked on integration

**Discriminator:** can the gate be measured against **real product behavior**,
**headless / no-GPU**, with the subsystems that are **actually wired**? CI
(`windows-latest`) has **no display, no GPU, and usually no WSL distro**. A harness
that only exercises the iced-text placeholder, an unwired crate, or passes by
construction is gate-gaming — forbidden (cf. the `GateEnforcement::Default` near-miss
and the storage-sqlite coverage-deletion incident).

### Measurable now — build/verify this session

| # | Plan | Existing? |
|---|---|---|
| **1** | Shell-smoke integration test: for each profile, resolve exe → spawn real `TerminalSession` (ConPTY→parser→snapshot) → write probe → assert probe text in grid. **Skip-and-loudly-log** absent profiles. | NEW |
| **5 (RSS only)** | Working-set of a process holding 1 real `TerminalSession`; honest core-engine RSS (**excludes GUI/renderer**). VRAM part → BLOCKED. | NEW |
| **8** | Fixture corpus already green (`bongterm-blocks`: 4 fixtures + confidence model) — wire `cargo test -p bongterm-blocks`. Block-detection **latency** bench (≤5 ms after `command_end`) → NEW, headless (pure logic). | PARTIAL |
| **29** | `sidecar.rs` already detects torn-write + checksum-mismatch + `scan_for_recovery` (tested). EXTEND with corrupt-DB case; consolidate as `storage_recovery_suite`. | PARTIAL |
| **28** | `FileSettingsProvider` keeps last-valid snapshot after reload failure. BUILD missing **backup-on-corrupt + Safe-Mode fallback + schema migration**, then `settings_migration_and_last_known_good`. | PARTIAL |

### Blocked on integration / display / GPU — NOT this session, do **not** fake green

| # | Why blocked |
|---|---|
| **4** | GUI cold-start-to-first-frame needs a display; shell-integration-ready needs OSC-133 scripts installed. |
| **5 (VRAM)** | Needs the real wgpu renderer wired into the app **and** a GPU. = renderer-integration. |
| **6** | Meaningful only against the real render loop; a headless session idles at ~0 → green-by-construction. |
| **7** | App has no pane support; `bongterm-mux` is unwired. = integration. |
| **17** | `bongterm-ledger` is unwired into the app. = integration. |

The blocked set **is** the SHIP-READINESS audit's "wire the real renderer +
re-integrate subsystems" step. It deserves its own session and a **human** for the
GUI visual check (no headless session can confirm glyphs render).

---

## Wiring & verification protocol

1. Build/extend the harness in the owning crate (TDD: red → green).
2. **Run it and confirm it emits a real number/observation** (not a constant-true).
3. Only then add a step to `.github/workflows/nightly.yml` (pattern: existing
   `gate15` / `gate24` steps).
4. Commit per gate with the measured evidence in the message.

## Honest scope note

"Ship" = `1.exit`→`6.exit` **plus** 30 working-days Stage-A dogfood + Stage-B beta +
trademark/brand review + the human GUI-visual check. Calendar- and human-bound; **not
one session**. This session advances the measurable subset of `1.exit` and leaves an
honest `[block]` at the integration wall.
