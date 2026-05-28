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
| What is BongTerm? | `docs/superpowers/specs/2026-05-27-bongt-mvp0-design.md` (canonical) or `docs/PRD/bongterm_prd_v5.md` (authoritative 3071-line PRD) |
| What was decided architecturally? | `docs/adr/` — 7 ADRs, all Accepted |
| What was built in Phase 0? | `git log --oneline v0.0.1-scaffold..v0.0.4-phase0-exit` |
| What is the Phase 0 TDD plan (full detail)? | `docs/superpowers/plans/2026-05-27-bongt-mvp0.md` |
| What crates exist? | Root `Cargo.toml` `[workspace.members]` — 20 product crates + xtask + 5 spike harnesses |
| What port traits were defined? | `crates/bongterm-*/src/lib.rs` — each crate has trait + mock + conformance tests |
| What did the Wave 0 spikes measure? | `docs/adr/0003-0007` — latency, VRAM, device integration, IME, wezterm API |
| What are the hard constraints? | `CLAUDE.md` §Hard non-goals, §Security contract, §Terminal hot-path rules |
| What CI gates exist? | `.github/workflows/` (skeleton) + `CLAUDE.md` §Required CI gates |
| Memory from past sessions? | `C:\Users\souba\.claude\projects\C--Users-souba-Documents-Projects-BongT\memory\` |

### Phase completion status

| Phase | Status | Tag | Exit condition |
|-------|--------|-----|----------------|
| **Phase 0** Scaffold + Spikes | ✅ **COMPLETE** | `v0.0.4-phase0-exit` | All gates green; ADRs 003–007 Accepted |
| **Phase 1** Usable Terminal | ⏳ Re-plan required before start | — | §6.1 #1,#4-8,#17,#28,#29 green × 7 nightlies |
| **Phase 2** Agent Observability | 📋 Outline only | — | §6.1 #15,#24 green |
| **Phase 3** Developer UX | 📋 Outline only | — | §6.1 #9-14 green |
| **Phase 4** MCP + Secrets + Security | 📋 Outline only | — | §6.1 #16,#19,#23,#31 green + threat-model review |
| **Phase 5** Hardening + Release Prep | 📋 Outline only | — | §6.1 #18,#20,#21,#25,#26,#30 green + clean-VM smoke |
| **Phase 6** Dogfood → Public | 📋 Outline only | — | `v0.1.0-mvp0` shipped |

### Key known issues / deferred items

- **wezterm submodule gitlink** not created — `vendor/wezterm/.gitkeep` removed; fix cmd in `docs/adr/0007-wezterm-submodule.md` § "Fix required before Phase 1.B.3"
- **wgpu workspace pin** must bump from `"0.20"` → `"22"` before Phase 1.C.1 (glyphon 0.6 requires wgpu ≥22; tracked in ADR-003 + ADR-005 Consequences)
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

- 1.A.4b Persist onboarding choices to disk (SettingsWriter port + `FileSettingsProvider::write`)
- [block](wezterm submodule gitlink) 1.B.3 `WezTermAdapter::ingest_bytes` real wiring to wezterm-term *(fix submodule gitlink first — see ADR-007)*
- [next] 1.C.1 `bongterm-render` real wgpu device + swap chain per ADR-005 *(bump wgpu workspace pin to "22" first)*
- 1.C.1 `bongterm-render` real wgpu device + swap chain per ADR-005 *(bump wgpu workspace pin to "22" first)*
- 1.C.2 Shared glyph atlas with LRU eviction per ADR-004
- 1.C.3 Frame pacing controller respecting backpressure
- 1.C.4 Renderer device-loss recovery (DXGI device-removed)
- 1.C.5 VRAM ceiling enforcement
- 1.D.1 Pane + tab model in `bongterm-mux` over vendored `wezterm-mux`
- 1.D.2 Split h/v, resize, focus cycle
- 1.D.3 Layout save/restore (workspace only, no detach daemon)
- 1.E.1 Shell-integration OSC consumer in `bongterm-blocks`
- 1.E.2 Confidence model: High / Medium / Low / Unsupported per shell
- 1.E.3 Block boundary detection + tests against `tests/fixtures/osc/`
- 1.E.4 Block actions: copy / rerun / attach / save snippet
- 1.F.1 Resource dashboard Iced view
- 1.F.2 `bongterm-ledger` 1 Hz sampler (CPU, RSS, IO, handles)
- 1.F.3 VRAM sampling via DXGI `QueryVideoMemoryInfo`
- 1.F.4 Per-process attribution: BongT / shell / conhost / agent / MCP / plugin-zero
- 1.G.1 SQLite WAL + migration runner + `0001_init.sql`
- 1.G.2 Sidecar chunk writer (blake3 + monotonic IDs + retention)
- 1.G.3 Crash recovery scan on startup
- 1.G.4 `xtask cleanup-chunks` real impl
- 1.exit Phase 1 exit gate: §6.1 #1, #4-8, #17, #28, #29 green 7 consecutive nightlies
- 1.replan **Invoke `superpowers:writing-plans`** for Phase 2

---

## PHASE 2 — Agent Observability *(outline; re-plan before execution)*

Gates: spec §6.1 #15, #24.

- 2.A.1 `bongterm-agents::AgentAdapter` production wiring
- 2.A.2 `ClaudeCodeAdapter::discover` (binary, version, auth)
- 2.A.3 `ClaudeCodeAdapter::create_classifier` stateful
- 2.A.4 `CodexCliAdapter::discover` + `create_classifier`
- 2.A.5 `agent_adapter_conformance` passes for both
- 2.B.1 Transcript writer (`TranscriptRepo` impl)
- 2.B.2 File-change tracker via `git status --porcelain=v1`
- 2.B.3 Approval queue UI with explicit `EnforcementLevel` labels
- 2.B.4 Replay with summarized context (`summarize_exit` → re-launch with prefilled prompt)
- 2.C.1 Agent sidebar Iced view
- 2.C.2 Lifecycle controls: stop / kill process tree / restart
- 2.C.3 Prompt-injection corpus seed (≥30 scenarios) + `xtask prompt-injection-corpus` real impl
- 2.exit Phase 2 exit gate: §6.1 #15, #24 green
- 2.replan **Invoke `superpowers:writing-plans`** for Phase 3

---

## PHASE 3 — Developer UX *(outline; re-plan before execution)*

Gates: spec §6.1 #9, #10, #11, #12, #13, #14.

- 3.A.1 `bongterm-devassist::ai` Claude Code subprocess wrapper
- 3.A.2 Cmd-K palette entry, preview-only, explicit Run confirmation
- 3.A.3 Failed-command explainer button on non-zero exit blocks
- 3.A.4 "Claude Code not installed" graceful fallback UI
- 3.B.1 `bongterm-devassist::history` smart filters `cwd:` `branch:` `agent:` `exit:` `time:` `shell:` `duration:`
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

## PHASE 4 — MCP, Secrets, Security *(outline; re-plan before execution)*

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

## PHASE 5 — Hardening + Release Preparation *(outline; re-plan before execution)*

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

## PHASE 6 — Dogfood + Public Release *(outline; re-plan before execution)*

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
