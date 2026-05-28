# BongTerm — Enforcement-Hardened Native Agentic Terminal PRD and Build Guidance

**Document type:** Product Requirements Document + Engineering Build Guidance  
**Product name:** BongTerm  
**Target platform:** Windows 11 primary; Windows 10 / Windows Server compatibility governed by explicit downgrade matrix  
**Primary audience:** Product, engineering, design, QA, security, release, and enterprise review teams  
**Core product category:** Resource-governed native Terminal Development Environment for safe, observable, parallel agent-assisted development  
**Time estimates:** Intentionally omitted  
**Revision:** v6 - enforcement- and MVP-cutline-hardened: adds policy-enforcement classification, MVP-0, JobObject-vs-sandbox clarification, pinned MCP execution, worktree ownership/attribution, OS support matrix, benchmark fixtures, release/update security, retention controls, and secret exposure classes  

---

## 1. Executive Summary

BongTerm is a native Windows developer terminal designed for modern software engineering, agentic development, and safe parallel execution workflows. It combines a high-performance ConPTY-based terminal engine, native Windows rendering, structured command blocks, tmux-style workspace multiplexing, Markdown preview/review, command learning, CLI-agent observability, Git worktree orchestration, MCP management, and strict security controls.

The product must remain a real terminal first. Agent features must never compromise terminal correctness, shell compatibility, startup speed, local-first behavior, or resource usage. BongTerm should support PowerShell, Windows PowerShell, CMD, WSL, Git Bash, MSYS2, Cygwin, SSH, Docker exec shells, and custom shells while providing first-class support for CLI agents such as Claude Code, Codex CLI, OpenCode, Gemini CLI, Aider, and local/custom command-based agents.

The refined product thesis is:

> **BongTerm is a native Windows terminal development environment that lets developers run, observe, isolate, review, and safely merge AI-agent work across parallel Git worktrees.**

BongTerm is not an Electron wrapper, not a generic chat UI, not a browser-based agent GUI, and not an IDE replacement. It is a Windows-first terminal command center with an agent cockpit, structured command intelligence, secure MCP governance, and environment isolation for parallel development.

### 1.1 Strategic Product Pillars

1. **Native terminal correctness and performance**  
   The terminal hot path shall use native Windows APIs and remain isolated from agents, MCP servers, indexing, settings UI, cloud services, and expensive background tasks.

2. **Structured developer workflows**  
   Commands, outputs, logs, diffs, Markdown, tasks, and agent activity shall be represented as reusable, searchable, attachable workflow objects.

3. **Safe agentic execution**  
   CLI agents shall be observable, stoppable, restartable, replayable, transcripted, and constrained only to the degree supported by the selected enforcement layer. The UI shall distinguish advisory warnings from technically enforceable controls.

4. **Parallel agent workspaces**  
   Git worktrees shall allow multiple agent tasks to run concurrently without corrupting the primary working tree.

5. **Environment isolation**  
   Parallel worktrees must not collide on ports, databases, Docker resources, temp directories, dependency caches, or local environment files.

6. **MCP governance and context routing**  
   MCP tools shall be visible, permissioned, auditable, and semantically filtered so agents receive only relevant tools and context.

7. **Local-first security and privacy**  
   BongTerm shall work without accounts or cloud sync. Command history, transcripts, attachments, secrets, and logs remain local by default.


### 1.2 Critical Review Incorporation Summary

This revision incorporates a resource-efficiency-first critique of the prior PRD. The critique is accepted as a hardening input, not as a full product replacement.

Accepted changes:

1. **Quantitative budgets replace vague performance promises.** Terms such as “near-zero CPU,” “imperceptible latency,” and “lower memory than Electron” are replaced with measurable RSS, VRAM, CPU, startup, throughput, and latency targets.
2. **MCP context optimization is separated from process governance.** The former reduces LLM token/context load; the latter reduces local RAM/CPU/process growth. They are separate systems.
3. **Agent steering is downgraded.** BongTerm shall not assume stdin/slash-command injection works for interactive CLIs. MVP scope is observability, lifecycle control, approvals, replay, transcript capture, and restart-with-context. True mid-session steering is allowed only where the upstream tool exposes a supported IPC/API.
4. **Worktrees are treated as useful but leaky.** Worktree creation must be serialized, lock-aware, and backed by collision detection. Dedicated WSL2/sandbox/remote runners are first-class alternatives for high-risk or high-parallel workloads.
5. **Plugin architecture is narrowed.** No Node-style extension host in v1. Low-risk extensions use WASM; high-risk adapters run out of process under JobObject limits.
6. **The MVP is cut down.** Markdown review, Command Lens, database branching, plugin marketplace, deep review workflows, and full session daemon move out of MVP unless explicitly reintroduced after terminal/agent/resource budgets are met.
7. **Resource accounting becomes a differentiator.** BongTerm must show CPU, RSS, VRAM, process count, MCP count, token use, cost estimates, and policy violations per pane, agent, worktree, MCP server, and plugin.

#### v5 hardening additions

This revision (v5) additionally incorporates a security- and platform-hardening review:

1. A consolidated **threat model** with trust boundaries and explicit indirect-prompt-injection defenses (Section 35).
2. **Native Windows platform requirements** treated as real work: UI Automation accessibility, GPU device-loss/TDR recovery, and IME/complex-text input (Section 36).
3. A first-class **secrets and environment-credential feature** for API keys and `.env` files with references-only config, a DPAPI-backed vault, and least-privilege injection (Section 37).
4. An explicit **concurrency, failure, recovery, and versioning** model, including crash reporting and config/format migration (Section 38).
5. A **gate-driven execution plan** with workstreams, de-risking spikes, phase gates, risk burn-down, CI enforcement, and product success metrics (Section 39).
6. Concept corrections: SOLID interpreted for a Rust-first core (closed enums vs open registries), a contract-honoring reading of Liskov substitution, and a corrected source-of-truth principle that distinguishes cache from primary data.

#### v6 enforcement and MVP-cutline additions

This revision (v6) addresses the remaining over-claiming and implementation-risk gaps:

1. A binding **Policy Enforcement Model** distinguishes advisory, cooperative, brokered, OS-enforced, and runner-enforced controls (Section 17.5).
2. Job Objects are clarified as **resource governors, not security sandboxes** (Section 17.6).
3. A smaller **MVP-0** cutline is added to prove the product wedge before committing to the full v1 MVP (Section 30.5).
4. MCP examples now require pinned, integrity-checked execution rather than unpinned `npx -y` launch patterns (Section 20.4).
5. Worktree ownership, mixed-author attribution, ignored-file tracking, and cleanup classification are specified (Section 18.8).
6. Windows 10, Windows Server, RDP/GPU-limited, and WSL2 networking behavior are covered by an explicit OS-support and downgrade matrix (Section 36.7).
7. Reproducible benchmark fixtures are defined so performance budgets can be tested consistently (Section 25.7).
8. Release/update security, storage retention, private sessions, and secret exposure classes are added (Sections 32.5, 38.6, and 37.11).

Revised thesis:

> **BongTerm is a resource-governed terminal cockpit for Windows-first developers who run CLI agents in parallel and need observable, bounded, auditable execution rather than another unbounded agent wrapper.**

### 1.3 Dual Resource Strategy

BongTerm has two independent performance problems. The PRD shall not collapse them into one generic “performance” concern.

| Resource Track | Primary Risk | Correct Control Plane | Explicit Non-Solution | Owner |
|---|---|---|---|---|
| **Terminal hot-path latency** | Slow input, slow scroll, delayed render frames, high CPU under log output | zero-allocation parser, zero-copy ConPTY pipe buffers, SIMD-assisted VT scanning, virtualized scrollback, shared glyph atlas, dirty-region rendering, renderer/I/O thread separation | bypassing ConPTY, injecting into child processes, using undocumented syscalls, or writing kernel/display drivers | Terminal Core + Renderer |
| **Agent/MCP/plugin RAM bloat** | process-tree explosion from panes, agents, MCP servers, plugins, shells, and `npx` wrappers | MCP host/process pool, JobObject memory/CPU limits, per-workspace budgets, admission control, process-tree dashboard, idle lifecycle policies, WASM-first plugins | MCP tool-schema pruning alone; it saves tokens, not resident memory | Resource Governor + Agent/MCP Runtime |

A feature is not accepted merely because BongTerm’s own process remains small. Acceptance must account for the whole child process tree: BongTerm, ConPTY/conhost, shell processes, agents, MCP servers, plugins, render surfaces, and background workers.

---

## 2. Product Vision

### 2.1 Vision Statement

Build the most capable native Windows terminal for serious developers: fast enough to replace Windows Terminal, structured enough to outperform classic terminal emulators, safe enough to supervise CLI agents, and powerful enough to coordinate parallel agent work without losing user control.

### 2.2 Positioning

BongTerm is a Windows-first Terminal Development Environment with:

- native process hosting through ConPTY;
- native rendering through DirectWrite, Direct2D, and Direct3D;
- tmux-style sessions, windows, tabs, panes, detachable layouts, and optional session daemon;
- structured command blocks with metadata, replay, export, and agent handoff;
- Markdown source, preview, split, review, and diff modes;
- attachment-aware context workflows for files, folders, logs, blocks, diffs, screenshots, and context bundles;
- first-class CLI-agent observability, lifecycle control, replay, and enforcement-aware approvals;
- Git worktree orchestration for isolated parallel agent tasks;
- environment isolation for ports, databases, Docker resources, and dependency caches;
- MCP server management, permissions, audit logs, health checks, and semantic tool routing;
- local-first Windows command learning through PowerShell help, CLI help, alias education, and Linux-to-PowerShell analogies.

### 2.3 Differentiation

| Competitor / Category | BongTerm Differentiation |
|---|---|
| Windows Terminal | Adds command blocks, agent cockpit, Markdown review, attachments, worktree orchestration, MCP governance, command learning, and parallel agent safety. |
| Warp | Native Windows-first approach, local-first operation, lower-resource goal, stronger Windows shell compatibility, explicit Git worktree isolation, and MCP governance. |
| T3 Code-style agent GUI | BongTerm is a real terminal with native process hosting, panes, sessions, shell compatibility, terminal telemetry, and local developer workflows. |
| VS Code terminal | Terminal-first UX, lower footprint, structured command model, native workspace multiplexing, and agent observability without requiring an IDE. |
| tmux | Native Windows UI, pane/session model, command blocks, attachments, Markdown review, Windows shell intelligence, and agent/MCP safety features. |
| MobaXterm-style admin tools | Focuses on local and agentic development workflows rather than remote infrastructure administration bundles. |
| Browser/Electron terminal wrappers | Keeps terminal hot path native and avoids Chromium/WebView rendering as the terminal engine. |

### 2.4 Product Wedge and Adoption Validation

The initial adoption wedge shall be narrower than the full product vision:

> **A native Windows terminal that lets developers run CLI coding agents in parallel while seeing the full process tree, transcript, file changes, resource cost, and worktree state.**

The first releaseable product experience must make a developer prefer BongTerm over Windows Terminal, VS Code Terminal, Warp, or WezTerm for at least one concrete workflow:

1. Launch an agent in an isolated worktree.
2. Watch exactly what process tree, files, commands, MCP servers, and resources it uses.
3. Review the resulting diff and transcript.
4. Stop, replay, discard, or merge with clear provenance.

Product validation shall separately measure:

- percentage of dogfood users who set BongTerm as their default terminal;
- percentage of agent tasks completed without falling back to another terminal;
- worktree create-to-merge success rate;
- frequency of resource-budget warnings and collisions;
- zero tolerated confirmed secret leaks in exports/transcripts;
- top three switching reasons captured from users.

Commercial model, pricing, open-core/commercial split, and enterprise packaging remain product decisions, but the PRD shall not imply enterprise readiness until signed installers, SBOMs, local-only operation, audit logs, update controls, and policy lock behavior are implemented.

---

## 3. Goals and Non-Goals

### 3.1 Goals

BongTerm shall:

1. Provide a correct, fast, native Windows terminal built around ConPTY and native rendering.
2. Support PowerShell 7+, Windows PowerShell, CMD, WSL, Git Bash, MSYS2, Cygwin, SSH, Docker exec, and custom shells.
3. Keep the terminal hot path isolated from agents, MCP servers, search indexes, Git polling, settings parsing, networking, and background workers.
4. Provide structured command blocks where shell integration is reliable, with graceful fallback where shell/ConPTY limitations prevent deterministic boundaries.
5. Provide tabs, panes, layouts, workspace restore, command palette, search, and terminal telemetry.
6. Provide CLI-agent observability: launcher profiles, activity timelines, transcripts, current command detection, file-change tracking, enforcement-level-aware approvals, rollback assistance, and replayable exports.
7. Provide safe Git worktree workflows for isolated agent execution, with serialized worktree creation, Git lock handling, stale-state detection, and cleanup safeguards.
8. Provide basic environment isolation for agent worktrees: ports, env files, temp directories, Docker naming, and package-manager-aware dependency policies.
9. Provide MCP management with two distinct layers: **Context Optimizer** for token/schema pruning and **MCP Process Governor** for local process/RSS/CPU governance.
10. Provide local-first security: DPAPI/Windows Credential Manager secrets, secret redaction, dangerous-command approvals, workspace trust, audit logs, and signed installers.
11. Provide measurable performance budgets and CI gates for startup, input latency, RSS, VRAM, ConPTY throughput, high-output rendering, pane scaling, MCP scaling, and plugin scaling.
12. Optimize inside documented, supported Windows user-mode abstractions instead of bypassing ConPTY, Win32, DirectWrite/Direct2D/Direct3D, or OS security boundaries.
13. Enforce zero-allocation / zero-copy discipline in the terminal hot path, including bounded allocations, reusable buffers, SIMD-assisted parsing, and dirty-region rendering.
14. Enforce SOLID-aligned module boundaries so terminal, rendering, agent, MCP, worktree, security, storage, and UI systems can evolve independently without hidden coupling.
15. Remain usable without accounts, cloud sync, AI features, MCP servers, or remote services.

### 3.2 Non-Goals for MVP

BongTerm MVP shall not:

1. Use Electron, Chromium, or WebView as the terminal hot path.
2. Bypass supported Windows abstractions for marginal performance gains, including DLL injection into child processes, hidden-console scraping, undocumented `ntdll` syscall paths, kernel-mode display drivers, global keyboard hooks, process hollowing, or direct GPU-driver access.
3. Replace full IDEs such as Visual Studio, VS Code, Cursor, or JetBrains IDEs.
4. Implement a full code editor.
5. Ship a general plugin marketplace.
6. Ship a Node-style extension host.
7. Bundle third-party agent CLIs or MCP servers without explicit license and redistribution review.
8. Promise true mid-session steering for CLI agents unless the upstream tool exposes supported IPC/API control.
9. Treat MCP tool-schema pruning as a RAM/process reduction mechanism.
10. Treat Git worktrees as strong isolation for all workloads.
11. Auto-symlink `node_modules` or other mutable dependency directories across worktrees.
12. Ship database branching as a required workflow.
13. Ship Markdown review, Command Lens, collaboration, cloud sync, RDP/X11 suites, or a durable detach/reattach daemon as MVP requirements.
14. Auto-run destructive shell, Git, Docker, Kubernetes, Terraform, deployment, filesystem, or MCP actions without explicit approval.


### 3.3 Deferred but Valid Roadmap Items

The following remain valuable but are explicitly post-MVP unless terminal correctness and resource budgets are already met:

- Markdown preview/review and document annotation.
- Command Lens / Windows command learning.
- AI inline command generation.
- Deep AST-aware diff review.
- Multi-agent adversarial review automation.
- Database branch provisioning adapters.
- Plugin marketplace.
- Enterprise SSO/RBAC.
- Cloud/team collaboration.
- Durable session daemon.
- Cross-platform ports beyond Windows/WSL2.

## 4. Target Users and Personas

### 4.1 Primary Persona: Windows Developer

A developer primarily using Windows 11 with PowerShell, WSL, Git, Docker, Node, Python, .NET, SSH, and local development servers.

Needs:

- fast shell startup;
- reliable terminal emulation;
- panes, layouts, and session restore;
- command history and search;
- Git and project awareness;
- Markdown preview/review;
- low resource usage;
- readable, polished UI.

### 4.2 Primary Persona: Agentic Workflow Developer

A developer using Claude Code, Codex CLI, OpenCode, Gemini CLI, Aider, or custom/local coding agents.

Needs:

- agent visibility;
- safe approvals;
- context attachments;
- MCP setup and governance;
- isolated worktrees;
- transcripts and diff review;
- rollback;
- parallel task execution;
- environment collision prevention.

### 4.3 Secondary Persona: Linux-to-Windows Power User

A developer familiar with zsh, tmux, man pages, shell aliases, and Linux workflows who needs strong Windows-native equivalents.

Needs:

- zsh-like autosuggestions;
- syntax highlighting;
- correction;
- autojump/zoxide behavior;
- tmux-like panes/sessions;
- PowerShell learning support;
- Linux-to-PowerShell command mapping.

### 4.4 Secondary Persona: Technical Manager / Reviewer

A technical lead reviewing logs, Markdown docs, build contracts, agent transcripts, diffs, and implementation plans.

Needs:

- Markdown review;
- terminal block export;
- transcript review;
- session replay;
- search/filtering;
- safe context packaging;
- review-ready diff summaries.

### 4.5 Secondary Persona: Enterprise Platform / Security Team

A team responsible for approving developer tools in managed environments.

Needs:

- local-first operation;
- no mandatory cloud account;
- signed installer;
- dependency inventory;
- secret redaction;
- audit logs;
- policy-driven agent/MCP permissions;
- controlled updates;
- safe workspace trust model.

---

## 5. Product Principles

1. **Terminal correctness before visual polish.** Terminal emulation failures are release blockers.
2. **Hot path isolation.** PTY parsing, grid mutation, scrollback, and rendering must be isolated from agents, MCP, search, settings, and cloud features.
3. **Native where it matters.** Terminal I/O, rendering, input, process lifecycle, and shell integration must be native and low-latency.
4. **Respect supported OS boundaries.** BongTerm shall use ConPTY, documented Win32/Windows APIs, DirectWrite/Direct2D/Direct3D, Windows Credential Manager/DPAPI, and Job Objects rather than unsupported hooks, injection, or kernel bypass techniques.
5. **Optimize by doing less work.** Performance improvements shall come from zero-copy buffers, bounded allocations, SIMD-assisted parsing, shared glyph atlases, dirty-region rendering, backpressure, and process governance, not from bypassing security-sensitive OS abstractions.
6. **Local-first by default.** Core usage must work offline.
7. **Visible automation.** Agent actions, MCP calls, file changes, commands, and approvals must be visible and auditable.
8. **Safety over convenience.** Destructive commands, privileged MCP tools, production environments, and external writes require explicit controls.
9. **User-owned configuration.** Profiles, themes, keybindings, safety policies, agents, and MCP definitions are exportable JSON.
10. **Composable context.** Blocks, files, folders, diffs, logs, Markdown, and transcripts can become explicit context bundles.
11. **No hidden bloat.** Background workers must be bounded, observable, cancellable, and suspendable.
12. **Keyboard-first UX.** Every major workflow must be accessible from keyboard shortcuts and the command palette.
13. **Secure by default.** Secrets, PII, tokens, and private keys must be detected, redacted, and permission-scoped.
14. **Right source of truth per data class.** Git is the source of truth for repository/worktree/PR state, and the SQLite copy of that state is a reconstructable cache. Transcripts, command history, and the resource ledger are local source-of-truth and must be crash-safe. See Section 38.4.
15. **SOLID boundaries over convenience coupling.** Each module must have one clear reason to change, expose narrow interfaces, accept substitute implementations, depend on abstractions, and remain extensible without modifying terminal-core hot-path code.


---

## 6. System Architecture

### 6.1 High-Level Architecture

```text
BongTerm Native Windows App
├─ App Shell
│  ├─ Win32 / C++20 / C++/WinRT host
│  ├─ native window lifecycle
│  ├─ custom titlebar
│  ├─ command palette
│  ├─ settings UI
│  ├─ glass/material compositor
│  └─ accessibility bridge
│
├─ Terminal Core
│  ├─ ConPTY session host
│  ├─ process launcher
│  ├─ VT/ANSI parser
│  ├─ terminal grid
│  ├─ scrollback engine
│  ├─ input editor
│  ├─ selection model
│  └─ shell integration bridge
│
├─ Rendering Engine
│  ├─ Direct3D swap chain
│  ├─ Direct2D primitives
│  ├─ DirectWrite text/glyph system
│  ├─ glyph atlas/cache
│  ├─ dirty-region tracker
│  ├─ virtualized scrollback renderer
│  └─ frame pacing controller
│
├─ Workspace Multiplexer
│  ├─ workspaces
│  ├─ sessions
│  ├─ windows
│  ├─ tabs
│  ├─ panes
│  ├─ layouts
│  ├─ optional detach/reattach daemon
│  └─ dashboard grid
│
├─ Developer Layer
│  ├─ command blocks
│  ├─ shell intelligence
│  ├─ syntax correction
│  ├─ autojump index
│  ├─ project detector
│  ├─ task runner
│  ├─ Markdown viewer/reviewer
│  ├─ attachment system
│  ├─ context bundles
│  └─ Command Lens help system
│
├─ Git and Worktree Layer
│  ├─ repository detector
│  ├─ worktree manager
│  ├─ branch/PR tracker
│  ├─ diff monitor
│  ├─ merge/discard/archive flows
│  ├─ Git truth reconciler
│  └─ state conflict detector
│
├─ Environment Isolation Layer
│  ├─ port allocator
│  ├─ env file generator
│  ├─ Docker Compose project isolator
│  ├─ database branch adapter
│  ├─ dependency cache/symlink manager
│  ├─ temp/cache directory allocator
│  └─ collision detector
│
├─ Agent Cockpit
│  ├─ agent profile registry
│  ├─ agent launcher
│  ├─ side chat
│  ├─ agent sidebar
│  ├─ agent timeline
│  ├─ control capability adapters
│  ├─ approval queue
│  ├─ transcript store
│  ├─ diff monitor
│  └─ rollback controls
│
├─ MCP Manager and Gateway
│  ├─ MCP registry/import
│  ├─ server installer
│  ├─ server supervisor
│  ├─ permissions
│  ├─ secrets
│  ├─ health checks
│  ├─ logs
│  ├─ audit trail
│  ├─ semantic tool router
│  └─ context/token budget controller
│
├─ Security and Policy Engine
│  ├─ secret vault
│  ├─ redaction engine
│  ├─ dangerous command guardrails
│  ├─ production safety mode
│  ├─ workspace trust
│  ├─ MCP permission policy
│  ├─ agent permission policy
│  └─ audit log
│
└─ Persistence
   ├─ SQLite metadata store
   ├─ append-only terminal log chunks
   ├─ command history store
   ├─ transcript store
   ├─ JSON settings/themes/profiles
   ├─ encrypted local secret vault
   └─ cache/index storage
```

### 6.1.1 SOLID Architecture Contract

BongTerm shall use SOLID principles as enforceable engineering constraints. The goal is not abstract purity; the goal is to prevent the product from becoming a coupled terminal/agent/MCP monolith that cannot meet resource, security, and reliability budgets.

| SOLID Principle | BongTerm Requirement | Concrete Application |
|---|---|---|
| **Single Responsibility Principle** | Each module shall have one primary reason to change. | `terminal_core` changes for PTY/parser/grid behavior only; `renderer` changes for drawing only; `agents` changes for CLI-agent lifecycle only; `mcp` changes for MCP process/tool governance only; `security` changes for policy/redaction/secret handling only. |
| **Open/Closed Principle** | New shells, renderers, agents, MCP transports, exporters, task providers, and policy checks shall be added through interfaces/registries rather than by editing core switch statements. | Adding a new agent CLI requires an `AgentAdapter` implementation and contract tests, not changes to `TerminalCore`, `Renderer`, or `SecurityPolicyEngine`. |
| **Liskov Substitution Principle** | Any implementation of a BongTerm interface shall satisfy the same behavioral contract as the default implementation. | `ConPtyTerminalSession`, `MockTerminalSession`, and future `RemoteTerminalSession` must honor the same behavioral contract (preconditions not strengthened, postconditions not weakened); a `RemoteTerminalSession` may add states the local one never enters (e.g., disconnected/reconnecting) without violating that contract. A `RendererBackend` implementation must not change selection, scrollback, or cursor behavior. |
| **Interface Segregation Principle** | Interfaces shall be small and role-specific. No module may depend on a broad “god service” for unrelated capabilities. | Agent code may depend on `TranscriptWriter`, `ApprovalGate`, `ResourceMeter`, and `SecretResolver` separately, not on a global `AppContext`. |
| **Dependency Inversion Principle** | High-level policy and orchestration code shall depend on abstractions. Platform details shall implement ports/adapters. | Worktree orchestration depends on `GitProvider`; MCP governance depends on `McpServerSupervisor`; secret resolution depends on `SecretStore`; none directly hard-code `git.exe`, DPAPI, SQLite, or Windows Credential Manager. |

#### SOLID in a Rust-first core

SOLID is applied as a design discipline, not as an OOP mandate. In the Rust-first core (Section 6.2), "Open/Closed" means open extension points where the set of implementors is genuinely open - agents, MCP transports, renderer backends, exporters, task and policy providers added through traits/registries - and **closed enums with exhaustive matching where the set is bounded** (parser states, risk classifications, command kinds). Exhaustive `match` is preferred over dynamic dispatch when it lets the compiler force every case to be handled; registries are not assumed to be the better answer by default. "Liskov Substitution" is interpreted as honoring an interface's behavioral contract, which may include implementation-specific states (such as a remote session's disconnected state), not as identical behavior across implementations.

#### Required architectural style

BongTerm shall use a **ports-and-adapters / clean architecture** structure for non-rendering business logic:

```text
UI / App Shell
    ↓ depends on
Application Services / Use Cases
    ↓ depends on
Domain Models + Port Interfaces
    ↑ implemented by
Infrastructure Adapters
    - Windows ConPTY adapter
    - DirectWrite/Direct2D/Direct3D renderer adapter
    - SQLite metadata adapter
    - Windows Credential Manager / DPAPI adapter
    - Git CLI/libgit adapter
    - MCP stdio/HTTP adapters
    - Agent CLI adapters
```

Dependency rules:

1. Domain models shall not import Windows UI, renderer, SQLite, MCP process, agent process, or network packages.
2. Application services may orchestrate ports but shall not directly call platform APIs.
3. Infrastructure adapters may depend on platform APIs but shall not contain product policy decisions.
4. UI components shall not directly mutate terminal grid state, Git state, MCP state, agent state, or secret state.
5. Terminal hot-path code shall not depend on agent, MCP, plugin, analytics, cloud, settings UI, or Markdown modules.
6. Cross-module communication shall use typed events, bounded queues, or explicit service interfaces with backpressure and timeout behavior.

#### SOLID-driven module boundaries

| Module | Owns | Must Not Own |
|---|---|---|
| `terminal_core` | ConPTY session lifecycle, VT/ANSI parsing, terminal grid, scrollback, input dispatch | Agent decisions, MCP permissions, UI menus, Git polling, cloud calls |
| `renderer` | Glyph atlas, dirty regions, frame pacing, drawing terminal snapshots | Command semantics, agent state, MCP calls, security policy |
| `command_blocks` | Command/output boundary model, block metadata, block persistence references | Shell process control, renderer internals, agent execution |
| `agents` | Agent profiles, launch lifecycle, transcript capture, file-change attribution, replay | Terminal parser, MCP process internals, secret storage implementation |
| `mcp` | MCP registry, transport adapters, tool routing, process governance, tool-call audit | Agent UI, terminal rendering, Git reconciliation |
| `worktrees` | Worktree creation, lock-aware operations, branch/worktree lifecycle | Port allocation, database branching, renderer state |
| `environment_isolation` | Ports, env files, Docker names, temp/cache allocations, optional DB branch adapters | Git branch truth, terminal rendering, UI layout |
| `security` | Policy engine, approvals, redaction, secret access, workspace trust | Renderer, shell parser, agent-specific UX |
| `storage` | SQLite schema, migrations, append-only chunks, resource ledger persistence | Business policy, UI decisions, shell behavior |
| `ui` | Presentation, commands, accessibility, user gestures | Direct process spawning, direct secret reads, direct Git mutation |

#### Interface examples

The following interfaces are required design targets. Names are illustrative, but the separation is mandatory.

```text
TerminalSession
├─ start(profile)
├─ write_input(bytes)
├─ read_events()
├─ resize(cols, rows)
└─ terminate(reason)

RendererBackend
├─ upload_glyphs(font_key, glyphs)
├─ render_frame(snapshot, dirty_regions)
├─ set_vram_budget(bytes)
└─ collect_metrics()

AgentAdapter
├─ detect()
├─ launch(profile, workspace, context_bundle)
├─ observe_events()
├─ request_stop(mode)
├─ replay_with_context(context_bundle)
└─ collect_metrics()

McpTransport
├─ start(server_config)
├─ list_tools()
├─ call_tool(request)
├─ stop(reason)
└─ collect_metrics()

PolicyEvaluator
├─ evaluate_command(command_context)
├─ evaluate_mcp_call(tool_context)
├─ evaluate_agent_action(agent_context)
└─ request_approval(decision)
```

#### Anti-patterns prohibited by the PRD

- A global `AppContext` passed through most modules.
- UI components directly launching agents, MCP servers, shells, or Git commands.
- Terminal parser calling security, MCP, Git, AI, telemetry, or settings UI code.
- Agent adapters writing directly to SQLite without using transcript/resource-ledger ports.
- MCP tools receiving raw secrets rather than scoped resolved credentials.
- Large interface surfaces such as `IWorkspaceService` that mix Git, terminal, MCP, storage, and UI concerns.
- Feature flags that bypass policy evaluation for convenience.
- Hidden background workers without ownership, metrics, cancellation, and resource limits.

#### SOLID acceptance criteria

A feature touching core architecture is acceptable only when:

1. The owning module and reason to change are documented.
2. Public interfaces are narrow, stable, and covered by contract tests.
3. A mock/fake implementation can be substituted in tests without changing production code.
4. Adding a peer implementation does not require modifying the terminal hot path.
5. High-level orchestration depends on interfaces, not concrete Windows, Git, SQLite, MCP, or agent implementations.
6. Resource metrics and cancellation semantics are exposed at the same abstraction boundary as lifecycle control.
7. Security policy is invoked through `PolicyEvaluator` or equivalent, never bypassed by infrastructure adapters.

### 6.2 Recommended Technology Stack

The critique identified C++/WinRT-only architecture as a velocity, safety, and cross-platform-exit risk. BongTerm remains Windows-first, but the recommended stack is revised to reduce unsafe parsing surface and preserve future portability.

| Layer | Revised Recommendation |
|---|---|
| Systems core | **Rust-first** for VT parser, terminal grid, scrollback, command blocks, MCP host, resource governance, policy engine, and agent supervision |
| Windows integration | `windows-rs` / thin C++ interop only where Windows APIs require it |
| App host | Native Win32 HWND / Windows App SDK shell; no Chromium/WebView terminal hot path |
| Terminal backend | Windows ConPTY APIs, with explicit ConPTY overhead tracking and benchmark gates |
| Renderer | DirectWrite + Direct2D/Direct3D on Windows; renderer abstraction should not prevent future wgpu/other backend |
| Parser | BongTerm-owned VT/ANSI/OSC parser; command-block reliability must not depend blindly on ConPTY preserving all ordering invariants |
| MCP process layer | `bongterm-mcp-host.exe`: shared local host, process pool, JobObject limits, HTTP loopback where supported, stdio adapters where unavoidable |
| Plugin sandbox | WASM-first for safe extensions; out-of-process native adapters only for agent/MCP/task integrations; no Node extension host in v1 |
| Persistence | SQLite metadata; append-only chunks for terminal output/transcripts; separate resource ledger store |
| IPC | Named pipes/local RPC with explicit backpressure and timeout behavior |
| Secrets | Windows Credential Manager and DPAPI-backed encrypted vault |
| Installer | MSIX/MSI with signed binaries; winget later |
| Diagnostics | Local-only resource profiler and exportable diagnostic bundle |
| CI | Terminal compatibility, ConPTY regression, high-output, memory, VRAM, MCP/process-scaling, and plugin-sandbox tests |

The app may still expose native Windows UI and feel Windows-first. The core parsing, orchestration, and policy logic should be memory-safe by default.

### 6.3 Terminal Hot Path

```text
ConPTY output bytes
→ VT/ANSI parser
→ terminal grid mutation
→ dirty-region computation
→ render queue
→ DirectWrite/Direct2D/Direct3D draw
```

Requirements:

- No synchronous disk I/O on the hot path.
- No network calls on the hot path.
- No agent/MCP calls on the hot path.
- No large JSON config parsing on the hot path.
- No blocking search/index/Git operations on the hot path.
- PTY I/O processing must be decoupled from font shaping and rendering.
- Inactive tabs must not render continuously.
- Large scrollback must be virtualized.

### 6.3.1 Zero-Allocation / Zero-Copy Hot Path Contract

BongTerm shall treat the terminal hot path as a real-time-ish pipeline. The goal is not to bypass Windows abstractions; the goal is to minimize work inside the supported user-mode path.

#### Required hot-path techniques

- **Single-read buffer ownership:** ConPTY pipe bytes are read into reusable ring/slab buffers owned by Terminal Core. The parser consumes slices; it must not eagerly copy output into strings.
- **In-place parsing:** VT/ANSI/OSC parsing operates over byte slices and emits compact grid mutations/events.
- **SIMD-assisted scanning:** parser fast paths should scan for control bytes, ESC, BEL, CSI introducers, OSC terminators, newline, carriage return, and UTF-8 boundaries in chunks rather than only byte-by-byte.
- **Bounded allocation:** steady-state typing, scrolling, and high-output rendering must have zero or near-zero heap allocations per frame. Any unavoidable allocations must be pooled or arena-scoped.
- **Backpressure:** if renderer, transcript writer, search indexer, or agent observer falls behind, hot-path I/O must degrade gracefully without unbounded queues.
- **Dirty-region rendering:** renderer receives row/cell damage ranges, not full-screen redraw requests.
- **Shared glyph atlas:** panes sharing font, DPI, weight, ligature, and color mode shall reuse atlas resources where safe.
- **Virtualized scrollback:** scrollback storage must be chunked and append-only; visible rows are materialized on demand.
- **Side-channel emission:** command-block events, transcripts, search indexing, and agent observation consume compact event streams off the hot path.

#### Hot-path anti-patterns

- Converting every ConPTY read into a UTF-16/UTF-8 owned string before parsing.
- Copying output through parser buffer → grid buffer → transcript buffer → renderer buffer as separate full payloads.
- Re-rendering entire panes for localized cursor or row changes.
- Running syntax highlighting, Git status, MCP checks, AI context extraction, or transcript compression inline with PTY parsing.
- Allowing UI controls, settings JSON, plugin callbacks, or agent observers to block the parser or renderer.

#### Required microbenchmarks

- allocations per MB of terminal output;
- copies per MB of terminal output;
- parser throughput in MB/s;
- p99 keystroke-to-glyph latency;
- draw calls and dirty cells per frame;
- scrollback materialization latency;
- glyph atlas hit/miss/eviction behavior;
- backpressure behavior under slow transcript/index consumers.

### 6.4 Session Daemon Policy

A durable tmux-style daemon is **deferred from MVP**. The critique correctly identifies daemon-owned ConPTY handles as a debugging and lifecycle risk on Windows.

MVP behavior:

- BongTerm may restore layouts, tabs, working directories, and commands.
- BongTerm does not promise process survival across app restarts.
- Agent and MCP processes are explicitly terminated, preserved, or detached only according to user-visible policy.

Post-MVP daemon requirements, if introduced:

- The daemon must be optional.
- It must expose process tree, ConPTY handles, CPU, RSS, VRAM, open files, and agent/MCP children in diagnostics.
- It must fail closed for agents and MCP servers unless configured otherwise.
- It must support clean shutdown and orphan cleanup.
- It must be covered by crash/reconnect tests before being enabled by default.

## 7. Native Terminal Requirements

### 7.1 Shell Profiles

BongTerm shall support profiles for:

- PowerShell 7+;
- Windows PowerShell;
- CMD;
- WSL distributions;
- Git Bash;
- MSYS2;
- Cygwin;
- SSH;
- Docker exec shells;
- custom executable profiles.

Each profile shall support:

- name;
- icon;
- command line;
- starting directory;
- environment variables;
- tab title;
- theme override;
- safety profile;
- agent permissions;
- MCP permissions;
- shell integration settings;
- scrollback settings;
- font settings.

### 7.2 Terminal Emulation

The terminal shall support:

- ANSI and VT escape sequences;
- truecolor;
- alternate screen buffer;
- bracketed paste;
- mouse reporting;
- OSC 8 hyperlinks;
- OSC command boundary events where supported;
- Unicode and emoji;
- Nerd Font glyphs;
- optional ligatures;
- cursor shape and blinking controls;
- selection and clipboard behavior;
- rectangular selection;
- find/search in scrollback;
- copy as plain text, HTML, Markdown, and command block.

### 7.3 Input Handling

The input system shall support:

- normal terminal input mode;
- enhanced command editor mode where shell-compatible;
- multiline command editing;
- smart paste;
- bracketed paste;
- keybinding customization;
- command correction prompts;
- syntax highlighting overlays;
- autosuggestion ghost text;
- agent control input channels where supported by official agent IPC/API;
- keybinding pass-through control so that application-mode/full-screen TUIs (which may need keys such as F1 or Ctrl-combinations) can receive them, with a configurable resolution policy for conflicts between BongTerm chords and terminal applications.

### 7.4 Terminal Telemetry

BongTerm shall expose local-only terminal diagnostics:

- PTY throughput;
- render frame timing;
- input latency sampling;
- scrollback memory usage;
- dropped/late frames;
- active/inactive pane rendering state;
- agent process resource usage;
- MCP server process resource usage.

Diagnostics shall be opt-in, local, and exportable as redacted bundles.

---

## 8. Visual Design and Windows 11 UI

### 8.1 Design Language

BongTerm shall use a premium Windows 11-native visual language while prioritizing terminal readability. Glassmorphism is allowed only when it does not harm contrast, input latency, or battery/performance constraints.

Supported materials:

- Mica;
- Mica Alt;
- Acrylic;
- transparent titlebar;
- subtle glass cards;
- soft pane separators;
- depth shadows;
- active pane glow;
- configurable blur and opacity.

### 8.2 Surface Guidelines

| Surface | Default Material |
|---|---|
| Main window | Mica Alt |
| Titlebar/tab strip | Mica or Mica Alt |
| Terminal surface | contrast-preserving semi-opaque layer |
| Sidebar | tinted Mica / card hybrid |
| Command palette | Acrylic, performance permitting |
| Context menu | Acrylic, performance permitting |
| Attachment drawer | Acrylic/card hybrid |
| Agent sidebar | Mica/card hybrid |
| Markdown preview | opaque or semi-opaque reading surface |

### 8.3 Readability and Performance Controls

The app shall provide:

- terminal opacity slider;
- blur intensity slider;
- background tint;
- text contrast guardrail;
- auto-dim background;
- disable transparency toggle;
- disable blur on battery saver;
- disable blur for production sessions;
- high contrast mode;
- reduced motion mode;
- automatic material downgrade during high output or low frame rate.

### 8.4 UI Quality Requirements

- Terminal text must remain legible at all supported opacity levels.
- Glass effects must not cause measurable input latency regressions beyond the performance budget.
- Animations must be interruptible and disableable.
- UI must scale on high-DPI and mixed-DPI multi-monitor setups.
- Production Safety Mode must visibly distinguish high-risk sessions.

---

## 9. JSON Settings, Profiles, Themes, and Policies

### 9.1 Configuration Philosophy

Configuration shall be human-editable, version-controllable, schema-validated JSON. Users shall be able to import compatible concepts from Windows Terminal profiles and color schemes.

### 9.2 Configuration File Structure

```json
{
  "$schema": "https://bongterm.dev/schema/settings.json",
  "profiles": [],
  "schemes": [],
  "themes": [],
  "actions": [],
  "keybindings": [],
  "workspaces": [],
  "agents": [],
  "mcpServers": [],
  "mcpPolicies": {},
  "worktreePolicies": {},
  "environmentIsolation": {},
  "attachments": {},
  "security": {},
  "performance": {},
  "accessibility": {}
}
```

### 9.3 Settings UI

The application shall provide:

- visual settings editor;
- raw JSON editor;
- schema validation;
- autocomplete;
- live preview;
- import/export;
- settings diff view;
- per-profile overrides;
- per-workspace overrides;
- reset to defaults;
- policy lock indicators for enterprise-managed settings.

---

## 10. Workspace Multiplexing

### 10.1 Workspace Model

```text
Workspace
├─ Sessions
│  ├─ Windows
│  │  ├─ Tabs
│  │  │  ├─ Panes
│  │  │  │  └─ Terminal / Markdown / Diff / Agent / Task surface
```

Definitions:

- **Workspace:** Project-level container with root directory, profiles, settings, agents, MCP servers, attachments, safety policies, and layouts.
- **Session:** Durable group of windows/tabs/panes.
- **Window:** Native window or logical tmux-like window.
- **Tab:** Horizontal or vertical tab container.
- **Pane:** Split surface hosting terminals, Markdown, diffs, logs, agent output, or task output.

### 10.2 Multiplexer Features

BongTerm shall support:

- horizontal and vertical splits;
- pane resizing;
- pane movement;
- pane zoom;
- pane maximize;
- pane focus cycling;
- pane titles;
- pane process badges;
- tab groups;
- vertical tabs;
- pinned tabs;
- detachable tabs;
- tear-out panes;
- multiple native windows;
- dashboard grid;
- saved layouts;
- workspace templates;
- session restore;
- optional detach/reattach through daemon;
- broadcast input to selected panes;
- synchronized working directory across panes.

### 10.3 Built-In Layout Templates

- Full-stack development;
- frontend/backend split;
- API + database + logs;
- agent implementation workspace;
- agent review workspace;
- Markdown review workspace;
- production SSH safety workspace;
- Docker Compose workspace;
- WSL + Windows hybrid workspace;
- test/watch workspace;
- multi-agent worktree dashboard.

---

## 11. Command Blocks

### 11.1 Block Data Model

Each executed command shall be represented as a command block where shell integration makes this possible.

```text
CommandBlock
├─ id
├─ sessionId
├─ paneId
├─ commandText
├─ shell
├─ workingDirectory
├─ startTime
├─ endTime
├─ duration
├─ exitCode
├─ outputRange
├─ gitBranch
├─ gitWorktree
├─ workspaceId
├─ attachments
├─ agentAssociation
├─ riskClassification
└─ metadata
```

### 11.2 Block Actions

Each block shall support:

- copy command;
- copy output;
- copy command + output;
- copy as Markdown;
- copy as bug report;
- copy as prompt context;
- rerun command;
- rerun in new pane;
- rerun in new tab;
- collapse/expand;
- bookmark;
- annotate;
- attach to side chat;
- attach to agent;
- export block;
- create reusable task;
- create issue template;
- convert to runbook step;
- search within block.

### 11.3 Block Classification

BongTerm shall classify blocks as:

- Git command;
- build command;
- test command;
- lint command;
- package manager command;
- Docker command;
- Kubernetes command;
- Terraform command;
- SSH command;
- database command;
- agent session command;
- failed command;
- long-running command;
- watch process;
- destructive or risky command;
- production-sensitive command.

### 11.4 Shell Integration and OSC Boundary Detection

Shell integration shall provide reliable command boundaries and metadata.

Supported shells:

- PowerShell;
- Bash;
- Zsh;
- Fish;
- Git Bash;
- WSL shells;
- CMD fallback heuristics.

Required events:

```json
{
  "event": "command_start",
  "command": "pnpm test auth",
  "cwd": "C:\\Projects\\App",
  "shell": "pwsh",
  "gitBranch": "feature/auth",
  "gitWorktree": "agent-auth-tests",
  "timestamp": "2026-05-26T18:00:00Z"
}
```

```json
{
  "event": "command_end",
  "exitCode": 1,
  "durationMs": 8421
}
```

Implementation requirements:

- Emit semantic shell markers using OSC-compatible escape sequences where supported.
- Capture command start/end boundaries, exit codes, cwd, shell, branch, and worktree.
- Detect conflicts with Oh My Posh, Starship, custom prompts, PSReadLine, zsh plugins, and user-defined shell hooks.
- Wrap existing hooks instead of overwriting them.
- Provide diagnostics when shell integration is partially broken.
- Fall back to safe heuristics when hooks are unavailable.

---

## 12. Shell Intelligence

### 12.1 Autosuggestions

BongTerm shall provide local deterministic ghost-text autosuggestions.

Sources:

- global command history;
- workspace command history;
- current directory command history;
- Git branch history;
- successful previous commands;
- package scripts;
- Makefile targets;
- Justfile recipes;
- Taskfile targets;
- Docker Compose services;
- Kubernetes contexts;
- CLI agent commands;
- MCP commands/tools;
- pinned commands.

### 12.2 Syntax Highlighting

The input editor shall highlight:

- command;
- cmdlet;
- alias;
- function;
- flag;
- parameter value;
- existing path;
- missing path;
- string;
- variable;
- pipe;
- redirection;
- glob;
- subshell;
- comment;
- dangerous command;
- unknown command;
- suspected secret.

### 12.3 Syntax Correction

BongTerm shall provide suggestion-only correction.

Default behavior:

- never silently run corrected commands;
- require confirmation for risky corrections;
- never auto-correct destructive commands;
- classify correction risk as low, medium, high, or critical.

Example:

```text
Correction available:
gti status → git status

[Tab] Accept  [Esc] Ignore  [Ctrl+Enter] Run original  [F1] Explain
```

### 12.4 Autojump Navigation

BongTerm shall maintain a frecency-ranked local directory index with:

- `j <query>` support;
- command palette jump;
- fuzzy directory search;
- abbreviation matching;
- global directory memory;
- workspace directory memory;
- pinned directories;
- excluded directories;
- Git repo root detection;
- monorepo package detection;
- WSL path support;
- UNC path support;
- directory preview.

---

## 13. Markdown Preview and Review

### 13.1 Markdown Modes

BongTerm shall support:

- source mode;
- rendered preview mode;
- split source/preview mode;
- review mode;
- diff mode;
- presentation mode.

### 13.2 Markdown Support

The renderer shall support:

- GitHub-flavored Markdown;
- headings;
- tables;
- task lists;
- code fences;
- syntax highlighting;
- Mermaid diagrams;
- frontmatter;
- footnotes;
- images;
- relative links;
- local anchors;
- table of contents;
- collapsible headings.

### 13.3 Markdown Review Features

BongTerm shall support:

- inline comments;
- section comments;
- review summary;
- TODO extraction;
- broken link detection;
- image reference validation;
- heading hierarchy validation;
- table formatting checks;
- Mermaid syntax checks;
- frontmatter validation;
- Markdown lint integration;
- attach review to agent;
- export review comments.

---

## 14. Attachment System and Context Bundles

### 14.1 Attachment Entry Points

The UI shall provide visible attachment entry points in:

- terminal input area;
- side chat;
- agent session panel;
- Markdown preview;
- command block menu;
- diff view;
- command palette.

### 14.2 Attachment Types

| Type | Behavior |
|---|---|
| Text/code file | preview, path reference, optional content extraction |
| Folder | tree manifest, filtered file list, optional context bundle |
| Image | preview, optional OCR, metadata stripping |
| PDF | metadata, page preview if supported, path reference |
| Terminal block | command, output, exit code, metadata |
| Git diff | staged/unstaged diff bundle |
| Log file | compressed/indexed preview |
| Screenshot | local image asset with preview |
| Environment report | generated diagnostic context |
| Agent transcript | searchable, attachable transcript slice |
| Worktree summary | branch, files changed, test output, PR state |

### 14.3 Context Bundle Data Model

```text
ContextBundle
├─ files
├─ folders
├─ selected command blocks
├─ Git diff
├─ logs
├─ screenshots
├─ notes
├─ environment metadata
├─ MCP tools included
├─ redaction report
└─ intended agent/tool target
```

### 14.4 Attachment Safety

BongTerm shall:

- detect `.env` files;
- detect private keys;
- detect token-like strings;
- warn on large files;
- warn on binary files;
- redact secrets where configured;
- strip sensitive metadata from images where configured;
- scope attachments per chat/agent/workspace;
- display exactly what context is being passed to an agent;
- avoid hidden uploads or hidden context injection.

---

## 15. Side Chat

### 15.1 Chat Surfaces

BongTerm shall support side chat for:

- workspace chat;
- file chat;
- Markdown chat;
- command block chat;
- diff chat;
- agent chat;
- MCP inspection chat;
- review chat.

### 15.2 Side Chat Requirements

Side chat shall allow:

- attaching current file;
- attaching current block;
- attaching current diff;
- attaching current Markdown preview;
- attaching current worktree summary;
- selecting target agent;
- selecting allowed MCP tools;
- saving prompt templates;
- converting message to terminal command;
- converting message to agent control instruction;
- preserving conversation per workspace;
- exporting conversation;
- searching conversation history.

### 15.3 Inference Backend

Side chat requires a model backend, which must be reconciled with the local-first principle (Section 5, principle 6). BongTerm shall:

- treat side chat as **optional**; all core terminal and observability features work with no model configured;
- support routing side-chat requests through a configured agent CLI/provider or a user-supplied API key/endpoint (including local model endpoints), with any key stored only as a secret reference (Section 37);
- never enable a default cloud backend implicitly; the user chooses and configures the backend;
- apply the same redaction and attachment-disclosure rules as agents (Sections 14.4, 35.5) to anything sent to the side-chat backend.

The side-chat action "convert message to agent control instruction" is constrained by the control capability levels in Section 17.2: where true steering is unsupported, it produces a restart-with-context action rather than a simulated mid-session injection.

---

## 16. CLI-Agent Cockpit

### 16.1 Supported Agents

BongTerm shall support configurable profiles for:

- Claude Code;
- Codex CLI;
- OpenCode;
- Gemini CLI;
- Aider;
- GitHub Copilot CLI where applicable;
- local LLM CLIs;
- custom command-based agents.

### 16.2 Agent Profile Schema

```json
{
  "name": "Claude Code - Safe Review",
  "command": "claude",
  "args": [],
  "workingDirectory": "${workspace.root}",
  "env": {},
  "mcpServers": ["filesystem-readonly", "github-readonly"],
  "attachments": {
    "mode": "manifest",
    "maxFileSizeMb": 5,
    "redactSecrets": true
  },
  "permissions": {
    "allowShell": true,
    "allowFileWrite": true,
    "allowNetwork": false,
    "requireApprovalFor": [
      "git push",
      "rm",
      "deploy",
      "kubectl delete",
      "terraform destroy"
    ]
  }
}
```

### 16.3 Agent Sidebar

The agent sidebar shall show:

- agent name;
- status;
- task summary;
- current command;
- current working directory;
- associated branch/worktree;
- files touched;
- current diff summary;
- active MCP servers/tools;
- pending approvals;
- transcript link;
- runtime duration;
- resource usage;
- controls.

### 16.4 Agent Statuses

Supported statuses:

- idle;
- starting;
- thinking;
- reading files;
- editing files;
- running command;
- calling MCP tool;
- waiting for approval;
- waiting for user input;
- failed;
- stopped;
- completed;
- blocked by policy;
- environment collision detected.

### 16.5 Agent Activity Timeline

BongTerm shall record:

- started session;
- received instruction;
- attached context;
- created worktree;
- read files;
- modified files;
- ran commands;
- called MCP tools;
- requested approval;
- produced summary;
- opened PR;
- completed/failed/stopped.

### 16.6 Agent File-Change Tracking

BongTerm shall display:

- files created;
- files modified;
- files deleted;
- per-file diff;
- agent attribution;
- timestamps;
- revert file;
- revert all agent changes;
- create branch/worktree;
- commit changes;
- open external editor.

---

## 17. Agent Control, Observability, and Replay

### 17.1 Revised Product Position

BongTerm shall not claim universal mid-session steering of CLI agents. Many interactive agents run through TUIs/REPLs that do not reliably accept programmatic stdin submission or interruption. Therefore, MVP scope is:

- observe;
- approve;
- stop;
- restart with summarized context;
- replay from transcript/context bundle;
- enforce policy where the selected enforcement layer supports actual control;
- provide advisory warnings and audit trails where enforcement is not technically possible;
- export diagnostics.

True steering is supported only when the agent exposes a documented IPC/API/SDK capability or a reliable non-interactive mode.

### 17.2 Control Capability Levels

| Level | Capability | MVP Status |
|---|---|---|
| 0 | Observe stdout/stderr, process state, files touched, command blocks | Required |
| 1 | Lifecycle control: start, stop, kill, restart, retry | Required |
| 2 | Approval gates before shell/file/MCP/destructive actions | Required |
| 3 | Replay with context bundle into a new agent run | Required |
| 4 | One-shot non-interactive prompt execution where CLI supports it | Optional |
| 5 | Mid-session steering through official IPC/API/SDK | Post-MVP unless supported by target agent |
| 6 | Unsupported stdin/slash-command injection into TUI agents | Not a product guarantee |

### 17.3 Agent Actions

The UI shall provide:

- start agent from profile;
- stop agent after current visible step where detectable;
- hard kill process tree;
- summarize transcript;
- restart with summary and selected attachments;
- run tests in associated worktree;
- switch to read-only review mode;
- require approval before shell commands where the agent/tool path is cooperative, brokered, OS-enforced, or runner-enforced;
- warn and audit shell commands where only advisory detection is possible;
- require approval before file writes where detectable and enforceable;
- disable selected MCP tools for next run;
- export transcript, diff, command blocks, and resource ledger.

### 17.4 Safety Requirements

BongTerm shall:

- display the control method being used;
- never hide injected control text or policy changes;
- distinguish “requested stop” from “confirmed stopped”;
- block destructive actions only when the configured enforcement layer can technically prevent them; otherwise warn, require visible confirmation where possible, and audit the limitation;
- record agent process tree and child MCP processes;
- preserve transcript and resource ledger even after a forced kill;
- mark unsupported steering features as unavailable, not silently simulated.

### 17.5 Policy Enforcement Model

BongTerm shall classify every policy claim by enforcement level. User-facing copy may use the word **block** only when the configured mechanism can technically prevent the action.

| Enforcement level | Definition | Examples | User-facing claim allowed |
|---|---|---|---|
| **Advisory** | BongTerm detects, warns, records, or recommends, but cannot prevent the action. | Pattern-matched dangerous commands in arbitrary terminal output; post-hoc file diff detection; transcript warnings. | "Warn", "detect", "flag", "audit". Not "block". |
| **Cooperative CLI** | The agent/tool exposes documented hooks, approvals, IPC, SDK, or non-interactive modes that BongTerm can use. | Agent-provided approval callbacks; official MCP permission prompts; documented agent stop/replay API. | "Require approval" only for actions routed through that capability. |
| **BongTerm broker** | The action must pass through a BongTerm-controlled broker before execution. | MCP gateway; shell-command broker; file-write broker; secret resolver; runner launch service. | "Block", "allow", "deny", "approve" for brokered actions. |
| **OS-enforced** | Windows or the execution substrate enforces the restriction. | ACLs, restricted token, AppContainer where feasible, Windows Firewall rules, JobObject kill tree for process control. | "Block" only for the exact OS-enforced capability. |
| **Runner-enforced** | A container, WSL2 runner, VM, sandbox, or remote dev-box enforces the restriction outside the terminal app. | Network-disabled container; disposable WSL2 runner; remote isolated Linux dev box. | "Block" where the runner policy enforces it. |

#### Policy enforcement matrix

| Capability | Advisory | Cooperative CLI | BongTerm broker | OS-enforced | Runner-enforced |
|---|---:|---:|---:|---:|---:|
| Warn on dangerous command | Yes | Yes | Yes | Yes | Yes |
| Block shell command before execution | No | Maybe | Yes | Yes, if shell is constrained | Yes |
| Block file write | No | Maybe | Yes | Yes, via ACL/sandbox | Yes |
| Block network egress | No | No | Partial | Yes, via firewall/AppContainer where practical | Yes |
| Prevent raw secret exposure | No | No | Partial | Partial | Stronger when combined with brokered/ephemeral credentials |
| Kill runaway process tree | Partial | Partial | Yes | Yes, via JobObject/process control | Yes |
| Attribute child process | Partial | Partial | Yes | Yes, where launched under supervised tree | Yes |
| Prevent an arbitrary CLI from ignoring policy | No | No | No unless fully brokered | Partial | Yes, if runner owns execution |

#### Binding wording rule

Any feature, UI label, marketing page, or release note that uses **safe**, **blocked**, **sandboxed**, **permissioned**, or **prevented** must name the enforcement level. For arbitrary unmodified CLI agents running as normal child processes, BongTerm provides observability, warnings, lifecycle control, transcript capture, file-change detection, and rollback assistance; it does not guarantee pre-execution prevention.

### 17.6 Job Objects Are Resource Governors, Not Sandboxes

BongTerm shall use Windows Job Objects for process-tree accounting, termination, CPU controls, memory limits where enforceable, and runaway-process handling. Job Objects shall not be described as a complete security sandbox.

| Control objective | Job Object sufficient? | Required additional mechanism when hard enforcement is needed |
|---|---:|---|
| Kill process tree | Yes | None beyond correct process ownership and tests. |
| CPU/memory cap | Partially | Monitoring, admission control, fail-closed policy, and user-visible exceptions. |
| File read/write restriction | No | ACLs, restricted token, AppContainer, WSL/container/VM/remote runner. |
| Network restriction | No | Windows Firewall/AppContainer/container/VM/remote-runner networking. |
| Secret exfiltration prevention | No | No raw secret injection, scoped/ephemeral tokens, brokered secret access, network policy. |
| Supply-chain compromise containment | No | Version/hash pinning, provenance review, sandbox/runner isolation, least privilege. |

Security documentation and UI shall state that JobObject-capped MCP servers and plugins are resource-bounded, not inherently trustworthy.

## 18. Parallel Agent Workspaces and Git Worktrees

### 18.1 Product Goal

BongTerm shall support Git worktrees as a useful isolation primitive for parallel agent work, while explicitly acknowledging that worktrees are not strong sandboxing. They share Git object storage and can collide through locks, dependency folders, submodules, LFS, package managers, databases, ports, and generated artifacts.

### 18.2 Worktree Creation Flow

```text
Create Agent Worktree
├─ select repository
├─ run preflight checks
│  ├─ clean/dirty state
│  ├─ submodule/LFS detection
│  ├─ package manager detection
│  ├─ existing Git locks
│  ├─ branch naming collision
│  └─ disk-space estimate
├─ acquire BongTerm repository mutex
├─ execute serialized git worktree add
├─ create branch
├─ generate worktree path
├─ apply environment isolation profile
├─ choose target agent
├─ choose Context Optimizer/MCP profile
├─ attach context bundle
└─ launch agent in isolated pane/session
```

### 18.3 Worktree Safety Rules

BongTerm shall:

- serialize `git worktree add`, branch creation, deletion, prune, rebase, merge, and cleanup operations per repository;
- detect `.git/config.lock`, `.git/index.lock`, and `.git/worktrees/*/index.lock` before running Git operations;
- show stale lock warnings with safe remediation guidance;
- avoid background `git status` polling while another Git operation is active;
- treat SQLite state as cache and Git as truth;
- never run `git checkout -- .`, `git clean -fdx`, `git reset --hard`, or worktree delete without explicit confirmation and target path display;
- block destructive cleanup if uncommitted or staged changes exist;
- capture pre-cleanup diff snapshots.

### 18.4 Worktree Dashboard

The dashboard shall show:

- worktree name and physical path;
- branch and base branch;
- associated agent and transcript;
- Git truth status;
- active locks or stale locks;
- package manager and dependency policy;
- files changed;
- test status;
- PR status where configured;
- allocated ports/env/Docker resources;
- resource usage;
- cleanup/merge/discard actions.

### 18.5 Worktree Actions

Each worktree shall support:

- open in pane;
- open in external editor;
- view diff;
- run tests;
- commit changes;
- push branch;
- create PR;
- merge to base;
- rebase onto base;
- archive worktree;
- safe delete with diff snapshot;
- restart agent with transcript summary;
- export worktree diagnostic bundle.

### 18.6 High-Isolation Alternatives

For serious parallel-agent workloads, BongTerm shall support or plan:

- WSL2-based agent runner;
- sandbox VM runner;
- remote Linux dev-box runner over SSH;
- containerized runner profile;
- dedicated machine/runner profile for enterprise use.

These are not merely “deployment options”; they are risk controls when Git worktrees are insufficient.

### 18.7 Cross-Agent Review

Cross-agent adversarial review is post-MVP. The minimal acceptable first version is manual: export Worktree A diff/transcript as read-only context, then launch Agent B in review mode without write permissions.

### 18.8 Worktree Ownership and Attribution Model

BongTerm shall treat every agent worktree as an owned execution surface. Ownership is not a security boundary, but it is required for attribution, cleanup, and user trust.

| Case | Required behavior |
|---|---|
| One agent owns one worktree | Normal mode. The dashboard shows agent, branch, path, base branch, transcript, resource ledger, and current diff. |
| User edits files during agent run | Mark changed files as **mixed-author** unless BongTerm can attribute the edit to the agent process tree. Display the user-edit window and affected files. |
| Two agents target same worktree | Block by default at the BongTerm orchestration layer. Allow only with explicit override and visible mixed-ownership warning. |
| Agent modifies ignored/generated files | Track separately from Git diff using filesystem watchers and known ignore rules. Show tracked, untracked, ignored, generated, and deleted files separately. |
| Long-running dev server writes artifacts | Classify as runtime/generated changes and avoid attributing them as source edits unless user confirms. |
| Cleanup requested | Show target path, tracked diff, untracked files, ignored/generated files, staged files, open processes, and pre-cleanup snapshot before deletion. |
| Merge/rebase requested | Require Git truth reconciliation immediately before action. Abort if locks, stale branch state, mixed ownership, or unresolved generated artifacts are detected. |

#### Attribution sources

BongTerm shall use multiple attribution sources and display confidence:

- Git diff for tracked files;
- filesystem watcher events for untracked/ignored files;
- process-tree association for launched agent/shell processes;
- transcript and command-block timestamps;
- known build-output/generated-file patterns;
- explicit user marks for manual edits.

The UI shall not claim perfect attribution. It shall classify attribution as high, medium, low, or mixed.

## 19. Environment Isolation Layer

### 19.1 Product Goal

Git worktrees isolate source files but not runtime infrastructure. BongTerm shall provide a practical isolation layer that prevents common collisions without pretending to solve every infrastructure problem automatically.

### 19.2 MVP Isolation Scope

MVP shall include:

- per-worktree port assignment;
- per-worktree `.env.agent.local` generation;
- per-worktree temp/cache/log directories;
- Docker Compose project-name isolation;
- collision detection for ports, env files, branch names, Docker resources, and obvious lockfiles;
- warnings for unsupported package-manager or database isolation cases.

### 19.3 Port Allocation

BongTerm shall:

- detect common dev server commands;
- allocate per-worktree ports from a configurable range;
- write assignments to generated env files;
- detect `EADDRINUSE` failures;
- suggest or apply a safe remap;
- show mappings in the worktree dashboard;
- account for the active WSL2 networking mode (NAT versus mirrored) when allocating and forwarding ports for WSL-hosted dev servers, since localhost-forwarding behavior differs between modes.

### 19.4 Environment Files

BongTerm shall support:

- `.env.agent.local` generation;
- inherited env templates;
- secret placeholder references;
- redaction in attachments/exports;
- warning if env files are staged for commit;
- policy to block sending env files to agents.

### 19.5 Database Isolation

Database branching is **post-MVP** and must be framed as an optional adapter with user-visible cost/account requirements.

MVP shall support:

- naming conventions for local test databases;
- local SQLite file copy/isolation;
- user-supplied DB connection profiles;
- warnings before running migrations against shared databases.

Post-MVP adapters may support Neon, Supabase, Dockerized Postgres, and enterprise database branching systems.

### 19.6 Docker Isolation

BongTerm shall support:

- per-worktree Docker Compose project names;
- isolated container names;
- isolated network names;
- isolated volume prefixes;
- cleanup preview before deletion;
- collision detection for ports and volumes.

### 19.7 Dependency and Cache Strategy

BongTerm shall not blindly symlink `node_modules` or mutable dependency directories across worktrees.

Rules:

- pnpm projects may use `enableGlobalVirtualStore` guidance where supported.
- npm/yarn projects default to per-worktree install or package-manager cache reuse.
- symlink strategies require explicit compatibility checks and opt-in.
- native modules, Vite/Vitest, monorepos, submodules, and LFS trigger caution warnings.
- BongTerm shall estimate install cost and disk cost before launching many worktrees.

### 19.8 Collision Detection

BongTerm shall detect and report collisions involving:

- ports;
- databases;
- Docker resources;
- Git locks;
- package lockfiles;
- temp directories;
- generated assets;
- test output directories;
- cache directories;
- branch names;
- PR names.

## 20. MCP Manager, Context Optimizer, and Process Governor

### 20.1 MCP Product Goal

BongTerm shall make MCP usage visible, permissioned, auditable, and resource-bounded. The PRD now separates two concerns that were previously conflated:

1. **Context Optimizer:** reduces LLM token/context load by exposing only relevant MCP tools/schemas to an agent.
2. **MCP Process Governor:** reduces local process/RSS/CPU growth through shared hosting, lifecycle control, and JobObject limits.

### 20.2 MCP Manager UI

The MCP area shall include:

- installed servers;
- project/global/agent-specific MCP profiles;
- transport type: stdio, HTTP, streamable HTTP, remote;
- process owner: shared host, dedicated process, external process;
- permissions;
- health checks;
- logs;
- secrets;
- audit trail;
- tool schema viewer;
- context/token budget preview;
- process/RSS/CPU budget display;
- lifecycle controls;
- risk score.

### 20.3 MCP Process Governor

BongTerm shall provide a managed `bongterm-mcp-host.exe` where feasible.

Requirements:

- one host per user by default;
- shared MCP server process pool where transport allows;
- local HTTP loopback adapter for agents where supported;
- stdio bridge only when unavoidable;
- Win32 JobObject memory and CPU limits per MCP server or child tree;
- idle policy that distinguishes CPU suspension from memory reclamation;
- cold-start latency measurement;
- crash restart policy;
- per-server process tree display;
- hard kill and cleanup action;
- audit log for tool calls and server lifecycle events;
- defined blast radius for the shared host: a host crash restarts servers under the crash-restart policy, in-flight tool calls fail safe (idempotent retry where the tool declares idempotency, surfaced error otherwise), and workspace isolation is preserved by running servers of different trust levels or workspaces in separate child processes within the host rather than sharing one process.

### 20.4 MCP Server Schema

```json
{
  "name": "github-readonly",
  "transport": "stdio",
  "processMode": "managed-stdio-bridge",
  "command": "bongterm-mcp-runner",
  "args": [
    "--package",
    "@modelcontextprotocol/server-github@x.y.z",
    "--integrity",
    "sha256-REPLACE_WITH_PINNED_PACKAGE_INTEGRITY"
  ],
  "env": {
    "GITHUB_PERSONAL_ACCESS_TOKEN": "${secret:github_pat_readonly}"
  },
  "scope": "workspace",
  "resourceLimits": {
    "maxRssMb": 60,
    "maxCpuPercent": 10,
    "idleShutdownSeconds": 900
  },
  "permissions": {
    "network": true,
    "filesystem": false,
    "shell": false,
    "externalWrites": false
  }
}
```

#### MCP package execution rules

The schema example intentionally uses a pinned runner rather than direct `npx -y`. BongTerm shall block or require an explicit high-risk override for:

- unpinned `npx -y` / `pnpm dlx` / `yarn dlx` package execution;
- `curl | sh`, `irm | iex`, or equivalent remote script execution;
- mutable tags such as `latest` for MCP servers or native adapters;
- unsigned installers or binaries without provenance metadata;
- package updates that change declared permissions, transport, or binary hash.

Allowed MCP installation flows shall prefer pinned package versions, integrity hashes, provenance display, permission review, and rollback.

### 20.5 Context Optimizer

The Context Optimizer shall:

- analyze the active task, workspace, agent profile, and context bundle;
- prune irrelevant MCP tool definitions before agent launch;
- show which MCP tools will be exposed;
- estimate token/context overhead;
- support per-agent allowlists and denylists;
- block high-risk tools unless approved;
- hide database tools from frontend-only tasks unless explicitly enabled;
- hide filesystem write tools from read-only review agents;
- record all tool exposure decisions in audit logs.

It must be documented clearly: **Context Optimizer reduces model context/token pressure, not local process/RAM usage.**

### 20.6 MCP Security Requirements

The MCP subsystem shall implement:

- provenance display;
- package/version display;
- hash pinning where possible;
- version pinning;
- update review;
- rollback;
- allowlist/blocklist;
- workspace trust;
- tool schema inspection;
- permission prompts;
- least-privilege configuration;
- secret vault integration;
- secret redaction;
- filesystem sandboxing where possible;
- network restrictions where possible;
- audit logs;
- tool call display;
- human approval for destructive/external-write tools.

### 20.7 MCP Observability

When an agent uses MCP, BongTerm shall show:

- server name;
- process tree;
- transport;
- tool name;
- input summary;
- output summary;
- duration;
- success/failure;
- RSS/CPU at call time;
- permission state;
- redacted sensitive data;
- transcript link.

## 21. Project, Git, PR, and Task Features

### 21.1 Project Detection

BongTerm shall detect:

- Git repository;
- Git worktree;
- monorepo root;
- `package.json`;
- `pnpm-workspace.yaml`;
- `turbo.json`;
- `nx.json`;
- `Cargo.toml`;
- `go.mod`;
- `.sln`;
- `.csproj`;
- `pyproject.toml`;
- `requirements.txt`;
- `Dockerfile`;
- `docker-compose.yml`;
- `Taskfile.yml`;
- `justfile`;
- `Makefile`.

### 21.2 Task Runner

The task runner shall discover and run:

- npm/pnpm/yarn scripts;
- Makefile targets;
- Justfile recipes;
- Taskfile tasks;
- Cargo tasks;
- dotnet build/test commands;
- Docker Compose tasks;
- custom user tasks.

Tasks shall be runnable in:

- current pane;
- new pane;
- new tab;
- background task;
- agent context;
- isolated worktree context.

### 21.3 Git Features

BongTerm shall show:

- branch;
- worktree;
- dirty state;
- ahead/behind;
- staged/unstaged summary;
- latest commit;
- file changes;
- diff preview;
- branch switcher;
- create branch;
- create worktree;
- open PR link;
- copy branch/commit;
- warn on risky Git operations.

### 21.4 PR Integration

BongTerm shall support:

- GitHub CLI/API integration;
- PR creation from agent worktree;
- PR status display;
- remote branch detection;
- reviewer assignment where configured;
- PR template selection;
- test/check status display;
- stale PR metadata refresh.

---

## 22. Command Lens — Windows Command Learning System

### 22.1 Feature Goal

Command Lens shall provide a Windows-native equivalent to Linux `man` workflows with PowerShell-aware help, Windows command docs, examples, aliases, CLI help, and cross-shell learning aids.

### 22.2 Help Sources

Command Lens shall use:

- PowerShell `Get-Help`;
- PowerShell `Get-Command`;
- PowerShell module metadata;
- local CLI `--help` and `/?` output;
- Git help;
- Docker help;
- npm/pnpm/yarn help;
- WSL `man` where available;
- MCP tool schemas;
- agent CLI help output;
- curated offline docs cache.

### 22.3 Command Lens UI

Modes:

- quick popover;
- side manual panel;
- split manual view;
- learn mode;
- examples mode;
- parameters mode;
- troubleshooting mode.

Default shortcut:

- `F1` or `Ctrl+Shift+H` opens help for the current command/token.

### 22.4 Linux-to-PowerShell Analogies

| Linux | PowerShell |
|---|---|
| `ls -la` | `Get-ChildItem -Force` |
| `grep "x" file` | `Select-String "x" file` |
| `ps aux` | `Get-Process` |
| `kill <pid>` | `Stop-Process -Id <pid>` |
| `systemctl status service` | `Get-Service service` |
| `which command` | `Get-Command command` |

---

## 23. Search and Command Palette

### 23.1 Search Surfaces

BongTerm shall search:

- current pane;
- current tab;
- all tabs;
- all sessions;
- command history;
- output history;
- command blocks;
- attachments;
- Markdown files;
- agent transcripts;
- MCP logs;
- Git branches;
- worktrees;
- directories;
- tasks;
- settings.

### 23.2 Search Filters

Supported syntax should include:

```text
type:command
type:output
exit:nonzero
cwd:backend
branch:main
worktree:agent-auth
agent:claude
has:attachment
duration:>30s
date:today
risk:high
mcp:github-readonly
```

### 23.3 Command Palette

The command palette shall cover:

- terminal commands;
- tabs;
- panes;
- workspaces;
- worktrees;
- Markdown;
- attachments;
- agents;
- MCP;
- Git;
- PRs;
- tasks;
- themes;
- settings;
- diagnostics;
- resource controls.

---

## 24. Security, Safety, and Privacy

This section is governed by the consolidated threat model in Section 35 and the secrets and environment-credential feature in Section 37.

### 24.1 General Security

BongTerm shall implement:

- local encrypted secret vault;
- Windows Credential Manager or DPAPI integration;
- secret detection in attachments;
- secret detection in terminal output;
- secret redaction in exports;
- signed updates;
- dependency auditing;
- extension permission manifests;
- crash-safe local storage;
- audit logs for agent/MCP actions;
- secure defaults for new workspaces.

### 24.2 Dangerous Command Guardrails

BongTerm shall warn or require confirmation for:

- `rm -rf`;
- `del /s`;
- `format`;
- `diskpart`;
- `git push --force`;
- `git reset --hard`;
- `kubectl delete`;
- `terraform destroy`;
- `docker system prune`;
- production deploy commands;
- destructive MCP tool calls;
- commands targeting protected branches/environments.

### 24.3 Production Safety Mode

Profiles may enable production safety mode:

```json
{
  "safety": {
    "requireConfirmForDestructiveCommands": true,
    "disableAgentAutoRun": true,
    "disableMcpWriteTools": true,
    "confirmPaste": true,
    "watermark": "PRODUCTION",
    "themeOverride": "Danger Glass"
  }
}
```

### 24.4 Workspace Trust Model

BongTerm shall classify workspaces as:

- trusted;
- limited trust;
- untrusted.

Untrusted workspaces shall:

- disable auto-running workspace scripts;
- disable MCP servers by default;
- disable agent auto-launch;
- require confirmation before loading workspace config;
- show visible trust state;
- block secrets from being exposed unless explicitly approved.

### 24.5 Privacy

BongTerm shall:

- operate without cloud sync;
- keep command history local by default;
- allow disabling history per profile;
- allow excluding directories from indexing;
- allow purging history;
- allow redacting exports;
- disclose all data sent to agents/tools;
- provide per-agent attachment visibility;
- keep telemetry off by default unless explicitly enabled.

---

## 25. Performance and Resource Requirements

### 25.1 Quantitative Performance Budgets

The prior PRD used qualitative terms that could not be regression-tested. BongTerm shall enforce the following initial budgets on a documented reference Windows laptop and publish benchmark methodology.

| Axis | MVP Target |
|---|---:|
| Cold startup, warm cache, one pane | ≤ 300 ms to app shell; ≤ 800 ms to first prompt with shell integration |
| Keystroke-to-glyph latency, p99 | ≤ 16 ms; stretch target ≤ 8 ms on 120 Hz displays |
| Idle CPU, one pane, 60-second window | ≤ 0.1% average CPU |
| Inactive tabs | zero continuous rendering work |
| BongTerm core RSS, one pane | ≤ 120 MB |
| Additional pane RSS overhead | ≤ 25 MB per pane, excluding shell process |
| Additional pane VRAM overhead | ≤ 8 MB where glyph atlas can be shared |
| Total VRAM ceiling | ≤ 256 MB by default, with eviction beyond ceiling |
| ConPTY throughput test | ≥ 50 MB/s sustained for controlled high-output benchmark, or documented platform exception |
| MCP server RSS | configurable default cap ≤ 60 MB per managed server child tree where enforceable |
| Out-of-process plugin RSS | configurable default cap ≤ 40 MB per plugin |
| 4-pane idle battery contribution | ≤ 1% per hour target on reference laptop |
| Agent budget | user-configurable max runtime, process count, token estimate, and cost estimate |

All budgets require CI or nightly benchmark coverage. Any missed budget must be visible in release notes before public release.

### 25.2 ConPTY Tax Accounting

BongTerm must explicitly account for ConPTY overhead:

- each pane may create a ConPTY/conhost process;
- shell process RSS is separate from BongTerm RSS;
- ConPTY throughput and OSC ordering limitations must be tested;
- command-block reliability must be graded per shell/profile, not assumed globally;
- performance dashboards must show BongTerm process, shell process, conhost process, agent child processes, and MCP child processes separately.

### 25.3 Dual Resource Governance

BongTerm shall manage CPU/latency and RAM/process bloat through different mechanisms. A single “performance mode” is insufficient.

#### Track A — Terminal hot-path CPU/latency

| Concern | Required mitigation | Acceptance gate |
|---|---|---|
| VT/ANSI parse cost | SIMD-assisted scanner, in-place byte-slice parser, compact event emission | parser MB/s and allocations/MB benchmark |
| Render frame cost | dirty-region rendering, row/cell damage tracking, batched draws | p99 keystroke-to-glyph and scroll FPS benchmark |
| Scrollback cost | append-only chunks, virtualization, lazy materialization | huge-scrollback benchmark |
| Queue growth | bounded channels and backpressure | slow-consumer stress test |
| GPU memory | shared glyph atlas and VRAM ceiling | VRAM growth test by pane/window count |

#### Track B — Agent/MCP/plugin RAM and process growth

| Concern | Required mitigation | Acceptance gate |
|---|---|---|
| MCP process multiplication | shared `bongterm-mcp-host`, process pool, local HTTP loopback where supported, stdio bridge where unavoidable | N agents × M MCP test with process-count/RSS ceiling |
| Agent runaway cost | JobObject limits, runtime/cost/token kill-switches, approval gates | runaway-agent test |
| Plugin bloat | WASM-first plugins, no Node extension host, JobObject-capped native adapters | plugin RSS/CPU cap enforcement test |
| Duplicate wrappers | package resolver/cache for MCP launchers, avoid repeated `npx` trees where legally and technically safe | duplicate-process detector |
| User visibility | resource ledger and dashboard | every child process attributable to a feature owner |

#### Admission control

Before launching a new agent, MCP server, plugin, or pane, BongTerm shall evaluate the workspace budget. If projected CPU/RSS/process/VRAM usage exceeds configured thresholds, the user must receive an explicit choice: continue, lower resource profile, reuse an existing MCP host, suspend inactive resources, or cancel.

#### Abstraction-respecting performance policy

Performance regressions shall be fixed by reducing copies, allocations, redraws, queue growth, or process duplication. They shall not be fixed by unsupported process injection, hidden-console scraping, undocumented syscall paths, kernel-mode drivers, or EDR-hostile hooks.

### 25.4 Resource Ledger

BongTerm shall maintain a local resource ledger for:

- panes;
- tabs;
- workspaces;
- agents;
- worktrees;
- MCP servers;
- plugins;
- render surfaces;
- background workers.

For each resource, record where available:

- CPU;
- RSS/private bytes;
- VRAM estimate;
- process count;
- child processes;
- open files/handles;
- network endpoints;
- tokens in/out;
- estimated cost;
- runtime duration;
- policy violations.

### 25.5 Implementation Requirements

BongTerm shall use:

- reusable ring/slab buffers for ConPTY reads;
- in-place VT/ANSI/OSC parsing over byte slices;
- SIMD-assisted scanning for terminal control characters and UTF-8 boundaries;
- bounded allocation strategy with parser/render allocation counters;
- virtualized scrollback;
- dirty-region rendering;
- shared glyph atlas across panes with identical font/DPI/ligature tuples;
- VRAM ceiling and eviction;
- batched drawing;
- lazy indexing;
- inactive tab suspension;
- append-only output chunks;
- bounded worker pools;
- cancellable background operations;
- local diagnostics mode;
- resource dashboard;
- process-tree accounting;
- benchmark harness for high-output and many-pane scenarios.

### 25.6 User Resource Controls

Users shall configure:

- max scrollback lines;
- max memory per tab/session;
- max pane count warning threshold;
- inactive tab suspension;
- inactive agent suspension;
- agent runtime/cost kill-switch;
- MCP server memory/CPU limits;
- plugin memory/CPU limits;
- idle MCP shutdown;
- animation disablement;
- acrylic/Mica disablement;
- software rendering fallback;
- indexing limits;
- log compaction;
- transcript retention;
- command history retention.

### 25.7 Benchmark Fixtures and Reference Methodology

The Section 25 budgets are binding only when measured against reproducible fixtures. BongTerm shall publish the benchmark harness, reference hardware profile, Windows version, GPU/driver version, shell versions, and test commands for every release-quality benchmark.

| Benchmark | Required fixture |
|---|---|
| Cold/warm startup | Launch one PowerShell profile and measure app shell, ConPTY spawn, first prompt, and shell-integration readiness. |
| Keystroke-to-glyph latency | Synthetic typing stream plus manual validation on 60 Hz and 120 Hz displays. |
| High output | Emit at least 1 GB of mixed ANSI, UTF-8, OSC 8, color, progress-bar, and long-line output. |
| Parser allocation/copy budget | Count allocations and copies per MB using release build instrumentation. |
| TUI compatibility | Validate vim/nvim, less, git interactive rebase, npm/pnpm watch, Python REPL, PowerShell prompts, and WSL TUIs. |
| Scrollback | Generate 1M lines, then test search, selection, copy, virtualized materialization, and memory ceiling. |
| Many panes | Open 1, 4, 8, and 16 panes with mixed active/inactive states and report BongTerm, conhost, shell, and VRAM cost separately. |
| Agent load | Run 4 supervised dummy agents, each spawning child commands and modifying files in separate worktrees. |
| MCP load | Run N agents × M pinned MCP sample servers with process/RSS/cold-start/idle-shutdown reporting. |
| GPU recovery | Simulate DXGI device removal/reset and verify swap-chain/glyph-atlas recovery without scrollback loss. |
| IME | Validate Japanese, Chinese, and Korean composition in terminal mode and enhanced editor mode. |
| RDP/GPU-limited | Validate software fallback and reduced-material rendering under RDP or device-limited environments. |
| Secret leak regression | Run a corpus of known token/private-key patterns through command input, output, transcript, export, and diagnostics. |

A release may miss a target only if the release notes identify the fixture, measured result, platform, and mitigation plan.

## 26. Accessibility Requirements

BongTerm shall support:

- screen readers;
- keyboard-only navigation;
- high contrast mode;
- reduced motion;
- disable transparency;
- configurable font size;
- configurable cursor;
- colorblind-safe themes;
- semantic block navigation;
- accessible command palette;
- accessible attachment drawer;
- accessible MCP permission prompts;
- accessible Markdown preview;
- accessible diff review.

---

## 27. Extensibility

### 27.1 Extension Policy

BongTerm shall not ship a VS Code-style Node extension host in v1. Extension design must prioritize bounded resource usage and crash isolation.

### 27.2 Extension Tiers

| Tier | Use Cases | Runtime | Resource Policy | MVP Status |
|---|---|---|---|---|
| Tier 0 | Built-in commands and settings | Native BongTerm | Covered by app budgets | Required |
| Tier 1 | Themes, syntax metadata, exporters, simple parsers | WASM/WASI | fixed memory cap, no network by default | Optional |
| Tier 2 | Agent launchers, MCP installers, task providers | Out-of-process native adapter | JobObject RSS/CPU cap, signed, auditable | Limited |
| Tier 3 | Marketplace plugins | TBD | requires security review | Post-MVP |

### 27.3 Extension Safety

Extensions shall be:

- signed or clearly marked unsigned;
- enforcement-level-scoped;
- workspace-scoped where applicable;
- resource-bounded;
- crash-isolated;
- disabled in untrusted workspaces unless approved;
- visible in the resource ledger;
- removable with cleanup of local state.

No extension may run as hidden background infrastructure without appearing in diagnostics.

## 28. Build Guidance

### 28.1 Repository Structure

```text
/bongterm
├─ /src
│  ├─ /app_host
│  ├─ /terminal_core
│  ├─ /renderer
│  ├─ /workspace
│  ├─ /shell_integration
│  ├─ /command_blocks
│  ├─ /shell_intelligence
│  ├─ /markdown
│  ├─ /attachments
│  ├─ /agents
│  ├─ /worktrees
│  ├─ /environment_isolation
│  ├─ /mcp
│  ├─ /command_lens
│  ├─ /storage
│  ├─ /security
│  ├─ /settings
│  └─ /ui
│
├─ /plugins
├─ /schemas
├─ /themes
├─ /profiles
├─ /tests
│  ├─ /unit
│  ├─ /integration
│  ├─ /terminal_compat
│  ├─ /performance
│  ├─ /security
│  ├─ /agent_workflows
│  ├─ /mcp
│  └─ /ui_automation
│
├─ /tools
├─ /docs
├─ /installer
└─ /ci
```

### 28.2 Engineering Modules

| Module | Primary Responsibility | SOLID Boundary Rule |
|---|---|---|
| App Host | Window lifecycle, message loop, native UI shell | Owns OS windowing only; delegates business actions to application services. |
| Terminal Core | ConPTY, parser, grid, input, scrollback | No dependency on agents, MCP, Git, Markdown, settings UI, or cloud modules. |
| Renderer | DirectWrite/Direct2D/Direct3D drawing | Consumes immutable terminal snapshots; does not infer command, Git, or agent semantics. |
| Workspace | sessions, tabs, panes, layouts, optional daemon IPC | Coordinates surfaces through interfaces; does not own terminal parsing or agent policy. |
| Shell Integration | command boundaries, metadata, shell hooks | Emits/receives shell events; does not persist blocks or mutate UI state directly. |
| Command Blocks | block model, actions, persistence references | Depends on shell-event abstractions, not concrete shell adapters. |
| Shell Intelligence | suggestions, highlighting, correction, autojump | Uses read-only command/history/project ports; cannot run commands directly. |
| Markdown | source/preview/split/review modes | Post-MVP; isolated from terminal hot path and agent runtime. |
| Attachments | file/block/diff/context bundle handling | Uses security/redaction ports before exposing context to agents or exports. |
| Agents | profiles, launcher, observability, lifecycle, transcripts, replay | Implements `AgentAdapter`; cannot bypass approvals or secret policy. |
| Worktrees | create/manage/merge/discard isolated Git worktrees | Depends on `GitProvider`; does not allocate ports or database branches directly. |
| Environment Isolation | ports, env files, DB branches, Docker, caches | Depends on worktree IDs and policy ports; does not mutate Git state. |
| MCP | install, config, permissions, server supervision, context optimizer, process governor | Implements `McpTransport` and process governance; agent code sees only approved tools. |
| Command Lens | help, docs cache, command learning | Post-MVP; read-only by default and isolated from command execution. |
| Storage | SQLite, logs, cache, migrations, append-only chunks | Provides repositories/ports; does not make product policy decisions. |
| Security | secrets, policy, redaction, approvals, workspace trust | Central policy authority; must be called through explicit interfaces. |
| Settings | JSON schema, validation, import/export | Validates and publishes typed config snapshots; does not mutate live systems directly. |
| UI | command palette, panels, sidebar, dialogs | Presentation only; no direct process spawning, Git mutation, or secret access. |


### 28.3 SOLID Engineering Governance

Every BongTerm feature shall pass an architecture review proportional to risk. The review is mandatory for changes touching terminal hot path, process lifecycle, agents, MCP, security, storage, worktrees, plugins, or resource governance.

#### Required design-review fields

Each architectural PRD/task shall include:

| Field | Requirement |
|---|---|
| Owning module | Exactly one primary owner module must be named. |
| Reason to change | The change must state which responsibility is being modified. |
| Public interface | New or changed interfaces must be listed with expected callers. |
| Dependency direction | The change must confirm it does not reverse allowed dependency flow. |
| Substitution test | At least one fake/mock/alternate implementation must pass the same contract where practical. |
| Resource impact | CPU, RSS, VRAM, process count, and I/O impact must be estimated or measured. |
| Security path | The policy/redaction/approval path must be explicit when commands, files, secrets, agents, or MCP tools are involved. |
| Failure mode | Timeout, cancellation, crash, partial-write, and rollback behavior must be specified. |

#### Architecture fitness functions

CI shall include automated checks for:

1. Forbidden dependency imports across module boundaries.
2. Terminal hot-path dependencies on agent, MCP, Git polling, Markdown, cloud, or settings UI code.
3. Public interfaces exceeding agreed complexity thresholds.
4. Cyclic dependencies between modules/crates.
5. Missing contract tests for `TerminalSession`, `RendererBackend`, `AgentAdapter`, `McpTransport`, `GitProvider`, `SecretStore`, and `PolicyEvaluator` implementations.
6. Resource-governance regression tests for plugins, MCP servers, panes, and agents.
7. Policy-bypass tests for destructive command, MCP write tool, production profile, and secret-access paths.

#### Review gates

A change is blocked when it:

- adds a concrete platform dependency to domain/application logic;
- adds UI-driven direct process spawning;
- allows an agent/MCP/worktree operation without policy evaluation;
- expands a broad service interface instead of creating a narrow role interface;
- introduces a new background worker without lifecycle ownership, metrics, cancellation, and resource limits;
- modifies terminal hot-path behavior without benchmark evidence.

### 28.4 Suggested Build Order

Functional build order is revised to enforce the critique before scope expands:

1. Terminal host, ConPTY wrapper, VT parser, grid, scrollback, and renderer foundation.
2. Quantitative benchmark harness and resource ledger stub.
3. Profiles, settings, theming, keybindings, and basic UI shell.
4. Tabs, panes, layouts, search, command palette, and workspace restore.
5. Shell integration and command blocks for PowerShell and Bash/WSL, with reliability grading.
6. Resource dashboard with BongTerm/shell/conhost/process-tree accounting.
7. Basic agent launcher, transcript capture, file-change tracking, lifecycle controls, and approvals.
8. Serialized Git worktree creation, state reconciliation, lock detection, and safe cleanup.
9. MVP environment isolation: ports, env files, temp/cache/log directories, Docker Compose names.
10. MCP Manager v1: manual import/config, permission prompts, health checks, logs.
11. MCP Process Governor: managed host, process pool, JobObject limits, lifecycle controls.
12. Context Optimizer: tool-schema pruning, allowlists, token/context preview.
13. Security hardening: secret vault, redaction, workspace trust, dangerous-command policies.
14. Packaging, accessibility, diagnostics, terminal compatibility, and performance hardening.
15. Post-MVP: Markdown review, Command Lens, database branching, plugin marketplace, durable daemon, collaboration.

## 29. Testing Strategy

### 29.1 Unit Tests

- VT parser;
- OSC boundary parser;
- grid mutations;
- scrollback storage;
- JSON schema validation;
- syntax correction ranking;
- autosuggestion ranking;
- autojump ranking;
- Markdown parsing;
- attachment filtering;
- secret detection;
- security policy matching;
- MCP config validation;
- worktree metadata parsing;
- port allocator;
- property-based tests for grid mutation and scrollback invariants.

### 29.2 Integration Tests

- PowerShell ConPTY sessions;
- CMD sessions;
- WSL sessions;
- Git Bash sessions;
- SSH sessions;
- shell integration command boundaries;
- command block creation;
- session restore;
- Markdown preview with local links;
- attachment to agent workflow;
- MCP server lifecycle;
- MCP semantic pruning;
- agent process supervision;
- Git worktree lifecycle;
- environment isolation lifecycle.

### 29.3 Terminal Compatibility Tests

- ANSI color matrix;
- truecolor;
- alternate screen applications;
- full-screen TUIs;
- mouse mode;
- bracketed paste;
- Unicode and emoji;
- line wrapping;
- resize behavior;
- scrollback correctness;
- OSC 8 hyperlinks;
- OSC command boundaries.

### 29.4 Performance Tests

Performance tests shall include:

- cold/warm startup;
- keystroke-to-glyph latency;
- zero-allocation parser hot-path benchmark;
- allocations per MB of terminal output;
- copies per MB of terminal output;
- SIMD parser throughput benchmark;
- high-volume output throughput;
- `Get-Process`-style ConPTY stress test;
- rapid scrolling;
- huge scrollback;
- many inactive tabs;
- many panes;
- VRAM growth by pane/tab/window count;
- glyph atlas sharing/eviction;
- multiple active worktrees;
- multiple agent sessions;
- MCP process-tree scaling;
- MCP cold-start and idle-shutdown latency;
- plugin RSS/CPU cap enforcement;
- resource admission-control tests;
- process attribution tests for every child process;
- search index build;
- memory leak detection over long runs;
- battery/idle contribution benchmark.

### 29.5 Security Tests

- prohibited abstraction-bypass checks, including no DLL injection, no hidden-console scraping, no undocumented syscall stubs, and no kernel-driver dependencies;
- EDR-friendly process-tree review;
- child-process attribution audit;
- secret detection;
- redaction;
- dangerous command detection;
- MCP permission prompts;
- MCP tool audit logging;
- attachment boundary enforcement;
- production safety mode;
- agent approval flow;
- malicious path handling;
- malicious terminal escape handling;
- continuous fuzzing of the VT/ANSI/OSC parser (libFuzzer/cargo-fuzz) against malformed and adversarial byte streams;
- OSC 52 clipboard-write gating and OSC 8 hyperlink-target confirmation;
- untrusted workspace config;
- environment file redaction;
- worktree cleanup safety.

### 29.6 UI/UX Tests

- high-DPI scaling;
- mixed-DPI multi-monitor;
- keyboard-only workflows;
- screen reader behavior;
- reduced motion;
- high contrast;
- theme import/export;
- glass readability;
- pane resizing;
- command palette coverage;
- worktree dashboard usability;
- MCP permission prompts.


### 29.7 SOLID and Architecture Contract Tests

The testing strategy shall include explicit SOLID-alignment checks:

- **SRP tests:** module-level dependency checks prove terminal hot path, renderer, agent, MCP, worktree, storage, and security modules do not import unrelated modules.
- **OCP tests:** adding a sample fake agent, fake MCP transport, fake renderer, and fake Git provider works through registries/interfaces without modifying core orchestration code.
- **LSP tests:** mock and production implementations of core interfaces pass the same lifecycle, error, timeout, cancellation, and metric-collection contract tests.
- **ISP tests:** interface-size checks prevent broad service interfaces from accumulating unrelated methods.
- **DIP tests:** application services compile against port interfaces with infrastructure adapters swapped out in unit/integration tests.
- **Policy-path tests:** destructive commands, MCP write tools, production profiles, and secret access always route through the policy evaluator.
- **Resource-boundary tests:** each interface exposing lifecycle control must also expose metrics and cancellation semantics.

---

## 30. MVP Cutline

### 30.1 MVP Wedge

The MVP wedge is:

> **The best Windows-native terminal cockpit for observing and safely running multiple CLI coding agents with bounded resources.**

MVP must prove terminal correctness, resource accounting, agent observability, and controlled parallelism. It must not attempt to ship every adjacent developer-tool feature.

### 30.2 v1 MVP Must Include

- Native ConPTY terminal foundation.
- PowerShell, CMD, WSL, and Git Bash support.
- Tabs, panes, profiles, keybindings, themes.
- Command blocks for PowerShell and Bash/WSL where shell integration is reliable.
- Search and command palette.
- Basic project and Git detection.
- Resource ledger and dashboard.
- Basic agent launcher and agent sidebar.
- Agent transcripts, process-tree capture, file-change tracking, lifecycle controls, approvals, and replayable export.
- Dangerous command approvals.
- Serialized one-click agent worktree creation.
- Worktree lock detection and Git truth reconciliation.
- Basic environment isolation for ports, env files, temp/cache/log directories, and Docker Compose names.
- Basic MCP manual import/configuration, health checks, permission prompts, logs, and process display.
- MCP Process Governor v1 with resource caps where enforceable.
- Context Optimizer v1 for MCP tool-schema pruning and token/context preview.
- Local encrypted secret vault and the secrets/environment-credential feature (Section 37), including `.env` import and vault-backed environment injection.
- Signed installer.
- CI gates for performance, terminal compatibility, resource scaling, and security.

### 30.3 v1 MVP Should Exclude

- Full MCP marketplace.
- Full plugin ecosystem.
- Node extension host.
- Enterprise SSO/RBAC.
- Cloud collaboration.
- Remote desktop / RDP / X11 suite.
- Full IDE-like editor.
- Markdown review and presentation mode.
- Command Lens.
- Database branch provisioning.
- Deep AST-aware code review.
- Fully autonomous multi-agent orchestration.
- Durable session daemon.
- Browser/Electron terminal rendering.
- True mid-session steering unless an upstream agent exposes reliable IPC/API support.

### 30.4 Product Cut Rules

A feature is not allowed into MVP if it:

- lacks a measurable resource budget;
- creates hidden background processes;
- bypasses the resource ledger;
- depends on undocumented agent stdin/TUI behavior;
- requires paid third-party SaaS accounts to demonstrate core value;
- degrades terminal hot-path performance;
- weakens workspace trust or secret handling.

### 30.5 MVP-0 Cutline

The first proof-of-product shall be **MVP-0**, not the full v1 MVP. MVP-0 exists to validate the adoption wedge and avoid building a broad terminal/agent platform before the core workflow is proven.

#### MVP-0 Must Include

- Native ConPTY terminal foundation.
- PowerShell, CMD, WSL, and Git Bash launch support.
- Tabs, panes, profiles, themes, and keybindings.
- Resource ledger with BongTerm, conhost, shell, agent, and child-process accounting.
- Agent launcher for user-installed CLI agents.
- Transcript capture and replayable export.
- File-change summary using Git diff plus filesystem watcher for untracked/ignored files.
- One safe worktree creation flow with Git lock detection and target-path disclosure.
- Advisory dangerous-command detection, clearly labeled as advisory unless routed through an enforceable broker.
- Basic secret vault with references-only configuration and launch-time disclosure.
- Signed internal installer/build.
- Benchmark harness for startup, latency, parser throughput, RSS, VRAM, and process-tree attribution.

#### MVP-0 Must Exclude

- MCP Process Governor.
- Context Optimizer.
- General plugin system.
- Markdown review and presentation mode.
- Command Lens.
- Full environment isolation beyond basic port/env warnings.
- Durable session daemon.
- Deep PR/review automation.
- Full database branching.
- Claims of hard command/file/network blocking for arbitrary agents unless OS-enforced or runner-enforced.

#### MVP-0 Exit Criteria

MVP-0 is acceptable only when a dogfood user can launch an agent in a worktree, observe transcript/process/resource/file changes, stop or kill the process tree, review the diff, and discard or keep the worktree without using another terminal for that workflow.

## 31. Acceptance Criteria

### 31.1 Terminal MVP Acceptance

The terminal foundation is acceptable when:

- PowerShell, CMD, WSL, and Git Bash launch correctly.
- ANSI colors, cursor movement, selection, copy/paste, resizing, alternate screen, and scrollback work correctly.
- Terminal rendering remains responsive under high output.
- JSON profiles and themes work.
- Tabs and panes work.
- UI remains readable with default theme.
- ConPTY overhead is visible in diagnostics.
- The performance budgets in Section 25 are measured and reported.

### 31.2 Command Block Acceptance

Command-block workflow is acceptable when:

- command boundaries are reliable for PowerShell and Bash/WSL profiles with supported shell integration;
- BongTerm shows reliability status per shell/profile;
- fallback mode is clearly marked when OSC/shell integration is unavailable or unreliable;
- command/output copy actions work;
- blocks can be searched, bookmarked, attached, and exported;
- command-block creation does not depend on hidden brittle prompt string matching for critical workflows.

### 31.3 Agent Workflow Acceptance

Agent workflow is acceptable when:

- configured CLIs can launch from profiles;
- agent sessions appear in the sidebar;
- process tree, current visible command/activity, and resource usage are visible where detectable;
- transcripts are captured;
- file changes are tracked;
- lifecycle controls work;
- approvals are visible and auditable;
- replay with summarized context works;
- unsupported steering features are marked unavailable rather than simulated.

### 31.4 Worktree and Environment Acceptance

Parallel agent workflow is acceptable when:

- users can create an isolated agent worktree from the UI;
- worktree creation is serialized per repository;
- Git locks are detected and surfaced;
- branch/path/agent/task mapping is visible;
- environment file generation works;
- port assignment prevents common dev-server collisions;
- Docker Compose naming isolation works;
- worktree state is reconciled against Git truth;
- worktree changes can be diffed, committed, discarded, archived, or merged;
- cleanup actions show target path and pre-cleanup diff snapshot.

### 31.5 MCP Acceptance

MCP support is acceptable when:

- users can manually configure an MCP server;
- permissions are displayed before enabling;
- health checks work;
- logs are visible;
- secrets are stored securely;
- tool calls are visible during agent use;
- destructive/external-write tools require approval;
- Context Optimizer can filter tool exposure per agent/task;
- Process Governor displays child processes and enforces configured limits where possible.

### 31.6 Resource-Efficiency Acceptance

Resource-efficiency claims are acceptable only when:

- BongTerm core RSS, per-pane RSS, VRAM, and idle CPU meet Section 25 targets on the reference system;
- MCP process growth is visible and bounded by policy;
- plugins cannot run outside the ledger;
- all hidden child processes are visible in diagnostics;
- release builds include benchmark results or explicit budget exceptions.

## 32. Licensing and Open-Source Guidance

### 32.1 Native Windows Technologies

Using native Windows APIs such as Win32, ConPTY, DirectWrite, Direct2D, Direct3D, DWM, and Windows App SDK does not inherently prevent the application from being open source or commercial. The team must comply with Microsoft SDK/runtime terms and third-party dependency licenses.

### 32.2 Recommended License Strategy

Recommended default:

- core terminal: Apache-2.0;
- example themes/profiles: MIT;
- optional commercial services or enterprise modules: separate proprietary or dual license.

Rationale:

- Apache-2.0 is permissive and commercial-friendly.
- Apache-2.0 includes explicit patent language.
- Enterprise users are usually more comfortable with permissive licenses than strong copyleft for core developer tools.

### 32.3 Dependency, Agent, and MCP Distribution Rules

The team shall:

- maintain a third-party dependency inventory;
- generate a Software Bill of Materials (SBOM) per release for enterprise/security review;
- record license, source, version, and usage;
- avoid GPL/AGPL dependencies in the permissive core unless intentionally accepted;
- avoid copying code from AGPL projects into proprietary modules;
- verify font licenses;
- verify icon and asset licenses;
- verify MCP server redistribution rights;
- verify CLI agent bundling rights;
- avoid bundling Claude Code, Codex CLI, Gemini CLI, Copilot CLI, or other commercial CLIs unless written permission/license terms allow it;
- prefer “detect and launch user-installed CLI” over bundling;
- preserve MIT/Apache notices where required;
- document telemetry/data behavior for every agent/MCP integration.

### 32.4 Branding Rules

BongTerm shall not:

- use Microsoft, Windows Terminal, Warp, Claude, OpenAI, or other third-party branding misleadingly;
- use competitor logos without permission;
- imply official integration unless authorized;
- copy competitor UI assets, icons, naming, or trade dress.

---

### 32.5 Release, Update, and Distribution Security

BongTerm shall define a release-security model before public distribution.

Requirements:

- signed MSIX/MSI installers and signed binaries;
- stable, beta, and nightly channels with explicit risk labels;
- update manifests signed and verified before installation;
- rollback to prior signed version;
- offline enterprise installer package;
- SBOM published per release;
- dependency and license inventory published or exportable;
- code-signing key storage and rotation policy;
- compromised-release response plan;
- vulnerability disclosure process and security contact;
- reproducible or attestable build pipeline where practical;
- release notes listing benchmark results, known budget exceptions, and security-impacting changes.

No auto-update mechanism may silently enable cloud telemetry, new MCP servers, new agent capabilities, or broader secret access.

## 33. Open Questions and Decision Triggers

### 33.1 Open Questions

1. Should the core be fully Rust-first, or should C++ remain only for thin Windows interop?
2. What reference Windows laptop defines MVP performance budgets?
3. Which shells are supported with “reliable command blocks” at launch?
4. Which agent CLIs expose supported IPC/API for real steering, if any?
5. What MCP transports are mandatory for the first release?
6. Should WSL2 runner support be MVP or post-MVP?
7. What is the minimum acceptable remote/SSH workflow for Windows developers using Linux dev boxes?
8. How strict should production safety mode be by default?
9. How should untrusted workspace configuration files be sandboxed?
10. What exact licensing policy applies to marketplace-like agent/MCP installers?

### 33.2 Decision Triggers

| Trigger | Required Action |
|---|---|
| Average managed MCP RSS > 100 MB in dogfood fleets | Require shared/remote HTTP transport or disable always-on mode |
| ConPTY OSC ordering remains unreliable for target shells | Downgrade command blocks to best-effort for affected profiles and improve fallback UX |
| Agent steering remains unsupported by target CLIs | Keep steering out of MVP and market observability/replay instead |
| Worktree lock failures exceed acceptable dogfood threshold | Promote WSL2/sandbox/remote runners ahead of advanced worktree automation |
| Core RSS exceeds Section 25 budget | Freeze feature additions until performance regression is resolved |
| VRAM exceeds Section 25 budget | Reduce surfaces, improve atlas eviction, or enable software-rendering fallback |
| Plugin resource usage trends toward extension-host bloat | Keep marketplace closed and enforce WASM/out-of-process caps |
| Windows-only adoption is weaker than expected | Prioritize Rust/wgpu portability path |

## 34. Definition of Done

A feature is done only when:

1. Functional requirements are implemented.
2. Keyboard access exists.
3. Settings/config behavior is documented.
4. Error states are handled.
5. Logs/diagnostics exist where relevant.
6. Security review is completed for features touching files, agents, MCP, shell execution, secrets, Git, worktrees, or external tools.
7. Performance impact is measured across both hot-path latency and child-process resource growth.
8. The implementation stays within supported Windows user-mode abstractions unless an explicit security exception exists.
9. Accessibility behavior, including UI Automation exposure for terminal-surface changes, is validated (Section 36.2).
10. Tests are included at the correct level.
11. User-facing documentation is updated.
12. SOLID architecture review is complete for features touching terminal core, rendering, process lifecycle, agents, MCP, security, storage, worktrees, plugins, or resource governance.
13. Contract tests exist for new or changed interfaces, including substitute/mock implementations where practical.
14. Dependency direction, resource ownership, cancellation, and policy-evaluation paths are validated in CI.
15. For features touching agents, MCP, files, secrets, or shell execution, the relevant threat-model scenarios (Section 35) are considered, and any secret handled is routed through the vault and reference model (Section 37).

---

## 35. Threat Model and Trust Boundaries

### 35.1 Purpose

BongTerm runs untrusted code (CLI agents, MCP servers, arbitrary shell programs) and ingests untrusted content (terminal output, logs, diffs, file contents, MCP results) while holding high-value assets (source code, secrets, history). This section names the assets, trust boundaries, adversaries, and primary attack scenarios so that the security controls elsewhere in this PRD map to explicit threats rather than being a loose checklist. The methodology is STRIDE-informed (Spoofing, Tampering, Repudiation, Information disclosure, Denial of service, Elevation of privilege).

### 35.2 Protected Assets

- user source code, repositories, and worktrees;
- secrets and credentials (API keys, tokens, SSH/private keys, `.env` values);
- the local secret vault;
- command history, transcripts, and scrollback;
- workspace, profile, and policy configuration;
- the user's machine and any networks/services reachable from it;
- agent and MCP execution authority — i.e., what an agent or tool is permitted to do on the user's behalf.

### 35.3 Trust Boundaries

| Zone | Trust level | Notes |
|---|---|---|
| BongTerm core (parser, renderer, policy engine, vault) | Trusted | Memory-safe by default; smallest reasonable trusted computing base. |
| User / operator | Trusted | Grants authority; can approve dangerous actions deliberately. |
| Shell child processes | Semi-trusted | Run user commands, but their **output** is untrusted data. |
| Terminal output, log files, diffs, file contents | **Untrusted data** | May contain hostile escape sequences or prompt-injection payloads. |
| Agent CLI processes | Semi-trusted | Authorized to act, but steerable by untrusted content they ingest. |
| MCP servers | **Untrusted code + untrusted output** | Third-party processes; tool results are attacker-influenceable. |
| Workspace configuration files | **Untrusted until trust granted** | Can request auto-run, agent launch, MCP enablement. |
| Plugins | Sandboxed / untrusted | WASM or JobObject-capped out-of-process (Section 27). |
| Remote / SSH / WSL2 runners | Separate trust zone | Different failure and isolation semantics than local. |

### 35.4 Adversaries and Threat Scenarios

| Threat | Example | Primary mitigations |
|---|---|---|
| **Indirect prompt injection** (highest priority) | A poisoned README, log line, diff, or MCP tool result instructs the agent to run a destructive command or exfiltrate a secret. | Treat all agent-ingested content as untrusted (35.5); approval gates for shell/file/destructive/external-write actions (17, 24.2); least-privilege MCP exposure (20.5-20.6); secret non-exposure and scoping (37); production safety mode (24.3); visible authority (16, 35.5). |
| **Supply-chain compromise** | A malicious or typosquatted MCP package executes on launch. | Version and hash pinning, provenance display, update review (20.6); pinned runner instead of direct `npx -y` (20.4); resource caps via JobObject plus stronger sandbox/runner isolation where needed (17.6); SBOM and dependency inventory (32.3). |
| **Secret exfiltration** | Agent or MCP server sends an API key to an external endpoint. | Default-deny network for MCP; secrets never in argv/URL/logs; scoped in-memory resolution (37); egress and resource visibility in the ledger (25.4). |
| **Malicious terminal escapes** | Hostile program emits OSC 52 to hijack the clipboard, OSC 8 to spoof a hyperlink, or a malformed sequence to crash/own the parser. | Fuzzed VT/OSC parser (29.5); OSC 52 clipboard writes gated/confirmed; hyperlink target confirmation; bounded in-place parsing (6.3.1). |
| **Malicious workspace config** | A cloned repo ships config that auto-runs scripts, launches an agent, or enables MCP servers. | Workspace trust model with default-deny for untrusted workspaces (24.4); untrusted-config sandboxing (Open Question 9). |
| **At-rest data theft** | Local malware or another user reads the vault, transcripts, or scrollback. | DPAPI/Credential Manager per-user encryption for the vault; at-rest secret redaction/encryption for transcripts and scrollback (37.6, 38.4); restrictive ACLs on generated files (37.5). |
| **Plaintext secret sprawl** | API keys committed in `.env` or pasted into config. | References-only config and env-file feature (37); secret detection, `.gitignore` enforcement, staged-secret blocking (37.6, 21.3). |
| **BongTerm itself behaving as malware** | Injection/hooks trip enterprise EDR or expand attack surface. | Prohibited OS-bypass techniques (3.2); EDR-friendly, fully attributable process tree (25.2, 29.5). |
| **Denial of service / resource exhaustion** | Runaway agent or MCP fork-bomb starves the machine. | Admission control, JobObject caps, kill-switches, backpressure (25.3). |

### 35.5 Core Security Principles (binding)

1. **All content an agent ingests is untrusted.** Terminal output, files, diffs, logs, MCP results, and attachments may carry injection payloads. Authority is granted by explicit policy and is never inferred from ingested content.
2. **Default deny, least privilege.** Agents, MCP servers, and tools receive only the capabilities, tools, and secrets explicitly mapped to the task.
3. **Late, scoped secret resolution.** Secrets become plaintext only in memory, only at process-spawn time, and only for the explicitly authorized consumer.
4. **Visible authority.** The user can always see, before launch and during execution, what an agent/tool may do and exactly what data, tools, and secret references it received.

### 35.6 Explicitly Out of Scope

BongTerm does not attempt to defend against a fully compromised OS, kernel, or local administrator-level malware; it does not guarantee a malicious MCP server cannot misbehave within permissions the user granted it; and it does not prevent a user from deliberately approving a dangerous action. These limits are stated so security claims are not overread.

---

## 36. Native Windows Platform Requirements

### 36.1 Purpose

Choosing native ConPTY hosting and a custom DirectWrite/Direct2D/Direct3D renderer creates platform obligations that Electron/WebView apps inherit for free. These are first-class engineering requirements with their own acceptance criteria, not implied behavior.

### 36.2 Accessibility via UI Automation

A GPU-rendered terminal grid exposes no automatic accessibility tree. BongTerm shall implement a UI Automation (UIA) provider over the terminal buffer rather than treating "screen reader support" as a checkbox.

Requirements:

- expose terminal text, scrollback, selection, and cursor through UIA Text pattern providers (text/range providers);
- announce new output through rate-limited live regions without flooding the screen reader;
- track caret and selection changes;
- provide per-pane providers and accessible command-block navigation;
- expose accessible names/roles for the command palette, dialogs, attachment drawer, MCP permission prompts, and approval gates.

Acceptance: Narrator and at least one third-party screen reader can read terminal output, navigate scrollback and command blocks, and operate the palette and approval prompts. This is scoped as real implementation work, not assumed.

### 36.3 GPU Device Loss and TDR Recovery

The renderer shall handle Direct3D/DXGI device-removed and device-reset events (driver Timeout Detection and Recovery, GPU reset, driver update, hybrid-GPU switch, RDP session transitions). On such an event BongTerm shall detect the loss, release device resources, recreate the swap chain, glyph atlas, and pipeline state, and repaint from retained terminal state without crashing or losing scrollback. Repeated device loss shall trigger the software-rendering fallback (cross-ref 25.6).

Acceptance: a forced device-removed condition recovers automatically with no terminal-state loss; covered by an automated test.

### 36.4 IME and Complex Text Input

The terminal and enhanced command editor shall support Windows Input Method Editor (IME) composition for CJK and other composing scripts:

- in-place composition rendered over the grid;
- correct candidate-window positioning relative to the caret;
- composition cancel/commit semantics;
- correct interaction with command-editor mode, autosuggestions, and syntax overlays;
- correct handling of surrogate pairs, combining marks, and grapheme clusters; mixed-direction text shall render correctly (full bidirectional editing is post-MVP).

Acceptance: CJK input via IME works in both normal terminal mode and command-editor mode.

### 36.5 DPI, Monitors, and Window Management

BongTerm shall support per-monitor DPI awareness v2, mixed-DPI multi-monitor setups, live DPI changes when a window moves between monitors, snap/maximize/restore, multiple native windows, and tear-out panes/tabs (cross-ref 8.4, 10.2).

### 36.6 Localization Readiness

UI strings shall be externalized for localization and layouts shall tolerate string expansion; diagnostics shall use locale-aware formatting. Full localization and RTL layout are post-MVP, but the architecture shall not hard-code user-facing strings.

---

### 36.7 Operating System Support and Downgrade Matrix

Windows support shall be explicit. BongTerm shall not rely on Windows 11-only visual effects or GPU behavior without defined fallback.

| Platform / mode | Support status | Required behavior |
|---|---|---|
| Windows 11 23H2 / 24H2 or later | Primary | Full visual system, ConPTY, DirectWrite/Direct2D/Direct3D, UIA, IME, signed installer, and resource ledger support. |
| Windows 10 22H2 | Optional compatibility target | No Mica/Mica Alt dependency; reduced visual effects; ConPTY compatibility test required; unsupported features hidden rather than broken. |
| Windows Server | Explicit decision required | Either certify a server profile or state unsupported. If supported, assume limited GPU/material effects and stricter enterprise policy controls. |
| RDP / GPU-limited sessions | Required graceful degradation | Detect device limitations, disable expensive material effects, fall back to software or reduced rendering where needed. |
| Hybrid GPU / driver reset / TDR | Required | Device-loss recovery per Section 36.3. |
| WSL2 NAT networking | Required caveat support | Port allocation must account for localhost-forwarding behavior and WSL distribution boundaries. |
| WSL2 mirrored networking | Required caveat support | Port collision logic must account for host/guest overlap and mirrored behavior. |
| High contrast / reduced motion | Required | Visual design must not depend on blur/transparency. |

The installer and settings UI shall show unsupported or degraded features rather than failing silently.

## 37. Secrets and Environment Credential Management

### 37.1 Feature Goal

BongTerm shall provide first-class, local, secure handling of API keys and other credentials — including `.env` files — so that secrets are never stored in plaintext configuration, never committed to a repository, never leaked to agents, MCP servers, transcripts, scrollback, or exports unintentionally, and are injected into shells, agents, and MCP servers only through least-privilege, in-memory, scoped resolution. This feature is owned by the `security` module (Section 28.2) and integrates with environment isolation (Section 19), the agent cockpit (Section 16), MCP management (Section 20), and the threat model (Section 35).

### 37.2 Binding Principles

1. **Configuration holds references, not secrets.** Settings, profiles, themes, agent definitions, MCP definitions, and workspace config may contain only `${secret:NAME}` or `${env:NAME}` references. Plaintext secret values in committed configuration are rejected by schema validation and surfaced by linting.
2. **The vault is the only durable store.** Secret values live exclusively in the DPAPI / Windows Credential Manager-backed encrypted vault (per-user, local, no cloud), keyed by scope and name.
3. **Late, scoped, in-memory resolution.** References resolve to plaintext only at process-spawn time, only for the explicitly authorized consumer, and only in memory.
4. **No secret in argv, URLs, logs, transcripts, or scrollback.** Secrets are passed to child processes through the environment block, never on a command line or in a URL.
5. **Least privilege.** A secret is exposed to a shell/agent/MCP server only through an explicit mapping; the default is deny.
6. **Visible and audited.** Before launch, the user can see which secret references a consumer will receive; access is audit-logged by reference name, consumer, and timestamp — never by value.

### 37.3 Secret Reference Data Model

```text
SecretRef
├─ name
├─ scope            # global | workspace | profile | agent | mcp
├─ source           # vault | env-file | credential-manager | os-env (opt-in)
├─ consumers        # explicit agentIds / mcpIds / profileIds allowed to receive it
├─ rotation         # { lastRotated, rotateReminderDays }
├─ redactionPattern # optional override for detection/redaction
└─ metadata         # never contains the value
```

The secret value is never stored in this record. It resides in the encrypted vault keyed by `(scope, name)` and is resolved on demand.

### 37.4 Environment File Handling

BongTerm shall treat `.env` files as a first-class, security-aware workflow rather than opaque text:

- **Detection:** discover `.env`, `.env.local`, `.env.*`, and custom-named env files within a workspace/worktree.
- **Read-only parsing:** parse dotenv syntax (KEY=VALUE, quoting, comments, multiline values, tolerant of `export` prefixes) without mutating the file unless the user chooses an action.
- **Import flow:** for each key, offer to (a) import the value into the vault and rewrite the on-disk entry to a `${secret:NAME}` reference, (b) keep the value in place but mark the file as secret-bearing (excluded from attachments, exports, and agent context by default), or (c) ignore.
- **Vault-backed env mode:** BongTerm generates the process environment for a shell/agent/MCP run directly from the vault and references at launch, so that **no plaintext `.env` needs to exist on disk** for agent or MCP execution.
- **Ephemeral generated env:** when a tool genuinely requires a real file, BongTerm generates a per-run/per-worktree `.env.agent.local` at a path with restrictive ACLs, ensures it is `.gitignore`d, and deletes it on session end (integrates with 19.3 port assignment and 19.4 env files).
- **Template support:** maintain a non-secret `.env.example`-style template of required keys; warn when a consumer requires a key that has no vault value.

### 37.5 Injection Paths

| Consumer | How secrets are provided | Constraints |
|---|---|---|
| Shell profile | Resolved into the ConPTY child environment at spawn | Never echoed; inline secret entry excluded from history/PSReadLine. |
| Agent CLI | Only mapped secrets placed in the agent's environment | Agent profile `env` uses references; resolved at launch; redacted in transcript. |
| MCP server | Only mapped secrets in the server child environment | Formalizes the `${secret:...}` usage already shown in 20.4; default-deny others. |
| Generated env file (only if required) | Per-run/per-worktree temp path | Restrictive ACL, `.gitignore`d, deleted on exit. |

### 37.6 Detection, Prevention, and Hygiene

BongTerm shall:

- detect token-like strings, private keys, and common cloud-credential formats in attachments, terminal output/scrollback, env files, and command input;
- warn or block when a `.env` or other secret-bearing file is staged or committed, offer to add it to `.gitignore`, and surface this in Git features (21.3) and worktree preflight (18.2);
- block sending env files or secret-bearing files to agents by policy (default on) and show exactly what an agent will receive (cross-ref 14.4, 16, 35.5);
- redact detected secrets at rest in transcripts, scrollback, and logs, and in all exports and diagnostic bundles, while documenting that detection is best-effort and cannot catch every secret.

### 37.7 Rotation and Lifecycle

- Rotate or update a secret value in one place; all references resolve to the new value automatically; optional rotation reminders.
- Revoking a secret invalidates its references; a consumer that requires a missing secret shows an explicit "missing secret" state and **does not launch with an empty value**.
- Per-workspace purge is supported; untrusted workspaces cannot read secrets unless explicitly approved (cross-ref 24.4).

### 37.8 User Experience

- **Secrets manager:** list secrets by scope; show consumers, last-rotated, and source; add/import/rotate/revoke; preview "what will this agent/MCP receive?"; values are masked by default with reveal gated behind an explicit action (and OS re-authentication where supported).
- **Env file panel:** show detected files, their keys, which keys are vault-backed versus plaintext, available import actions, and `.gitignore` status.
- **Launch-time disclosure:** before starting an agent, MCP server, or worktree, display the resolved secret reference names (never values) that will be injected.

### 37.9 Non-Negotiable Security Requirements

BongTerm shall never:

- write resolved secret values to settings JSON, command history, transcripts, scrollback, logs, telemetry, or exports;
- place secrets in command-line arguments or URLs;
- transmit secrets to any network endpoint except as an environment variable to a user-authorized local consumer process.

BongTerm shall always use restrictive ACLs for any generated credential file, delete such files on session end, and log secret access by reference and consumer only — never by value.

### 37.10 Acceptance Criteria

The secrets and environment credential feature is acceptable when:

- references-only configuration validates, and plaintext secrets in committed config are flagged;
- env-file import moves values into the vault and rewrites entries to references;
- vault-backed env mode launches a shell/agent/MCP server with the correct environment and no plaintext `.env` on disk;
- automated tests confirm secrets never appear in argv, transcripts, scrollback, logs, or exports;
- a staged or committed `.env` is detected and can be blocked;
- launch-time disclosure lists the exact secret references to be injected;
- rotation updates all consumers, and a missing secret yields a clear, non-launching error.

---

### 37.11 Secret Exposure Classes

Environment-variable injection is sometimes necessary, but it is not equivalent to secret containment. Once a raw value is placed in a child process environment, that process and its descendants may read, print, persist, or transmit it. BongTerm shall classify secret exposure per consumer and prefer lower-risk classes.

| Exposure class | Description | Default status |
|---|---|---|
| **No secret access** | Consumer receives no secret values or references. | Default for all agents, MCP servers, plugins, and untrusted workspaces. |
| **Reference visible only** | Consumer/UI can see that a secret reference exists but not resolve the value. | Allowed for disclosure and configuration. |
| **Brokered operation** | BongTerm performs the sensitive operation; the consumer never receives the raw value. | Preferred where feasible. |
| **Ephemeral scoped token** | Short-lived, least-privilege token generated for one run/task. | Preferred for external APIs where supported. |
| **Read-only scoped credential** | Reduced-scope credential exposed to the process environment. | Allowed with explicit mapping and disclosure. |
| **Raw env injection** | Full secret value injected into the child environment. | High-risk last resort; requires explicit approval for untrusted or semi-trusted consumers. |

Additional requirements:

- child-process inheritance must be controlled where the platform allows;
- launch disclosure must show exposure class for every injected reference;
- high-risk raw injection requires a visible warning for agent/MCP consumers;
- missing secrets must fail closed and never launch with empty fallback values;
- secret audit logs record reference, exposure class, consumer, workspace, and timestamp, never value.

## 38. Concurrency, Failure, Recovery, and Versioning

### 38.1 Concurrency Model

BongTerm shall document and enforce an explicit concurrency model rather than leaving threading implicit:

- **Thread roles:** ConPTY reader(s), parser/grid worker, renderer thread, transcript/index workers, agent/MCP supervisors, and the UI thread. The terminal hot path (PTY → parse → grid → dirty-region → render) is isolated from agent, MCP, and I/O work per Section 6.3.
- **Bounded async:** worker pools are bounded; every cross-thread channel applies backpressure through bounded queues; no unbounded buffering is permitted.
- **Cancellation:** every long-running or background operation exposes cancellation, and cancellation plus metrics are co-located with lifecycle control at the same abstraction boundary (cross-ref 6.1.1 SOLID acceptance).
- **Ownership:** each pane owns its grid; rendering shares the glyph atlas; ConPTY/conhost process cost is accounted per pane (Section 25.2).

### 38.2 Application Failure Handling and Crash Reporting

- Rust panics and C++/WinRT interop faults (e.g., access violations) are contained at safe boundaries; a fault in one pane, agent, or MCP child must not crash the whole application where isolation is achievable.
- Local crash dumps and structured logs are written locally. Telemetry remains off by default; an explicit, opt-in "share diagnostics bundle" flow allows the user to send redacted crash and diagnostic data. There is no silent upload.
- Renderer device-loss recovery follows Section 36.3; storage corruption recovery follows Section 38.4.

### 38.3 Error Taxonomy and User Surfacing

Errors are classified as recoverable (retry/backoff), user-actionable (approval or configuration needed), or fatal (isolate and report). Each feature specifies its behavior for timeout, cancellation, crash, partial write, and rollback (cross-ref Section 34 Definition of Done).

### 38.4 Storage Durability and the Cache/Truth Split

This supersedes the over-broad phrasing of the original "SQLite is cache, Git is truth" principle:

- **Git is truth for repository, worktree, and PR state.** The corresponding SQLite state is a reconstructable cache; if it is lost or corrupt, it is rebuilt from Git.
- **Transcripts, command history, and the resource ledger are local source-of-truth.** They cannot be reconstructed from Git and must be crash-safe.
- SQLite shall run in WAL mode with periodic integrity checks. A corrupt Git-derived cache is rebuilt; a corrupt primary store is recovered from append-only chunks where possible and is never silently fabricated.

### 38.5 Versioning and Migration

- Settings, profile, and theme JSON carry a schema version. On load, BongTerm migrates forward, backs up the prior file, preserves unknown fields for forward compatibility, and never destructively overwrites configuration written by a newer version.
- Transcript, command-history, and resource-ledger formats are versioned, and migrations are covered by tests.
- MCP transport sessions perform protocol-version negotiation; `AgentAdapter.detect()` includes version detection and compatibility gating, analogous to MCP version pinning (cross-ref 20.6).
- Enterprise-locked policy fields are honored across upgrades and cannot be silently changed by a migration.

---

### 38.6 Retention, Private Sessions, and Local Data Controls

BongTerm shall provide explicit controls for local source-of-truth data that cannot be reconstructed from Git.

Requirements:

- default transcript, command-history, scrollback, diagnostic, and resource-ledger retention policies;
- per-workspace and global storage caps;
- per-workspace purge and export-before-purge flows;
- private session mode that disables persistent command history/transcripts unless explicitly saved;
- encrypted-at-rest option for transcripts, command history, and diagnostic bundles;
- redacted diagnostic preview before sharing;
- clear distinction between deleting UI history, deleting local source-of-truth logs, and deleting Git/worktree state;
- retention policy migration across schema upgrades.

The product shall document that local malware or an administrator-level adversary is out of scope (Section 35.6), while still using restrictive ACLs and optional encryption for stored data.

## 39. Execution Plan

### 39.1 Execution Principles

The execution plan is **gate-driven, not calendar-driven** (time estimates remain intentionally omitted per the document header). It sequences work so that the resource and safety contract is proven before scope expands, and it ties each phase exit to the acceptance criteria in Section 31, the budgets in Section 25, and the decision triggers in Section 33.

- Budgets and benchmarks precede feature growth: the harness and resource ledger exist before features that consume resources.
- High-uncertainty items are de-risked by spikes before they are committed to a phase.
- A phase exits only when its acceptance gate and the relevant CI gates pass.
- A feature that violates a Section 30.4 cut rule cannot enter the MVP phases regardless of progress.

### 39.2 Workstreams

Work is organized into parallel workstreams aligned to the SOLID module boundaries (Section 28.2). Each workstream owns its interfaces and contract tests and can progress independently behind ports.

| Workstream | Owns (modules) | Core deliverables |
|---|---|---|
| Terminal Core & Renderer | `terminal_core`, `renderer` | ConPTY host, VT/OSC parser, grid, scrollback, DirectX renderer, device-loss recovery |
| Resource Governance & Diagnostics | `storage` (ledger), cross-cutting | Benchmark harness, resource ledger, process-tree accounting, dashboards |
| Shell Integration & Command Blocks | `shell_integration`, `command_blocks`, `shell_intelligence` | OSC boundaries, reliability grading, blocks, suggestions/highlighting/autojump |
| Agent Cockpit | `agents` | Launcher, sidebar, transcripts, file-change tracking, lifecycle, approvals, replay |
| Worktrees & Environment Isolation | `worktrees`, `environment_isolation` | Serialized worktrees, lock detection, Git reconciliation, ports/env/Docker isolation |
| MCP & Context Optimizer | `mcp` | MCP manager, process governor, context optimizer |
| Security & Secrets | `security`, `settings` | Policy engine, workspace trust, secret vault, env-credential feature (Section 37), redaction |
| Platform, Accessibility & Packaging | `app_host`, `ui` | UIA accessibility, IME, DPI, MSIX/MSI signing, diagnostics/crash reporting |

### 39.3 De-Risking Spikes (run before committing dependent phases)

Each spike resolves an open question or decision trigger before the dependent work is scheduled.

| Spike | Question resolved | Linked trigger / open question |
|---|---|---|
| ConPTY OSC ordering reliability per shell | Are deterministic command blocks viable per profile? | Trigger: ConPTY OSC ordering unreliable (33.2); OQ 3 |
| Keystroke-to-glyph budget feasibility | Can the renderer hit the p99 latency budget on reference hardware? | Section 25.1; OQ 2 |
| UIA text-provider feasibility | How much accessibility work is required over a GPU grid? | Section 36.2 |
| Direct3D device-loss recovery | Can the renderer recover without state loss? | Section 36.3 |
| MCP shared-host RSS under load | Does one host per user stay within budget at N agents × M servers? | Trigger: managed MCP RSS > 100 MB (33.2) |
| Agent steering capability matrix | Which CLIs expose real IPC/API control? | Trigger: steering unsupported (33.2); OQ 4 |
| Rust-vs-C++ interop boundary | Where must thin C++ remain? | OQ 1 |

### 39.4 Phases and Gates

**Phase 0 — Foundations and spikes.** Stand up the benchmark harness and resource-ledger stub; build the ConPTY host, VT/OSC parser (fuzzed), grid, scrollback, and renderer skeleton with device-loss recovery; run the Section 39.3 spikes.
Exit gate: performance budgets are measurable on the documented reference machine; the parser passes fuzzing; forced device-loss recovers; spike findings are recorded against their decision triggers.

**Phase 1 — Usable terminal.** Profiles, settings/theming/keybindings, tabs/panes/layouts, search and command palette, workspace restore, shell integration and command blocks for PowerShell and Bash/WSL with reliability grading, and the resource dashboard with BongTerm/shell/conhost/process-tree accounting.
Exit gate: acceptance criteria 31.1 and 31.2 pass; Section 25 budgets are met and reported.

**Phase 2 — Agent observability MVP.** Agent launcher and profiles, sidebar, transcript capture, file-change tracking, lifecycle controls, approval gates, and replay with summarized context.
Exit gate: acceptance criteria 31.3 pass; unsupported steering is marked unavailable, not simulated.

**Phase 3 — Parallelism and isolation.** Serialized worktree creation, Git lock detection, state-vs-Git reconciliation, safe cleanup with diff snapshots, and MVP environment isolation (ports, env files, temp/cache/log directories, Docker Compose names, collision detection).
Exit gate: acceptance criteria 31.4 pass.

**MVP-0 release candidate gate.** After Phase 3, BongTerm may produce an internal/dogfood MVP-0 if Section 30.5 exit criteria pass. MCP Process Governor, Context Optimizer, plugin ecosystem, full environment isolation, Markdown review, Command Lens, and durable daemon remain excluded from MVP-0.

**Phase 4 — MCP governance and secrets.** MCP manager v1 (manual import/config, permissions, health checks, logs), MCP Process Governor (managed host, pool, JobObject limits, lifecycle), Context Optimizer v1, the secret vault and the Section 37 environment-credential feature, redaction, workspace trust, and dangerous-command policy.
Exit gate: acceptance criteria 31.5, 31.6, and 37.10 pass; the threat-model controls in Section 35 are reviewed.

**Phase 5 — Hardening and release.** UIA accessibility, IME, DPI/multi-monitor, MSIX/MSI signed packaging, diagnostics and opt-in crash reporting, performance hardening, the full security-test suite, and parser fuzzing wired into CI.
Exit gate: full Section 31 acceptance; signed installer; benchmark report (or documented budget exceptions) published in release notes.

**Phase 6 — Post-MVP (deferred).** Markdown review, Command Lens, database branching, durable session daemon, plugin marketplace, collaboration, and cross-platform ports — each admitted only after terminal correctness and resource budgets are already met (Section 3.3).

### 39.5 Phase Gates: Definition of Ready and Done

A phase is **ready** to start when its predecessor's exit gate has passed and its spikes (if any) are resolved. A phase is **done** when every feature in it satisfies the Section 34 Definition of Done, passes the proportional SOLID/architecture review (Section 28.3), and clears the phase's acceptance and CI gates. No phase may carry an unbudgeted feature, a hidden background process, or a ledger-bypassing resource (Section 30.4).

### 39.6 Risk Burn-Down Mapping

| Risk / open question | Resolved or contained in |
|---|---|
| OQ 1 Rust-first vs C++ interop | Phase 0 spike |
| OQ 2 reference hardware | Phase 0 (budgets defined and measured) |
| OQ 3 reliable command-block shells | Phase 0 spike → Phase 1 grading |
| OQ 4 agent IPC/steering | Phase 0 spike → Phase 2 capability levels |
| OQ 5 mandatory MCP transports | Phase 4 |
| OQ 6 WSL2 runner MVP vs post-MVP | Phase 3 decision; default post-MVP |
| OQ 7 minimum remote/SSH workflow | Phase 3 (SSH profile) → Phase 6 runners |
| OQ 8 default strictness of production safety mode | Phase 4 |
| OQ 9 untrusted workspace config sandbox | Phase 4 (workspace trust + threat model) |
| OQ 10 marketplace licensing policy | Phase 6 |
| Enforcement overclaim risk | Section 17.5, validated every phase |
| MVP scope creep | Section 30.5 and MVP-0 release gate |
| MCP RSS trigger | Phase 0 spike → Phase 4 enforcement |
| Worktree lock-failure trigger | Phase 3 → promote runners if exceeded |
| Core RSS / VRAM triggers | Phase 0 harness, enforced every phase |

### 39.7 CI Gate Enforcement Schedule

| CI gate | Becomes blocking at |
|---|---|
| Parser fuzzing, allocations/copies-per-MB, keystroke-to-glyph latency | Phase 0 |
| Terminal compatibility matrix (29.3) | Phase 1 |
| RSS/VRAM/process-tree budgets and attribution (25.1-25.2) | Phase 1, enforced thereafter |
| Architecture fitness functions and contract tests (28.3, 29.7) | From Phase 1, expanded each phase |
| Agent supervision and policy-bypass tests (29.5) | Phase 2 |
| Worktree-safety and environment-isolation tests (29.2) | Phase 3 |
| MCP process-scaling, secret-leak, and redaction tests (29.5, 37.10) | Phase 4 |
| Accessibility, IME, and device-loss tests (36) | Phase 5 |

### 39.8 Dogfooding and Product Success Metrics

The existing acceptance criteria measure engineering correctness; the product is validated separately through dogfooding from Phase 2 onward. Suggested success signals (to be calibrated on the reference fleet, no targets asserted here):

- crash-free session rate and mean session length;
- share of agent tasks completed end-to-end without manual terminal fallback;
- worktree create→merge success rate and median locks/collisions per worktree;
- resource budgets held in real usage (RSS/VRAM/process count within Section 25);
- secret-leak incidents in transcripts/exports (target: zero, measured by the redaction test corpus);
- adoption as the user's default terminal (replacement of the prior terminal) among dogfood users.

### 39.9 Persona Coverage by Phase

The MVP intentionally serves the Windows Developer and Agentic Workflow Developer personas first. The Linux-to-Windows Power User is partially served in MVP (autosuggestions, syntax highlighting, autojump, panes/sessions) with Command Lens deferred to Phase 6; the Technical Manager / Reviewer persona is largely served in Phase 6 (Markdown review, deep diff review); the Enterprise Platform / Security persona is served progressively, with its core needs (local-first, signed installer, secret vault, audit logs, workspace trust, threat model) landing by Phase 4-5.

---

## 40. Final Product Summary

BongTerm shall be a native Windows terminal for serious developers. It should feel fast, local, secure, and visually polished while providing structured workflows that classic terminals do not offer. Its strongest strategic advantage is not only being agent-aware, but making agent work **safe, parallel, inspectable, reversible, and mergeable** through Git worktrees, environment isolation, MCP governance, command blocks, transcripts, and explicit approvals.

The product should be built as a native Windows terminal first, a developer command center second, and an agentic workspace third. Terminal correctness, performance, safety, and workflow composability are the core product contract.
