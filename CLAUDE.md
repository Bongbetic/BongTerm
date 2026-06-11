# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Session start protocol (do this first, every session)

1. **Activate caveman ultra mode.** Invoke the `caveman:caveman` skill with arg `ultra` before any other action. Persist for the whole session. Exceptions per skill rules: code, commits, PRs, security warnings, irreversible-action confirms — write normal.
2. **Load project context from the knowledge graph.** If `graphify-out/graph_report.md` exists at the repo root, Read it before responding to the user's first substantive request. It's the canonical project-state snapshot. If absent, note that and continue.
3. **Load the task plan.** If `orca.md` exists at the repo root, Read it. It is the ordered, end-to-end task list orchestrating work until ship. Treat it as the single source of truth for *what to do next*. Each completed task is struck from the list — when you finish one, edit `orca.md` to remove it. If `orca.md` is absent, the planning phase has not concluded yet — see "Planning phase" below.

Steps 2 and 3 are reads, not writes — do them with the Read tool, not by spawning agents.

## Handoff documents

Always write `handoff.md` to the **repo root** (`D:\Programming\Bongbetic\BongT\handoff.md`), never to a temp dir, `%TEMP%`, or any path outside the workspace. A handoff written outside the workspace cannot be found by the next session's resume protocol and is effectively lost. If a prior handoff was written elsewhere, relocate it into the workspace.

## Planning phase (before `orca.md` exists)

No implementation work begins until the spec + plan are written. Planning uses the **`superpowers:brainstorming`** skill for spec design and the **`superpowers:writing-plans`** skill for the resulting plan. When the user requests planning, requirements work, or feature design, invoke `superpowers:brainstorming` before any other creative work.

`orca.md` shall be created *only after* the planning phase concludes. It must contain the ordered task list covering everything required to ship the application. Tasks are removed from the file as they complete (in-place edit, not append-only). Do not create `orca.md` speculatively or with placeholder tasks.

## Repository status

In implementation. Phase 0 complete (`v0.0.4-phase0-exit`): Cargo workspace with 20 product crates + `xtask` + 5 spike harnesses, port traits + mocks + conformance tests, CI skeleton, ADRs 0003–0007 Accepted. Phase 1 (Usable Terminal) in progress — see `orca.md` for `[next]`. The repo also contains the design artifacts:

- `docs/PRD/bongterm_prd_v7.md` — authoritative product + engineering spec ("Critical Analysis Resolved", 1063 lines, §0–§23). It supersedes all earlier drafts; all architecture, scope, and acceptance criteria flow from this document.
- `docs/PRD/bongterm_v7_resolution_matrix.md` — maps the critical-analysis resolutions into v7; use it to trace how earlier-draft requirements landed in v7.
- `docs/PRD/bongterm_prd_v6_revised.md` — superseded predecessor (3336 lines), "Enforcement-Hardened". Historical reference only; v7 wins on any conflict.
- `Research/BongT-Research_CPT.md` and `Research/BongT-Research_GT.md` — market/competitive research backing the PRD.

> **PRD numbering note.** All `PRD §N` citations in this file refer to **v7 §0–§23**: Thesis §1, Scope §2, Team §3, Architecture Strategy §4, System Architecture §5, Performance §6, Resource Governance §7, Agents §8, Worktrees §9, Shell/Blocks §10, UX §11, Dev features §12, Plugins §13, MCP §14, Accessibility §15, OS matrix §16, CI/Gates §17, Security/Legal/Distribution §18, Business §19, Acceptance §20, Risks §21, Definition of Done §22. The numeric P0 acceptance **gates** (`#1`…`#31`) referenced in `orca.md` and the phase plans come from the canonical design spec `docs/superpowers/specs/2026-05-27-bongt-mvp0-design.md` §6.1, not from PRD v7 §20 — keep those two numbering systems distinct.

Build/test via the Cargo workspace: `cargo build`, `cargo test`, `cargo clippy`, `cargo fmt`, and project tasks under `cargo xtask` (e.g. `cargo xtask doctor`). Run plans live under `docs/superpowers/plans/`. The PRD prescribes the stack (see "Stack" below).

## Product in one line

BongTerm: native Windows-first terminal that runs CLI coding agents in parallel Git worktrees with observable, bounded, auditable execution. PRD §1.1.

## Hard non-goals (PRD §2.2 out-of-scope + §4.2 forbidden techniques — release blockers if violated)

- **No Electron / Chromium / WebView** in the terminal hot path.
- **No OS-bypass techniques**: no DLL injection into children, no ConPTY bypass, no undocumented `ntdll` syscalls, no kernel-mode drivers, no global keyboard hooks, no process hollowing, no direct GPU-driver access.
- **No Node-style extension host** in v1. WASM-first for plugins; high-risk adapters out-of-process under JobObject limits.
- **No auto-run** of destructive shell/Git/Docker/k8s/Terraform/filesystem/MCP actions without explicit approval.
- **No symlinking** of `node_modules` or mutable dep dirs across worktrees by default.
- **No claim of mid-session agent steering** unless the upstream CLI exposes supported IPC/API (PRD §8.2–§8.3). Mark unsupported steering as unavailable; never silently simulate.
- **No MCP tool-schema pruning treated as RAM reduction** — that saves tokens, not resident memory (PRD §7.3).

## Architectural contract

PRD §4.3 (SOLID) and §6 (performance budgets; the zero-alloc hot path is BongTerm's own hot-path engineering rule, budgets in §6.2) are enforceable. Anything touching terminal core, renderer, agents, MCP, security, or storage must satisfy both.

**Module ownership matrix is binding** (PRD §4.3 SOLID SRP + §5 system architecture). Do not cross these lines:

| Module | Owns | Forbidden |
|---|---|---|
| `terminal_core` | ConPTY lifecycle, VT/ANSI parser, grid, scrollback, input dispatch | Agent decisions, MCP perms, UI menus, Git polling, cloud calls |
| `renderer` | Glyph atlas, dirty regions, frame pacing, draw | Command semantics, agent/MCP state, policy |
| `agents` | Agent profiles, lifecycle, transcript, file-change attribution, replay | Parser, MCP process internals, direct secret storage |
| `mcp` | Registry, transport adapters, tool routing, process governance, audit | Agent UI, renderer, Git reconciliation |
| `worktrees` | Lock-aware worktree lifecycle | Port allocation, DB branching, renderer |
| `environment_isolation` | Ports, env files, Docker names, temp/cache | Git branch truth, renderer, UI layout |
| `security` | Policy engine, approvals, redaction, secret access, workspace trust | Renderer, parser, agent UX |
| `storage` | SQLite schema, migrations, append-only chunks, resource ledger | Business policy, UI decisions |
| `ui` | Presentation, accessibility, gestures | Direct process spawn, direct secret reads, direct Git mutation |

**Required style**: ports-and-adapters / clean architecture for non-rendering business logic. Domain depends on port interfaces; infrastructure adapters implement them; UI never directly mutates terminal grid / Git / MCP / agent / secret state. Hot path never depends on agent, MCP, plugin, analytics, cloud, settings UI, or Markdown modules.

**SOLID in Rust-first core**: closed enums with exhaustive `match` where the set is bounded (parser states, risk classifications, command kinds). Traits/registries only where the implementor set is genuinely open (agents, MCP transports, renderer backends, exporters, policy providers). Don't reach for dynamic dispatch when an exhaustive match works.

## Terminal hot-path rules (PRD §6 performance; budgets §6.2)

The pipeline `ConPTY bytes → VT/ANSI parser → grid mutation → dirty region → render queue → D2D/D3D draw` is real-time-ish. In the hot path:

- No sync disk I/O. No network. No agent/MCP calls. No large JSON parsing. No blocking Git/search/index.
- Reusable ring/slab buffers own ConPTY bytes; parser consumes slices, doesn't eagerly own strings.
- In-place parsing over byte slices; SIMD scanning for control bytes (ESC, BEL, CSI, OSC terminators, CR/LF, UTF-8 boundaries).
- Steady-state typing/scrolling/high-output rendering: zero or near-zero heap allocations per frame. Unavoidable allocations are pooled or arena-scoped.
- Dirty-region rendering only — never full-screen redraw for localized changes.
- Backpressure: transcript writer / search indexer / agent observer falling behind degrades gracefully; no unbounded queues.
- Inactive tabs do not render continuously. Scrollback is chunked, append-only, materialized on demand.

Hot-path anti-patterns to reject in review: converting every ConPTY read into an owned string before parsing; copying through parser→grid→transcript→renderer as separate full payloads; running syntax highlighting / Git status / MCP checks / AI context inline.

## Security contract (PRD §18 security + §14.1 vault references — binding)

1. **All agent-ingested content is untrusted.** Terminal output, files, diffs, logs, MCP results, attachments may carry prompt-injection payloads. Authority is granted by explicit policy, never inferred from ingested content.
2. **Default deny, least privilege.** Agents/MCP/tools get only the caps, tools, and secrets explicitly mapped to the task.
3. **Late, scoped secret resolution.** Secrets become plaintext only in memory, at process-spawn time, only for the authorized consumer.
4. **Visible authority.** User can always see, pre-launch and during execution, what an agent/tool may do and which data + secret references it received.

**Secrets feature (§14.1 vault references + §18 security)**: configuration holds `${secret:NAME}` / `${env:NAME}` references only. Plaintext secret values in committed config are rejected by schema validation. Vault is DPAPI / Windows Credential Manager-backed, per-user, no cloud. Secrets never appear in argv, URLs, command history, transcripts, scrollback, logs, or exports. Pass via env block to children, never on a command line.

Threat model priorities (BongTerm engineering guidance; PRD §21 risks + §18 security give the v7 baseline, this list is the stricter project contract): indirect prompt injection (highest), supply-chain compromise, secret exfiltration, malicious VT/OSC escapes (OSC 52 clipboard hijack, OSC 8 hyperlink spoof), malicious workspace config (workspace trust required), DoS / resource exhaustion.

## Stack (PRD §4.1 reuse-first + §5 system architecture)

| Layer | Choice |
|---|---|
| Core (parser, grid, scrollback, command blocks, MCP host, resource governance, policy, agent supervisor) | **Rust-first** |
| Windows integration | `windows-rs` + thin C++ interop only where required |
| App host | Native Win32 HWND / Windows App SDK — **never** Chromium/WebView for terminal hot path |
| Terminal backend | Windows ConPTY (`CreatePseudoConsole`, Win10 1809+) |
| Renderer | DirectWrite + Direct2D/Direct3D, with renderer abstraction permitting future wgpu |
| MCP process layer | `bongterm-mcp-host.exe` — shared local host, process pool, JobObject limits, HTTP loopback preferred, stdio bridge only when unavoidable |
| Plugins | WASM-first; out-of-process native adapters for agent/MCP/task integrations only |
| Persistence | SQLite (WAL) for metadata; append-only chunks for terminal output + transcripts; separate resource ledger store |
| IPC | Named pipes / local RPC with explicit backpressure + timeouts |
| Secrets | Windows Credential Manager + DPAPI-backed encrypted vault |
| Installer | Signed MSIX/MSI; winget later |

## Source-of-truth split (PRD §9.2 — Git CLI truth vs SQLite cache; transcripts §5 persistence, §17.3 crash recovery)

- **Git is truth** for repo / worktree / PR state. SQLite copy is reconstructable cache — rebuild from Git if corrupt.
- **Transcripts, command history, resource ledger are local source-of-truth**. Not reconstructable from Git. Must be crash-safe. Recover from append-only chunks; never silently fabricate.

## Execution phasing (project-local orchestration in `orca.md`; scope tiers PRD §2, gates PRD §20 acceptance + design-spec §6.1 — gate-driven, not calendar-driven)

Don't ship features past their gate. Phases:

- **Phase 0** — Foundations: benchmark harness, resource ledger stub, ConPTY host, fuzzed VT/OSC parser, grid, scrollback, renderer skeleton with device-loss recovery. Plus reuse/risk spikes (PRD §21 critical risks + §4.1 reuse-first): OSC ordering per shell, keystroke-to-glyph budget, UIA feasibility, D3D device-loss, MCP shared-host RSS under load, agent steering capability matrix, Rust/C++ interop boundary.
- **Phase 1** — Usable terminal: profiles, settings/themes/keybindings, tabs/panes/layouts, search, palette, workspace restore, shell integration + command blocks (PowerShell, Bash/WSL) with reliability grading, resource dashboard.
- **Phase 2** — Agent observability MVP: launcher, sidebar, transcript, file-change tracking, lifecycle, approvals, replay-with-context.
- **Phase 3** — Parallelism: serialized worktrees, lock detection, Git truth reconciliation, safe cleanup, env isolation (ports, env files, temp/cache, Docker Compose names, collision detection).
- **Phase 4** — MCP governance + secrets: MCP manager v1, Process Governor, Context Optimizer v1, secret vault + env-credential feature (§14.1, §18), redaction, workspace trust, dangerous-command policy.
- **Phase 5** — Hardening + release: UIA accessibility, IME, DPI/multi-monitor, signed MSIX/MSI, opt-in diagnostics, parser fuzzing wired into CI.
- **Phase 6 — Post-MVP** (do not pull forward): Markdown review, Command Lens, database branching, durable session daemon, plugin marketplace, cross-platform ports.

**Session daemon is deferred from MVP** (PRD §2.2 out-of-scope; §2.1 "No detach daemon"). Layouts/working-dirs/commands can be restored; process survival across app restart is not promised in MVP.

## Required CI gates when code lands (PRD §17 CI/CD, Test Matrix, Release Gates)

Wire these as blocking checks at the phase indicated:

- Phase 0: parser fuzzing; allocations-per-MB and copies-per-MB; p99 keystroke-to-glyph latency.
- Phase 1: terminal compatibility matrix; RSS / VRAM / process-tree budgets + attribution.
- Phase 1+: architecture fitness functions, contract tests for every port interface, substitute/mock implementations validated.
- Phase 2: agent supervision + policy-bypass tests.
- Phase 3: worktree safety + environment-isolation tests.
- Phase 4: MCP process-scaling, secret-leak, redaction tests.
- Phase 5: accessibility (Narrator + ≥1 third-party screen reader), IME, D3D device-loss recovery tests.

## Definition of Done checklist (PRD §22) — apply to every feature

1. Acceptance criteria met. 2. Code reviewed. 3. Error states handled. 4. Logs/diagnostics where relevant. 5. Security review for anything touching files/agents/MCP/shell/secrets/Git/worktrees/external tools. 6. Performance measured for both hot-path latency *and* child-process resource growth (the whole tree — BongTerm, ConPTY/conhost, shells, agents, MCP servers, plugins, render surfaces, background workers). 7. Stays within supported Windows user-mode abstractions. 8. UIA accessibility validated (§15). 9. Tests at correct level. 10. Docs updated. 11. SOLID review for core-touching features. 12. Contract tests for new/changed interfaces. 13. Dependency direction, ownership, cancellation, policy paths validated in CI. 14. Threat-model scenarios (§21 risks + §18 security) considered; secrets routed through vault + reference model (§14.1).

## When in doubt

Read the PRD section referenced above. PRD wording overrides any general intuition — e.g. "MCP context optimization ≠ MCP process governance" (§7.3), "Git CLI output is the source of truth, SQLite is cache; worktree attribution is heuristic, not strong sandboxing" (§9.2), "true mid-session agent steering is not a product guarantee" (§8.3).
