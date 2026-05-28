# BongTerm PRD v7 — Critical Analysis Resolved

**Document type:** Product Requirements Document + Engineering Build Guidance  
**Product name:** BongTerm  
**Target platform:** Windows 11 first  
**Product category:** Resource-governed terminal cockpit for Windows-first agentic development  
**Supersedes:** Earlier BongTerm PRD drafts that treated the terminal core, MCP host pool, multi-agent support, Markdown review, Command Lens, and plugin marketplace as near-term scope  
**Primary audience:** Product, engineering, design, QA, security, release, enterprise review, and open-source governance teams  

---

## 0. Critical Analysis Resolution Summary

The critical analysis is accepted as a hardening input. The prior PRD was strong on security, SOLID principles, local-first behavior, and resource accounting, but it still had three unresolved structural risks:

1. **Scope concentration:** too many foundational systems were placed too early.
2. **NIH risk:** the plan implied BongTerm would write a custom terminal parser, grid, renderer, scrollback engine, UI framework, accessibility provider, MCP host, and agent adapters from scratch.
3. **Adoption gap:** developer-facing features that create daily retention, such as inline NL→command, failed-command explanation, smart history, snippets, and job notifications, were underweighted.

v7 resolves those risks by changing the product plan in five ways:

| Decision | v7 Resolution |
|---|---|
| Terminal core from scratch | **Rejected for MVP-0.** Reuse or fork proven terminal components where licensing permits. Custom terminal internals are allowed only after benchmarks prove reuse cannot meet BongTerm requirements. |
| MCP shared host pool in MVP | **Deferred.** MVP-0 uses simple MCP supervision with one process per enabled server/agent where needed, resource accounting, hard limits, and no pooling. Pooling becomes v1.1+ after usage data. |
| Seven agent profiles in MVP | **Rejected.** MVP-0 supports Claude Code and Codex CLI as built-in first-party profiles. Other agents use community/imported profiles until adapter contracts stabilize. |
| Custom raw-Win32 UI everywhere | **Rejected.** Terminal hot path remains native and performance-isolated. Non-hot-path UI may use WinUI 3 or Slint to avoid hand-rolled accessibility, DPI, IME, and layout defects. |
| “Everything MVP” | **Rejected.** MVP-0 becomes a narrow Windows terminal cockpit for observable agent execution, command blocks, resource governance, and retention-critical command UX. |

---

## 1. Revised Product Thesis

BongTerm is a Windows-first terminal cockpit for developers who use CLI-based AI coding agents and need the terminal, agents, MCP servers, and child processes to remain observable, bounded, and safe.

The product shall not try to win by being the biggest terminal. It shall win by being the clearest, safest, most resource-transparent terminal for running agent-assisted development workflows on Windows.

### 1.1 One-Sentence Positioning

> BongTerm is a resource-governed Windows terminal that lets developers run CLI agents, inspect commands, explain failures, attach context, and track resource cost without losing terminal correctness or local control.

### 1.2 Primary Wedge

The first durable wedge is **agent-aware terminal observability**, not a full IDE, not a cloud workspace, not a Markdown review suite, and not a plugin marketplace.

MVP-0 must prove:

- BongTerm can behave as a reliable daily terminal.
- BongTerm can make command output more actionable through blocks, explanations, search, snippets, and inline command generation.
- BongTerm can launch and observe selected CLI agents without promising unsupported mid-session control.
- BongTerm can show total process-tree cost, not just BongTerm.exe cost.
- BongTerm can remain within explicit CPU, RSS, VRAM, and latency budgets.

---

## 2. Product Scope

### 2.1 MVP-0 Scope

MVP-0 shall include only the features required to prove the wedge.

| Area | MVP-0 Requirement |
|---|---|
| Terminal host | PowerShell 7+, Windows PowerShell, CMD, WSL default distro, Git Bash, SSH profile launch. |
| Terminal rendering | Reused/forked parser/grid/scrollback where possible; WGPU/Direct3D-backed renderer; dirty-region rendering; shared glyph atlas. |
| Workspace | Tabs, split panes, pane focus, pane resize, session restore for local windows only. No detach daemon. |
| Command blocks | Shell-integrated command blocks for PowerShell and Bash-compatible shells where reliable; fallback heuristic mode with confidence labels. |
| Cmd-K inline NL→command | Natural-language-to-command prompt action with shell-aware preview, never silent execution. |
| Failed-command explainer | Non-zero exit blocks expose an **Explain** action using selected block output and command context. |
| Smart history | Query filters: `cwd:`, `branch:`, `agent:`, `exit:`, `time:`, `shell:`, `duration:`. |
| Snippets | Workspace/global snippets with `${param:name}` placeholders and confirmation before run. |
| Background jobs | Run command in background pane/task; desktop toast on completion/failure. |
| Inline errors | Recognize common compiler/test/log patterns and make file:line references clickable. |
| Agent support | First-party built-in profiles for Claude Code and Codex CLI only; observe stdout/stderr, transcript, process tree, exit state, resource usage. |
| MCP support | Manual MCP JSON import/config, visible permissions, one-process-per-server supervision, logs, JobObject caps. No shared MCP pool in MVP-0. |
| Resource ledger | Per-pane, shell, agent, MCP, plugin, and child-process CPU/RSS/VRAM/I/O attribution where OS APIs allow. |
| Security | DPAPI/Windows Credential Manager secret storage, secret redaction, dangerous-command warnings, signed installer, EDR-friendly process behavior. |
| UX foundation | Onboarding, empty/error/loading states, keyboard shortcut map, notification taxonomy, telemetry opt-in with redaction preview. |
| Accessibility | Narrator reads active terminal text, scrollback, command blocks, tabs/panes, and main controls. Rich text/UIA patterns are post-MVP unless inherited from reused components. |

### 2.2 Explicitly Out of MVP-0

The following are deferred:

- custom terminal parser from scratch;
- custom scrollback engine from scratch unless reused components fail benchmarks;
- shared `bongterm-mcp-host.exe` process pool;
- MCP marketplace;
- OpenCode, Gemini CLI, Aider, Copilot CLI, and arbitrary local-agent first-party adapters;
- full agent pipeline automation;
- strong mid-session steering for interactive CLIs;
- durable session daemon / detach-reattach;
- Markdown review suite;
- Command Lens learning system;
- runbook/notebook mode;
- database branching integrations;
- plugin marketplace;
- PR comment browser;
- team collaboration/cloud sync;
- Windows Server/RDP/mirrored WSL2 as required support.

### 2.3 v1 Scope

v1 expands only after MVP-0 meets performance, stability, accessibility, and dogfood gates.

| Area | v1 Candidate |
|---|---|
| Worktrees | Safe Git worktree creation, serialized operations, lock detection, cleanup, confidence-tagged attribution. |
| Attachments | Drag/drop file and selected block attachment into agent context. |
| Agent replay | Edit transcript/context bundle before re-running a failed or incomplete agent task. |
| Conflict UX | Basic diff conflict panel when two agent tasks modify the same file. |
| Dev Containers | `devcontainer.json` runner profile and WSL2-friendly workflows. |
| Branch graph | Git branch graph with worktree overlays. |
| Cross-shell translator | Bash ↔ PowerShell suggestions and previews. |
| HTTP/REST pane | Saved requests, JSON viewer, command export to curl/PowerShell. |

### 2.4 v1.1+ Scope

- shared MCP host/pool;
- MCP HTTP bridge and stdio multiplexing;
- advanced plugin system;
- DB query pane;
- database branch provisioning;
- runbook/notebook mode;
- PR comment browser;
- collaboration/sharing;
- additional first-party agent adapters.

---

## 3. Team, Budget, and Feasibility Assumptions

The implementation plan must be explicit about team profile because BongTerm’s feasible scope differs drastically between a solo builder and a specialist team.

### 3.1 Team Profiles

| Team Profile | Feasible Product Target |
|---|---|
| Solo builder | MVP-0 only, built on maximum reuse. No custom terminal core, no shared MCP pool, no marketplace, no full accessibility provider from scratch. |
| Small team, 3–5 engineers | MVP-0 plus limited v1 items. Requires at least one systems/terminal engineer, one Windows/UI engineer, one product/frontend engineer, one QA/security-minded engineer. |
| Specialist team, 6–10+ | v1.1 infrastructure items may run in parallel: MCP pool, richer worktree attribution, accessibility hardening, additional agents, plugin SDK. |

### 3.2 Required Capability Areas

- Terminal/PTY/VT expertise.
- GPU/text rendering expertise.
- Windows process/security expertise: ConPTY, JobObjects, ETW, DPAPI, code signing, SmartScreen, EDR behavior.
- Agent/MCP integration expertise.
- UI/accessibility expertise.
- Release/installer/telemetry/privacy expertise.

### 3.3 Build Principle

If team size is below the required specialization level, the PRD must shrink. The scope is not allowed to expand to compensate for limited resources.

---

## 4. Architecture Strategy

### 4.1 Reuse-First Terminal Core Policy

BongTerm shall not begin by implementing a custom terminal parser, terminal grid, scrollback store, renderer, and benchmark harness from scratch.

MVP-0 shall evaluate these reuse paths:

| Layer | Preferred Strategy | Fallback |
|---|---|---|
| VT/ANSI/OSC parser | Reuse `vte`, `termwiz`, or another mature Rust parser if license-compatible. | Build a narrow parser only for unsupported sequences, behind the same trait. |
| Grid/scrollback | Reuse/fork `termwiz`, WezTerm core components, or another mature terminal model if feasible. | Build minimal grid/scrollback only after spike proves reuse infeasible. |
| Rendering | WGPU with Direct3D backend on Windows, shared glyph atlas, dirty-region renderer. | Direct3D/DirectWrite-specific backend only if WGPU fails latency/VRAM gates. |
| Accessibility | Reuse mature terminal accessibility patterns/components where possible. | Minimal UIA provider for active text, scrollback, and command-block navigation only. |
| UI shell | WinUI 3 or Slint for non-hot-path surfaces. | Raw Win32 only where required for performance or OS integration. |

### 4.2 Native Boundary Policy

BongTerm shall optimize inside documented Windows user-mode boundaries.

Allowed:

- ConPTY;
- Win32;
- Windows JobObjects;
- Direct3D/WGPU Direct3D backend;
- DirectWrite where needed for text shaping/rasterization;
- DPAPI/Windows Credential Manager;
- named pipes/local RPC;
- ETW/performance counters where appropriate.

Forbidden:

- DLL injection into child processes;
- hidden-console scraping;
- undocumented `ntdll` syscall reliance;
- process hollowing;
- kernel-mode display drivers;
- global keyboard hooks for terminal capture;
- silent fallbacks to EDR-hostile techniques.

### 4.3 SOLID Architecture Contract

BongTerm remains SOLID-compliant after the v7 scope changes.

| Principle | Concrete BongTerm Rule |
|---|---|
| SRP | Terminal core owns PTY, parser, grid, scrollback. It does not own agents, MCP, Git, UI dashboards, policy decisions, or telemetry export. |
| OCP | New renderers, shells, agents, MCP transports, and plugin capabilities are added through interfaces/registries. |
| LSP | Mock, local, remote, and future adapters must pass the same contract tests before substitution. |
| ISP | Narrow interfaces only. No global `AppContext`, no monolithic `WorkspaceService`, no god `PerformanceManager`. |
| DIP | High-level workflows depend on traits such as `TerminalSession`, `RendererBackend`, `AgentAdapter`, `McpTransport`, `ResourceLimiter`, `PolicyEvaluator`, and `SecretStore`. |

---

## 5. System Architecture

```text
BongTerm
├─ App Shell / Non-Hot UI
│  ├─ WinUI 3 or Slint shell
│  ├─ command palette
│  ├─ settings surfaces
│  ├─ sidebars/dashboards
│  ├─ onboarding and telemetry consent
│  └─ accessibility bridge
│
├─ Terminal Core
│  ├─ ConPTY host
│  ├─ reused/forked VT parser
│  ├─ terminal grid model
│  ├─ virtualized scrollback
│  ├─ shell integration bridge
│  └─ command-boundary confidence classifier
│
├─ Renderer
│  ├─ WGPU / Direct3D backend
│  ├─ DirectWrite-assisted glyph shaping where needed
│  ├─ shared glyph atlas
│  ├─ dirty-region tracker
│  ├─ VRAM budget manager
│  └─ software/RDP fallback path
│
├─ Workspace Layer
│  ├─ tabs
│  ├─ panes
│  ├─ layouts
│  ├─ local restore
│  └─ background jobs
│
├─ Developer Command Layer
│  ├─ command blocks
│  ├─ Cmd-K inline NL→command
│  ├─ failed-command explainer
│  ├─ smart history
│  ├─ snippets
│  ├─ clickable errors
│  └─ live pane filter
│
├─ Agent Runtime
│  ├─ Claude Code adapter
│  ├─ Codex CLI adapter
│  ├─ generic profile importer
│  ├─ transcript capture
│  ├─ resource attribution
│  ├─ lifecycle controls
│  └─ replay package exporter
│
├─ MCP Runtime — MVP-0
│  ├─ manual JSON import
│  ├─ one server process per configured server when active
│  ├─ JobObject caps
│  ├─ logs/health
│  ├─ visible permissions
│  └─ context/tool-schema optimizer
│
├─ Git / Worktree Runtime — v1
│  ├─ serialized git operations
│  ├─ lock detection
│  ├─ worktree status
│  ├─ high/medium/low confidence attribution
│  └─ cleanup safeguards
│
├─ Resource Governance
│  ├─ process ledger
│  ├─ JobObject limiter
│  ├─ admission controller
│  ├─ budget policy evaluator
│  ├─ CPU/RSS/VRAM/I/O reporting
│  └─ diagnostic snapshot export
│
├─ Security
│  ├─ DPAPI / Windows Credential Manager
│  ├─ secret scanner/redactor
│  ├─ dangerous command policy
│  ├─ workspace trust
│  ├─ signed update validation
│  └─ audit log
│
└─ Persistence
   ├─ SQLite metadata
   ├─ append-only terminal chunks
   ├─ JSON settings/profiles/keybindings
   ├─ transcript bundles
   └─ migration manager
```

---

## 6. Performance and Reference Hardware

### 6.1 Reference Hardware

Budgets shall be measured on a pinned reference machine and separately tracked on higher-end and lower-end machines.

| Category | Reference Requirement |
|---|---|
| OS | Windows 11 24H2, current stable updates. |
| CPU | AMD Ryzen 7 7840U-class or Intel Core Ultra 5-class laptop CPU. |
| RAM | 16 GB. |
| GPU | Integrated GPU comparable to Iris Xe / Radeon 780M. |
| Storage | NVMe SSD. |
| Display | 1440p internal or external display; additional high-DPI run tracked separately. |
| Shell baseline | PowerShell 7 current stable. |

### 6.2 Numeric Budgets

| Axis | MVP-0 Target | Gate |
|---|---:|---|
| Warm startup to first prompt | ≤ 300 ms | CI perf test + local benchmark. |
| Startup with shell integration | ≤ 800 ms | CI perf test. |
| Keystroke-to-glyph p99 | ≤ 16 ms | Input latency benchmark. |
| Target keystroke-to-glyph p99 on 120 Hz display | ≤ 8 ms | Stretch benchmark. |
| Idle CPU, single pane | ≤ 0.1% over 60 s | CI perf machine + manual validation. |
| BongTerm core RSS, 1 pane | ≤ 120 MB | Process ledger. |
| Additional pane RSS | ≤ 25 MB | Pane-scaling test. |
| Additional pane VRAM | ≤ 8 MB | GPU memory telemetry where available. |
| Total VRAM ceiling | ≤ 256 MB default | Eviction/backpressure required. |
| MCP server RSS cap | ≤ 60 MB default per process unless user overrides | JobObject enforced. |
| Out-of-process plugin RSS cap | ≤ 40 MB default unless capability requires more | JobObject enforced. |
| `cat` / stream 10 MB output | ≥ 50 MB/s sustained where ConPTY permits | Throughput benchmark. |
| Four-pane idle battery contribution | ≤ 1%/hour target | Manual battery test. |

### 6.3 Performance Tests Required for Every Release

- Terminal throughput: 1 MB, 10 MB, 100 MB output.
- Many panes: 1, 4, 8, 12 panes.
- Long scrollback: 100k, 500k, 1M logical lines.
- Alternate screen apps: vim, less, htop equivalent, git interactive tools.
- WSL shell output.
- PowerShell high-output commands.
- Command-block boundary reliability.
- GPU memory growth with panes/tabs.
- Resource ledger correctness.
- Agent launch/resource attribution.
- MCP launch/resource attribution.

---

## 7. Resource Governance

### 7.1 Total Process-Tree Accounting

BongTerm shall account for the full runtime tree:

```text
1 BongTerm process
+ T × conhost/ConPTY host processes
+ T × shell processes
+ N × agent processes
+ M × MCP server processes
+ P × plugin processes
+ background jobs
+ helper processes
```

The Resource Dashboard shall display:

- process count;
- CPU;
- RSS/private working set;
- GPU/VRAM where available;
- disk I/O;
- network I/O where available;
- owning pane/session/workspace/agent/MCP/plugin;
- JobObject policy;
- restart/kill controls where safe.

### 7.2 Admission Control

Before launching a pane, background job, agent, MCP server, or plugin, BongTerm shall estimate projected resource impact.

If projected resource use exceeds configured budget, BongTerm shall present:

- launch anyway;
- launch with lower limits;
- disable selected MCP/plugin components;
- defer launch;
- open diagnostics.

### 7.3 MCP Governance Split

| System | Owns | Does Not Own |
|---|---|---|
| Context Optimizer | Tool-schema pruning, token budget, prompt context visibility, agent-specific MCP allowlist. | Process pooling, memory caps, server lifecycle. |
| MCP Process Governor | process launch, JobObject limits, health, logs, restarts, shutdown. | Deciding which tool schemas enter the LLM context. |

MVP-0 shall not pretend context pruning reduces RAM. It reduces tokens and model confusion only.

---

## 8. Agent Strategy

### 8.1 MVP-0 Built-In Agent Profiles

Built-in support:

- Claude Code;
- Codex CLI.

Community/import-only support:

- OpenCode;
- Gemini CLI;
- Aider;
- Copilot CLI;
- local/custom agents.

### 8.2 AgentAdapter Contract

Every first-party adapter must implement:

```rust
trait AgentAdapter {
    fn id(&self) -> AgentId;
    fn display_name(&self) -> &'static str;
    fn discover(&self) -> DiscoveryResult;
    fn auth_state(&self) -> AuthState;
    fn build_command(&self, launch: AgentLaunchSpec) -> ProcessSpec;
    fn classify_output(&self, chunk: OutputChunk) -> Vec<AgentEvent>;
    fn summarize_exit(&self, exit: ExitState) -> AgentExitSummary;
    fn capabilities(&self) -> AgentCapabilities;
}
```

Capabilities must explicitly state whether the adapter supports:

- non-interactive prompt mode;
- reliable transcript capture;
- current command detection;
- file-change detection;
- safe interruption;
- supported mid-session steering;
- MCP configuration handoff;
- cost/token reporting.

If a capability is not supported by upstream CLI/API, the UI must label it unavailable instead of simulating it with fragile stdin injection.

### 8.3 Agent Lifecycle Controls

MVP-0 supports:

- launch;
- stop/terminate;
- restart with same context;
- export transcript/context bundle;
- explain current/final output;
- resource limit display;
- process-tree kill with confirmation;
- dangerous command approval when BongTerm can detect it.

MVP-0 does not promise true mid-session steering for interactive CLIs.

---

## 9. Worktree Strategy

### 9.1 v1, Not MVP-0

Worktree orchestration is v1 unless MVP-0 stability gates are already met.

### 9.2 Worktree Safety Rules

- Worktree operations must be serialized per repository.
- BongTerm must detect `.git/config.lock`, `.git/index.lock`, and `.git/worktrees/*/index.lock` before write operations.
- Failed or stale lock situations must produce a repair suggestion, not automatic deletion unless explicitly approved.
- Git CLI output is the source of truth. SQLite is cache.
- File-system watcher attribution is heuristic only.
- Attribution labels: `git-confirmed`, `process-associated`, `watcher-associated`, `mixed`, `unknown`.
- Dependency reuse must be package-manager-aware. No automatic cross-worktree `node_modules` symlink.
- pnpm global/virtual-store modes may be suggested only when project policy permits.
- Large monorepos must show install/cache impact before creating additional worktrees.

### 9.3 Worktree Edge-Case Tests

- concurrent worktree creation attempts;
- stale lock files;
- AV-delayed file events;
- ignored files modified by agent;
- same file touched by two agents;
- package install in multiple worktrees;
- branch deletion while worktree exists;
- submodule project;
- Git LFS project;
- worktree cleanup after agent crash.

---

## 10. Shell Integration and Command Blocks

### 10.1 Reliability Model

Command blocks shall have a confidence score.

| Level | Meaning |
|---|---|
| High | Shell integration emitted reliable boundary markers and exit code. |
| Medium | Partial boundary metadata available; exit code or cwd may be inferred. |
| Low | Heuristic block boundaries based on prompt/output parsing. |
| Unsupported | Shell/profile combination cannot produce blocks reliably. |

### 10.2 Shell Support Priority

| Shell/Profile | MVP-0 Support |
|---|---|
| PowerShell 7 | High priority. |
| Windows PowerShell | High priority. |
| Bash via WSL/Git Bash | High priority. |
| Zsh/Fish via WSL | Medium priority. |
| CMD | Low-confidence heuristic only. |
| Oh My Posh / Starship / PSReadLine complex configs | compatibility-tested but confidence-labeled. |

### 10.3 Block Actions

- copy command;
- copy output;
- copy command + output;
- rerun;
- explain failure;
- attach to agent;
- save as snippet;
- background rerun;
- filter output;
- export diagnostic package.

---

## 11. UX Requirements

### 11.1 First-Launch Onboarding

First launch must include:

1. choose default shell;
2. import Windows Terminal profiles where available;
3. select theme/contrast;
4. enable/disable shell integration;
5. choose telemetry mode: off by default, diagnostic opt-in, or full local-only diagnostics;
6. detect Claude Code / Codex CLI installations;
7. configure secret storage;
8. show resource-budget defaults.

### 11.2 Empty, Loading, and Error States

Every major surface must define:

- empty state;
- loading state;
- partial failure state;
- permission denied state;
- unsupported capability state;
- degraded mode state;
- recovery action.

Examples:

| Surface | Required State |
|---|---|
| Agent panel | “No supported agent detected” with install/configure/import actions. |
| MCP panel | “No MCP servers configured” with JSON import and security explanation. |
| Command blocks | “Shell integration unavailable; heuristic mode active.” |
| Resource dashboard | “Metric unavailable from OS” rather than blank values. |
| Renderer | “GPU path degraded; using software/RDP fallback.” |

### 11.3 Design System

MVP-0 design tokens shall define:

- typography scale;
- spacing scale;
- radius scale;
- motion duration/easing;
- color tokens;
- terminal foreground/background/selection/cursor tokens;
- semantic status colors;
- danger/production mode tokens;
- focus ring tokens;
- high-contrast mapping;
- reduced-motion alternatives.

### 11.4 Keyboard Shortcut Map

Minimum default shortcuts:

| Action | Shortcut |
|---|---|
| Command palette | Ctrl+Shift+P |
| Inline NL→command | Ctrl+K |
| New tab | Ctrl+Shift+T |
| Split pane | Alt+Shift+D |
| Close pane | Ctrl+Shift+W |
| Search current pane | Ctrl+F |
| Smart history | Ctrl+R |
| Explain last failed command | Ctrl+Shift+E |
| Open resource dashboard | Ctrl+Shift+R |
| Attach file/context | Ctrl+Shift+A |
| Toggle background jobs | Ctrl+Shift+J |

### 11.5 Notification Taxonomy

| Event | Notification Type |
|---|---|
| Background job completed | Desktop toast + pane badge. |
| Background job failed | Desktop toast + red pane badge. |
| Agent waiting for approval | Persistent sidebar badge + optional toast. |
| Resource budget exceeded | Modal if launch-blocking; otherwise dashboard warning. |
| Dangerous command detected | Inline confirmation block. |
| MCP server crashed | Sidebar badge + logs link. |
| Telemetry export requested | Modal with redaction preview. |

### 11.6 Telemetry Opt-In UX

Telemetry is off by default. Diagnostic export must show:

- exact files/logs included;
- command history redaction preview;
- secret redaction summary;
- process metrics included;
- OS/hardware metadata included;
- “copy to clipboard” and “save local zip” options;
- explicit upload disabled unless user opts in.

---

## 12. Developer-POV Features

### 12.1 Tier A: MVP-0 or v1

| Feature | Scope |
|---|---|
| Cmd-K inline NL→command | MVP-0. Generate command preview; user must accept before insertion/run. |
| Failed-command explainer | MVP-0. Uses current block and local transcript selection. |
| Background job runner | MVP-0. Run long command and notify on finish. |
| Snippet library | MVP-0. Parameterized snippets with workspace/global scopes. |
| Smart history search | MVP-0. Supports `cwd:`, `branch:`, `agent:`, `exit:`, `time:` filters. |
| Drag/drop file attachment | v1 if not ready for MVP-0. |
| Clickable compiler/log errors | MVP-0 for common patterns; extensible later. |

### 12.2 Tier B: v1.1

- HTTP/REST pane;
- DB query pane;
- Dev Container runner profile;
- branch graph with worktree overlay;
- agent task pipeline;
- cross-shell translator;
- replay editor.

### 12.3 Tier C: Post-v1

- runbook/notebook mode;
- clipboard history with secret redaction;
- PR comment browser;
- role-based keybinding profiles;
- HTTP fetch + jq pipeline templates.

---

## 13. Plugin and Extension Model

### 13.1 Plugin Policy

No Node extension host in MVP-0 or v1.

### 13.2 Tier 1 Plugins: WASM

Allowed for:

- themes;
- syntax classifiers;
- exporters;
- safe parsers;
- snippet transformers.

Constraints:

- WASI/component model where practical;
- default 32 MB linear memory limit;
- no ambient filesystem/network access;
- capability manifest required;
- deterministic contract tests.

### 13.3 Tier 2 Plugins: Out-of-Process Native Adapters

Allowed for:

- agent launchers;
- MCP installers;
- task providers;
- external tool bridges.

Constraints:

- signed or explicitly marked unsigned;
- JobObject-capped;
- permission manifest;
- IPC schema versioning;
- crash isolation;
- resource attribution.

### 13.4 Minimal Plugin Interface

```rust
trait BongTermPlugin {
    fn manifest(&self) -> PluginManifest;
    fn capabilities(&self) -> Vec<Capability>;
    fn initialize(&mut self, host: PluginHostHandle) -> Result<()>;
    fn handle_event(&mut self, event: PluginEvent) -> Result<Vec<PluginAction>>;
    fn shutdown(&mut self) -> Result<()>;
}
```

Plugins must never receive direct access to terminal grid internals, secret stores, process handles, or UI internals.

---

## 14. MCP Requirements

### 14.1 MVP-0 MCP

- manual JSON import;
- server command preview;
- permission summary;
- secret reference through vault only;
- visible process tree;
- logs;
- health status;
- JobObject caps;
- no shared pool;
- no marketplace;
- no auto-install from arbitrary registry without user approval.

### 14.2 v1.1 MCP Host Pool

The shared host pool may be built only after MVP-0 usage shows that MCP process cost is a real adoption blocker.

Required before implementation:

- measured MCP fleet data;
- transport compatibility matrix;
- stdio bridge security design;
- HTTP loopback authentication;
- version pinning;
- integrity hashing;
- rollback strategy;
- crash recovery;
- process cap policy.

---

## 15. Accessibility

### 15.1 MVP-0 Accessibility Acceptance

MVP-0 passes accessibility only when:

- Narrator can read active terminal text;
- Narrator can navigate scrollback;
- tabs and panes expose accessible names;
- command blocks expose command, exit code, and summary;
- main controls are keyboard reachable;
- focus order is deterministic;
- high contrast mode works;
- reduced motion mode works.

### 15.2 Deferred Accessibility

Post-MVP:

- rich text UIA patterns for every terminal span;
- advanced semantic navigation;
- full screen-reader parity with mature IDEs;
- accessibility for plugin-rendered custom UI.

Where possible, BongTerm should reuse mature accessibility implementations instead of building a full GPU-grid UIA provider from scratch.

---

## 16. OS and Environment Support Matrix

### 16.1 MVP-0 Required

| Environment | Status |
|---|---|
| Windows 11 24H2 | Required. |
| Windows 11 23H2 | Best effort if test capacity permits. |
| PowerShell 7 | Required. |
| Windows PowerShell | Required. |
| CMD | Basic launch and heuristic blocks only. |
| WSL2 default distro | Required for launch/basic terminal. |
| Git Bash | Required. |
| SSH process launch | Required; deep remote session management deferred. |

### 16.2 Post-MVP / Best Effort

- Windows 10 22H2;
- Windows Server;
- RDP rendering fallback certification;
- WSL2 mirrored networking;
- multi-monitor mixed-DPI certification beyond smoke tests;
- enterprise VDI.

---

## 17. CI/CD, Test Matrix, and Release Gates

### 17.1 CI Pipeline

Every PR must run:

- Rust/C++/UI build;
- unit tests;
- parser/grid tests;
- shell integration tests where possible;
- SOLID architecture dependency checks;
- security linting;
- license/SBOM generation;
- secret scanning;
- packaging smoke test.

Nightly must run:

- performance suite;
- pane scaling;
- MCP/agent process scaling;
- GPU memory tests;
- accessibility smoke tests;
- Windows Defender/EDR-friendly behavior smoke tests;
- installer/update tests;
- crash recovery tests.

### 17.2 Release Channels

| Channel | Purpose |
|---|---|
| Dev | Internal builds, diagnostics enabled by default. |
| Canary | Early adopters, opt-in telemetry prompt. |
| Beta | Feature-complete release candidate. |
| Stable | Signed, documented, privacy-preserving release. |

### 17.3 Crash Recovery UX

After a crash/panic:

- restart shows recovery screen;
- affected panes are listed;
- transcripts/log chunks are preserved where possible;
- user can restore, discard, or export diagnostics;
- diagnostics are local unless explicitly shared;
- suspected plugin/adapter culprit is shown if attributable.

---

## 18. Security, Legal, and Distribution

### 18.1 Third-Party CLI Policy

BongTerm shall detect and launch user-installed CLIs. It shall not bundle Claude Code, Codex CLI, Gemini CLI, Copilot CLI, or other third-party agent tools unless legal review explicitly approves redistribution.

Marketing screenshots and docs must avoid implying official partnership unless authorized.

### 18.2 SmartScreen Plan

Because new signed Windows binaries may trigger reputation warnings, release planning must include:

- EV or OV code signing decision;
- release reputation warm-up plan;
- download page trust copy;
- checksum/signature verification instructions;
- enterprise deployment notes.

### 18.3 EDR Validation Plan

Before enterprise beta:

- test with Windows Defender;
- test with at least two common enterprise EDR products where accessible;
- document child-process supervision behavior;
- avoid injection/scraping/hooking techniques;
- provide allowlisting guidance;
- produce a security whitepaper explaining ConPTY, JobObjects, process supervision, and secret handling.

### 18.4 Trademark and Naming

Before public launch:

- search USPTO;
- search EUIPO;
- search Indian trademark databases if targeting India-first users;
- search GitHub/npm/crates/domain availability;
- verify “BongTerm” is usable in target geographies and not confused with existing terminal products.

### 18.5 Open-Source Maintenance Policy

If Apache-2.0 core is used:

- third-party agent profile PRs require test fixtures;
- maintainers may reject profiles for legal/security/maintenance reasons;
- community profiles are marked unsupported unless adopted;
- stale profiles are auto-flagged;
- security review required for any profile that launches external tools.

---

## 19. Business and Distribution

### 19.1 Business Model Decision

The PRD shall not defer business model indefinitely because it affects SBOM, telemetry, signing, support, and enterprise readiness.

Recommended split:

| Tier | Contents |
|---|---|
| Free/Open Core | Terminal, panes, command blocks, smart history, snippets, local resource dashboard, basic agent launch. |
| Pro | Advanced agent observability, replay bundles, cost ledger, deeper context attachments, advanced search. |
| Team/Enterprise | Policy packs, audit exports, SSO/RBAC if later added, managed settings, signed profile catalogs, enterprise support. |

### 19.2 Distribution

- MSIX installer;
- standalone installer if MSIX constraints block enterprise adoption;
- Winget package;
- GitHub releases;
- optional Microsoft Store after SmartScreen/reputation plan.

### 19.3 Landing and Adoption

Landing page must clearly communicate:

- resource-governed agent terminal;
- no Electron terminal hot path;
- no cloud account required;
- child-process resource dashboard;
- Cmd-K command generation;
- failed-command explanation;
- Claude/Codex support without bundling them;
- privacy and local-first defaults.

---

## 20. Acceptance Criteria

### 20.1 MVP-0 Acceptance

MVP-0 is acceptable only when:

1. PowerShell 7, Windows PowerShell, CMD, Git Bash, WSL, and SSH profiles launch.
2. Terminal input remains within latency budget on reference hardware.
3. High-volume output remains responsive and within RSS/VRAM budget.
4. Tabs and split panes work.
5. Command blocks work with confidence labels.
6. Cmd-K command generation produces preview-only suggestions.
7. Failed-command explainer works on last failed command block.
8. Smart history filters work.
9. Snippets work with parameter prompts.
10. Background job notifications work.
11. Clickable error patterns work for at least common Node, Python, Rust, .NET, and TypeScript patterns.
12. Claude Code and Codex CLI profiles launch if installed.
13. Agent transcripts are captured where output permits.
14. MCP manual config works with visible permissions and JobObject caps.
15. Resource dashboard displays BongTerm, shell, conhost, agent, MCP, and plugin/resource attribution.
16. Accessibility MVP passes Narrator smoke tests.
17. Telemetry is off by default and diagnostic export has redaction preview.
18. Installer is signed.
19. EDR-hostile techniques are absent.
20. Dogfood gate is met.

### 20.2 Dogfood Gate

MVP-0 cannot be called beta until:

- at least 10 internal or trusted developers use BongTerm as default terminal for a fixed dogfood cycle;
- each dogfood user runs at least one supported agent workflow;
- crash reports and resource snapshots are reviewed;
- average user-reported blocking defects are below release threshold;
- no unresolved P0/P1 terminal correctness bugs remain;
- resource budgets are met on the reference machine.

---

## 21. Critical Risks and Mitigations

| Risk | Mitigation |
|---|---|
| Terminal reuse component does not meet Windows performance/accessibility needs | Run reuse spike before product build; maintain interface boundary to swap component. |
| WGPU adds unacceptable latency or VRAM overhead | Keep renderer backend abstraction; allow Direct3D-specific backend if benchmarks require. |
| Agent CLIs change output/auth behavior | Built-in adapters limited to Claude/Codex; adapter contracts and fixtures required. |
| MCP processes exceed memory budgets | JobObject caps, visible resource ledger, admission control, no shared pool until measured. |
| Shell integration unreliable with prompt frameworks | Confidence labels, compatibility matrix, fallback mode, user-visible degraded state. |
| Worktree attribution not trustworthy | v1 only; confidence-tagged attribution; Git truth over SQLite; explicit edge tests. |
| SmartScreen hurts adoption | code signing, reputation warm-up, checksum instructions, enterprise deployment notes. |
| EDR flags process supervision | no injection/hooks; security whitepaper; EDR validation plan. |
| Trademark issue with BongTerm | trademark/domain/repo/package search before public launch. |
| Open-source profile maintenance burden | supported/community profile separation; tests required; stale profile policy. |

---

## 22. Definition of Done

A feature is done only when:

1. It meets functional acceptance criteria.
2. It has keyboard access.
3. It has empty/loading/error/degraded states.
4. It has tests at unit/integration/performance/security level as appropriate.
5. It passes resource-budget checks if it starts processes, renders output, reads terminal streams, or stores transcripts.
6. It respects SOLID boundaries.
7. It does not add hidden global dependencies.
8. It documents config behavior.
9. It handles crash/restart behavior.
10. It has accessibility validation.
11. It has security review if it touches processes, files, shell execution, secrets, MCP, agents, plugins, telemetry, or updates.
12. It updates user-facing documentation.

---

## 23. Final v7 Product Summary

BongTerm v7 is no longer an everything-at-once terminal super-app. It is a narrowed, buildable, resource-governed Windows terminal cockpit.

It keeps the strong principles from earlier PRDs:

- local-first operation;
- native terminal hot path;
- safe Windows abstractions;
- SOLID architecture;
- resource accounting;
- security and auditability.

It resolves the critical analysis by:

- removing custom terminal-core NIH from Phase 0;
- explicitly setting team assumptions;
- deferring the MCP pool;
- limiting first-party agents;
- narrowing OS support;
- using WinUI 3/Slint for non-hot UI;
- reducing accessibility acceptance to a realistic MVP bar;
- adding missing UX specifications;
- adding developer-retention features;
- adding CI/CD, update, crash, telemetry, licensing, SmartScreen, EDR, and trademark plans.

The product should now be pursued as:

> **BongTerm: a Windows-first terminal for developers who want AI-agent workflows with command clarity, process accountability, and strict local control.**
