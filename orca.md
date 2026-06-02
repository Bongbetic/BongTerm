# orca.md — BongTerm MVP-0 Task Orchestrator

> **Single source of truth for what to do next.** Each completed task is **struck from this list in place** — when a task is done, edit this file and remove it. Do not mark with checkbox-done and leave; do not append history. This file shrinks over time.
>
> **Re-plan rule.** Before starting Phase N+1, invoke `superpowers:writing-plans` against the spec section that governs that phase. Also query the AnythingLLM `engineer` workspace for phase-specific engineering insights, risks, ordering changes, and acceptance criteria; fold useful results into the new phase plan. Phases 1-6 are outlines only — they gain TDD-level detail at their re-plan boundary. **Never implement a phase from outline-level tasks alone.**
>
> **Status legend:**
> - `[next]` = next actionable task
> - `[block]` = blocked on a dependency named in parens
> - no prefix = not yet the next item

---

## Session Resume Guide

> **Read this section first on every new session.** Orients you without reading the full codebase.

### Where to find context

| Question | Where to look |
|----------|--------------|
| What is BongTerm? | `docs/superpowers/specs/2026-05-27-bongt-mvp0-design.md` (canonical, drives §6.1 gate numbering) or `docs/PRD/bongterm_prd_v7.md` (authoritative 1063-line PRD, §0–§23) |
| What was decided architecturally? | `docs/adr/` — 7 ADRs, all Accepted |
| What was built in Phase 0? | `git log --oneline v0.0.1-scaffold..v0.0.4-phase0-exit` |
| What is the Phase 0 TDD plan (full detail)? | `docs/superpowers/plans/2026-05-27-bongt-mvp0.md` |
| What crates exist? | Root `Cargo.toml` `[workspace.members]` — 20 product crates + xtask + 5 spike harnesses |
| What port traits were defined? | `crates/bongterm-*/src/lib.rs` — each crate has trait + mock + conformance tests |
| What did the Wave 0 spikes measure? | `docs/adr/0003-0007` — latency, VRAM, device integration, IME, wezterm API |
| What are the hard constraints? | `CLAUDE.md` §Hard non-goals, §Security contract, §Terminal hot-path rules |
| What CI gates exist? | `.github/workflows/` (skeleton) + `CLAUDE.md` §Required CI gates |
| Memory from past sessions? | `C:\Users\souba\.claude\projects\D--Programming-Bongbetic-BongT\memory\` |

### Phase completion status

| Phase | Status | Tag | Exit condition |
|-------|--------|-----|----------------|
| **Phase 0** Scaffold + Spikes | ✅ **COMPLETE** | `v0.0.4-phase0-exit` | All gates green; ADRs 003–007 Accepted |
| **Phase 1** Usable Terminal | 🔨 **IN PROGRESS** — `ci.yml` green _locally_ on stable 1.95 (CI not yet run; `master`-vs-`main` trigger mismatch). **`1.exit` measurable subset done** (gates #1, #8, #28, #29 fully done + #5-RSS *partial* headless tripwire, built+wired+green locally, commits `b81eaf0`→`2e0947e`). **Remaining = #4, #5 (full RSS+VRAM), #6, #7, #17 — blocked on wiring renderer/mux/ledger into `bongterm-app`** (needs GPU/display + human visual). | — | §6.1 #1,#4-8,#17,#28,#29 green × 7 nightlies |
| **Phase 2** Agent Observability | 🔨 **CODE COMPLETE** — all tasks 2.A.0–2.C.3c + 2.D.1 done; gates #15 + #24 GREEN locally + wired into `nightly.yml`; Phase 3 re-plan complete | — | §6.1 #15,#24 green × 7 nightlies |
| **Phase 3** Developer UX | 🔨 **IN PROGRESS** — `[next]` = `3.B.1` smart-history filters | — | §6.1 #9-14 green |
| **Phase 4** MCP + Secrets + Security | 📋 Planned (23 tasks) | — | §6.1 #16,#19,#23,#31 green + threat-model review |
| **Phase 5** Hardening + Release Prep | 📋 Planned (41 tasks) | — | §6.1 #18,#20,#21,#25,#26,#30 green + clean-VM smoke |
| **Phase 6** Dogfood → Public | 📋 Planned (24 tasks) | — | `v0.1.0-mvp0` shipped |

### Current status (2026-06-01, live-terminal slice advanced, tree clean)

Interactive session — the live terminal slice was driven forward in human-verified
increments (`cargo run -p bongterm-app`, user confirmed each render). Commits
`d90f0b6`→(resize) on `master`:

- **Colour + attributes** (`e10f8eb`): real per-run fg/bg + bold/italic/underline
  extracted from wezterm `Line::cluster` + palette; renderer draws per-span colour
  via cosmic-text rich text. ✅ visually confirmed.
- **Cursor** (`25436a3`): block glyph at the cursor cell in the layout stream
  (aligns free; mid-line is a v1 gap). ✅ visually confirmed.
- **Event-driven I/O** (`860b72b`): replaced the 33 ms poll timer with a
  `Subscription::run_with` worker that owns the ConPTY + reader thread and emits
  output only on real bytes; input/resize flow back via a `tokio` channel. Parser
  stays app-side. ✅ confirmed working. Worker is **pane-keyed** → #7-ready.
- **Window resize** (pending commit): `monospace_cell_size`/`grid_dims` map the
  window to cols/rows; `Resized` reflows both the parser and the ConPTY. Terminal
  now fills the window + reflows on drag. ✅ visually confirmed.
- **CI trigger fixed** (`d90f0b6`): `ci.yml` now triggers on `master` (+ dispatch).
  Still must push/PR to run on `windows-latest` — the 7-nightly clock can't start
  until then (true long-pole).
- **Gate #6 idle CPU**: measured (`35cbc94`) — ~0.05% all-core / ~0.6% single-core.
  Passes all-core, fails single-core; event-driven improved but did not reach ~0
  (floor = shell-driven repaints, suspected pwsh PSReadLine). Strict-pass needs
  unchanged-grid repaint suppression — flagged, not done. **Not green.**
- `cargo test --workspace` green; clippy `--workspace -D warnings` exit 0; fmt clean.
- Full session record: `handoff.md`; gate triage + #6 data: `docs/phase1-exit-gates.md`.

### Next actionables (priority order)

1. **Split panes → #7** (interactive). Foundation in place (pane-keyed worker +
   resize + cell metrics). App holds N panes (adapter+snapshot+`WorkerCmd` sender
   each) + `bongterm-mux::InMemoryMux` layout + active pane; one worker per pane via
   `run_with((pane_id, shell))`; `Message` carries a pane id; `view` lays panes per
   mux `Rect`; split-H/V + focus-next keybindings; per-pane cols/rows from rect ×
   cell metrics. See `handoff.md` for the plan.
2. **Resource dashboard → #17** (interactive). Worker now has `child.pid`; surface
   it so `bongterm-ledger` samples the pane process tree. Build
   `CurrentProcessSampler::register_pid` **with** this wiring (per-pane contract is
   integration-defined). Then dashboard view-model + app panel.
3. **#6 strict-pass**: suppress repaints when the visible grid is unchanged (cleanest
   in the worker if wezterm `Terminal: Send`); + a `cmd.exe` idle measurement to
   confirm the pwsh-animation hypothesis.
4. **Measure the renderer-dependent gates** (need GPU/display): #4 cold-start, #5
   full-app RSS + DXGI VRAM, #2/#3 keystroke-to-glyph p99 + throughput. Deferred bg
   quad pass (cell backgrounds + reverse + quad cursor) rides on `monospace_cell_size`.
5. **Confirm CI for real** — push/PR so `ci.yml` + `nightly.yml` run on
   `windows-latest`. Local green ≠ CI green; the Phase-1 exit needs 7 green nightlies.

### Key known issues / deferred items

- **`ci.yml` trigger mismatch** — triggers on `push:[main]`+PRs but the working branch is `master`, so CI has never executed. Decide: retarget the trigger or rename the branch. Until then "green" means *local repro only*.
- **rustfmt nightly-vs-stable drift** — `rustfmt.toml` declares nightly-only opts (`imports_granularity`, `group_imports`) ignored by the stable fmt gate. Code is stable-formatted (CI passes); a *nightly* `cargo fmt` may re-introduce diffs. Fix permanently via a pinned-nightly fmt job or by dropping the two opts.
- **wgpu workspace pin** bumped to `"27"` per ADR-008; glyphon replaced by cryoglyph
- **`cargo xtask doctor`**: 2 FAIL (`cl.exe` + `signtool.exe`) — Phase 5 prerequisites, not Phase 0/1 blocking
- **CJK IME round-trip** — harness written (`cargo run -p s3b-ime-composition`); live test deferred to Phase 5 gate §6.1 #21

### How to resume work

1. Read this file to find `[next]`
2. For context on the next task: check the column above for the right artifact
3. Dispatch subagent per `superpowers:subagent-driven-development`
4. Remove completed tasks in-place; move `[next]` to the following task

---

## Overall Journey: Empty Repo → Shipped MVP-0

```
Phase 0 (Scaffold + Spikes) ✅ COMPLETE — v0.0.4-phase0-exit
  │  20 crates, port traits, surface types, buffer pool, CI skeleton, Wave 0 ADRs
  ▼
Phase 1 (Usable Terminal) ← RE-PLAN REQUIRED FIRST
  │  UX Contract → real ConPTY → wgpu renderer → panes/tabs → shell integration
  │  → SQLite storage → resource dashboard
  │  Exit: §6.1 P0 gates #1,#4-8,#17,#28,#29 green 7 consecutive nightlies
  ▼
Phase 2 (Agent Observability)
  │  Claude Code + Codex adapters → transcript → file-change tracker
  │  → approval queue UI → replay
  │  Exit: §6.1 P0 gates #15,#24 green
  ▼
Phase 3 (Developer UX)
  │  Cmd-K (Claude Code subprocess) → explainer → smart history
  │  → snippets → background jobs → clickable patterns
  │  Exit: §6.1 P0 gates #9-14 green
  ▼
Phase 4 (MCP + Secrets + Security)
  │  MCP one-process-per-server → Context Optimizer v1
  │  → DPAPI vault → redaction → dangerous-command policy
  │  Exit: §6.1 P0 gates #16,#19,#23,#31 green + threat-model review
  ▼
Phase 5 (Hardening + Release Prep)
  │  UIA accessibility → IME → DPI → signed MSIX
  │  → parser fuzz CI → diagnostics → Wave 1 spikes resolved
  │  Exit: §6.1 P0 gates #18,#20,#21,#25,#26,#30 green + clean-VM install smoke
  ▼
Phase 6 (Dogfood → Public)
  │  Stage A: 30 working days solo dogfood (BongT as default terminal)
  │  Stage B: 3-5 trusted users, 14 days, signed dev-channel MSIX
  │  Brand review → trademark search → repo public flip → GitHub release
  └─ v0.1.0-mvp0 SHIPPED
```

**Gate rule:** All 25 P0 acceptance gates must be green for 7 consecutive nightly CI runs before public release.

**Reference hardware for all performance gates:** Ryzen 5 7535HS / 16 GB / RTX 2050 4 GB VRAM / Win11 24H2.

**Spec:** `docs/superpowers/specs/2026-05-27-bongt-mvp0-design.md`
**Plan (Phase 0 TDD detail):** `docs/superpowers/plans/2026-05-27-bongt-mvp0.md`
**ADR directory:** `docs/adr/`

---

## PHASE 1 — Usable Terminal *(outline; re-plan before execution)*

> Phase 1 re-plan completed: `docs/superpowers/plans/2026-05-28-bongt-phase1.md`.
>
> Context for the re-plan: ADR-005 (Iced Shader widget), ADR-003 (wgpu latency confirmed), ADR-004 (atlas eviction), ADR-007 (wezterm-term API surface at `crates/bongterm-term/src/adapter.rs`).

Gates this phase satisfies: spec §6.1 #1, #4, #5, #6, #7, #8, #17, #28, #29.

**Prerequisite — UX Contract artifacts under `docs/ux/`** (spec §9):

**Implementation outline:**

- [block] 1.exit Phase 1 exit gate: §6.1 #1, #4-8, #17, #28, #29 green 7 consecutive nightlies *(blocked on the integration gates below + 7 nightly runs)*
  - **Fully DONE this session** (commits `b81eaf0`→`2e0947e`, green locally, wired into `nightly.yml`): **#1** shell-smoke (real ConPTY→parser→snapshot per profile; PASS CMD/WinPS/PS7/SSH, skip-log Git Bash/WSL), **#8** blocks (fixture corpus + p99 500 ns ≤5 ms), **#28** settings (backup+SafeMode+migration built), **#29** storage recovery (torn/checksum/corrupt-DB). See `docs/phase1-exit-gates.md`.
  - **PARTIAL — #5-RSS** wired as a *headless engine-core lower-bound tripwire* only (real `CurrentProcessSampler`; 9.8 MB) — it does **not** spin up the window/wgpu/render loop, so it does **not** verify the full-app 120 MB budget (renderer-dominated). Does **not** count as #5 done.
  - **Remaining = blocked on renderer/subsystem integration** (do NOT fake green — all need the real subsystems wired into `bongterm-app` + a GPU/display + a human visual check): **#4** cold-start-to-first-frame, **#5** full-app RSS + VRAM, **#6** idle CPU (render loop), **#7** split panes (mux unwired), **#17** dashboard attribution (ledger unwired; also `CurrentProcessSampler::register_pid` is documented but unimplemented — but its per-pane contract is set by the app integration, so build it *with* that wiring, not in isolation). This is the SHIP-READINESS "wire the real renderer + re-integrate subsystems" step — its own session, ideally interactive with a human running the GUI.
  - ⚠ Gate criteria are canonical in spec §6.1; the set {1,4,5,6,7,8,17,28,29} is **not** "perf gates" (#2/#3 are the renderer-perf gates, deliberately excluded from Phase 1). §6.1 **#2/#3/#27** are in no phase exit set — land #2/#3 when the renderer is wired, #27 at release review.
- 1.replan **Invoke `superpowers:writing-plans`** for Phase 2

---

## PHASE 2 — Agent Observability

> Phase 2 re-plan completed: `docs/superpowers/plans/2026-05-29-bongt-phase2.md` (17 tasks). AnythingLLM `engineer` workspace consulted.

Gates: spec §6.1 #15, #24.

> **All Phase 2 implementation tasks complete** (2.A.0–2.C.3c + 2.D.1). Commits `5481a30`→`662e31b`. See `docs/codex/phase-status.md` for the per-task table.

- 2.exit *(code done — awaiting operational green ×7 nightlies)* — gates #15 + #24 GREEN locally and wired into `.github/workflows/nightly.yml`. Exit also requires the Phase 1 gates (`1.exit`, still pending) for a fully green nightly.

---

## PHASE 3 — Developer UX

> Phase 3 re-plan completed: `docs/superpowers/plans/2026-05-29-bongt-phase3.md` (21 tasks). AnythingLLM `engineer` workspace consulted.

Gates: spec §6.1 #9, #10, #11, #12, #13, #14.

- [next] 3.B.1 `bongterm-devassist::history` smart filters `cwd:` `branch:` `agent:` `exit:` `time:` `shell:` `duration:`
- 3.B.2 Frecency index in SQLite
- 3.B.3 Ctrl+R smart history + palette integration
- 3.C.1 `bongterm-devassist::snippets` JSON5 library with `${param:name}` placeholders
- 3.C.2 Parameter prompt UI before run
- 3.C.3 Snippet scope: workspace + global
- 3.D.1 `bongterm-devassist::jobs` background pane execution
- 3.D.2 Desktop toast on completion/failure (winrt Notifications API)
- 3.D.3 Job list panel
- 3.E.1 `bongterm-devassist::patterns` matchers for Node/Python/Rust/.NET/TS
- 3.E.2 Clickable file:line spans (overlay only, no scrollback mutation)
- 3.E.3 URL detection + OSC 8 hyperlink rendering
- 3.exit Phase 3 exit gate: §6.1 #9-14 green
- 3.replan **Invoke `superpowers:writing-plans`** for Phase 4

---

## PHASE 4 — MCP, Secrets, Security

> Phase 4 re-plan completed: `docs/superpowers/plans/2026-05-29-bongt-phase4.md` (23 tasks). AnythingLLM `engineer` workspace consulted.

Gates: spec §6.1 #16, #19, #23, #31.

- 4.A.1 `bongterm-mcp::Supervisor` real impl (1 proc / server / workspace)
- 4.A.2 JobObject caps via `bongterm-process-control`
- 4.A.3 MCP manual JSON config import + schema validation
- 4.A.4 No `npx -y` policy + `forbidden-install-policy` test
- 4.A.5 Idle shutdown only when no active agent attached
- 4.A.6 Health check (30s) + RSS sample (1-2s) + restart-with-backoff
- 4.B.1 Context Optimizer v1: per-agent tool allowlist + token-budget preview
- 4.B.2 Temporary scoped MCP config generation for supporting agents
- 4.B.3 Unavailable label for non-supporting agents
- 4.C.1 `bongterm-vault-windows` DPAPI / Cred Mgr `SecretStore` impl
- 4.C.2 `.env` import flow
- 4.C.3 Vault-backed env mode at spawn (no plaintext on disk)
- 4.C.4 Launch-time disclosure modal
- 4.D.1 `bongterm-security::Redactor` corpus (AWS / GitHub PAT / OpenAI / Anthropic / JWT / SSH key / high-entropy)
- 4.D.2 `xtask secret-leak-corpus` real impl
- 4.D.3 Telemetry redaction preview UI before opt-in export
- 4.E.1 Dangerous-command pattern matcher (`git push --force`, `rm -rf`, `kubectl delete`, `terraform destroy`)
- 4.E.2 Workspace trust prompt for newly opened repos
- 4.E.3 Production safety mode UI
- 4.exit Phase 4 exit gate: §6.1 #16, #19, #23, #31 green + threat-model review
- 4.replan **Invoke `superpowers:writing-plans`** for Phase 5

---

## PHASE 5 — Hardening + Release Preparation

> Phase 5 re-plan completed: `docs/superpowers/plans/2026-05-29-bongt-phase5.md` (41 tasks). AnythingLLM `engineer` workspace consulted.

Gates: spec §6.1 #18, #20, #21, #25, #26, #30.

- 5.A.1 UIA provider over BongT terminal surface (Narrator reads active text / scrollback / blocks / tabs / panes / main controls)
- 5.A.2 IME composition wired to ADR-005/006 shape
- 5.A.3 Per-monitor DPI v2 + live DPI changes
- 5.B.1 MSIX manifest in `packaging/msix/`
- 5.B.2 `xtask package-msix` real impl
- 5.B.3 Code-signing cert provisioning (OV first, EV evaluation ADR)
- 5.B.4 Install/upgrade/uninstall smoke on clean Windows VM
- 5.B.5 SmartScreen runbook `docs/runbook/smartscreen.md`
- 5.C.1 Parser fuzzing wired into nightly CI with pinned nightly toolchain (`docs/runbook/fuzzing.md`)
- 5.C.2 Defender real-time smoke nightly
- 5.C.3 Forbidden-abstraction checks → runtime process-tree checks
- 5.C.4 Renderer device-loss simulated test (DXGI device-removed)
- 5.C.5 Crash-recovery suite wired (pane panic / renderer panic / MCP crash loop / SQLite busy / sidecar torn-write / disk quota)
- 5.D.1 Diagnostic export flow with redaction preview
- 5.D.2 Telemetry consent flow (off by default)
- 5.D.3 `bongterm-diagnostics` minidump capture full impl
- 5.E.1 **S5** Claude Code non-interactive output reliability across last 3 versions → ADR (Wave 1)
- 5.E.2 **S6** Codex CLI auth flow end-to-end → ADR (Wave 1)
- 5.E.3 **S7** Defender + EDR-friendly process supervision smoke → ADR (Wave 1) + security whitepaper
- 5.E.4 **S8** Prompt-injection corpus expanded → ADR (Wave 1)
- 5.F.1 SBOM tooling decision (cargo-cyclonedx vs custom) + production impl
- 5.F.2 Provenance attestation (`attestation.intoto.jsonl`)
- 5.F.3 `known-issues.md` published
- 5.F.4 Rollback plan documented in `docs/runbook/release.md`
- 5.exit Phase 5 exit gate: §6.1 #18, #20, #21, #25, #26, #30 green + clean-VM signing + install smoke green
- 5.replan **Invoke `superpowers:writing-plans`** for Phase 6

---

## PHASE 6 — Dogfood + Public Release

> Phase 6 re-plan completed: `docs/superpowers/plans/2026-05-29-bongt-phase6.md` (24 tasks). AnythingLLM `engineer` workspace consulted.

Gates: spec §6.1 #22 + §6.6 ship-when checklist.

- 6.A.1 Begin Stage A: BongT as default terminal; daily log in `docs/dogfood/<date>.md`
- 6.A.2 Stage A workload minimums (per spec §6.2): ≥1 long-running cmd/wk, ≥1 explainer use/wk, ≥1 Cmd-K use/wk, ≥1 shell switch/wk, ≥1 agent run/working-day, ≥1 MCP session/wk if MCP shipped, ≥1 crash drill/wk
- 6.A.3 Stage A exit: 30 working days; zero P0/P1 defects; zero confirmed secret leaks
- 6.B.1 Recruit Stage B users (r/rust, r/PowerShell, r/commandline, ex-colleagues) — 3-5 people / 14 days
- 6.B.2 Issue signed dev-channel MSIX + private feedback channel (Discord/Matrix)
- 6.B.3 Aggregate findings; no public-facing defect
- 6.C.1 Trademark search (USPTO + EUIPO + Indian TM DB + GitHub/npm/crates/domain)
- 6.C.2 Brand-perception review (the "bong" connotation) → `docs/adr/0002-product-name.md` decision
- 6.D.1 Repo public flip
- 6.D.2 SmartScreen warm-up plan executed
- 6.D.3 SECURITY.md inbox monitored
- 6.D.4 GitHub release `v0.1.0-mvp0` with full artifact set (signed MSIX + cert + sha256 + checksums.txt + checksums.txt.sig + attestation.intoto.jsonl + THIRD_PARTY_NOTICES.md + sbom.cdx.json + benchmark-report.md + CHANGELOG.md + known-issues.md + SECURITY.md + INSTALL.md)
- 6.D.5 Landing-page copy (spec §19.3)
- 6.exit `v0.1.0-mvp0` shipped
- 6.replan **Invoke `superpowers:writing-plans`** for `0.2.0` (v1: worktrees, attachments, devcontainer, branch graph, replay editor, cross-shell translator, HTTP/REST pane)

---

*End of orca.md. Tasks are removed in-place when complete. Git history is the audit trail.*
