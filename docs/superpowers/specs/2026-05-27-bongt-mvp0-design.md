# BongTerm MVP-0 Design Spec

| Field | Value |
|---|---|
| **Status** | Accepted |
| **Date** | 2026-05-27 |
| **Owner** | Solo developer (single owner) |
| **Supersedes** | None (first formal design spec; sits below PRD v7) |
| **PRD reference** | `docs/PRD/bongterm_prd_v7.md` + `docs/PRD/bongterm_v7_resolution_matrix.md` |
| **Scope** | Architecture, stack, runtime, error handling, testing, deliverable definition, risks for MVP-0 (`0.1.0-mvp0`) |
| **Next step** | Invoke `superpowers:writing-plans` to produce ordered implementation plan → `orca.md` |

This spec translates PRD v7 into concrete, buildable decisions for a solo developer. PRD v7 is the product contract. This document is the engineering contract. Where the two disagree, the PRD wins. Where the PRD is silent, this document is authoritative.

---

## 0. Locked Decisions Summary

The following decisions were made during brainstorming and are no longer open for relitigation in MVP-0 scope:

1. **Team profile**: solo dev, side project, no fixed deadline. Scope is reuse-heavy.
2. **Terminal-core path**: fork WezTerm. Vendor the WezTerm source tree as a Git submodule pinned to an immutable tag. Consume `wezterm-term`, `wezterm-mux`, `termwiz`, `portable-pty` from the vendored tree where available; `crates.io` only where the vendored tree does not provide a compatible version.
3. **Renderer**: **Approach B (ADR-001).** BongTerm owns the terminal renderer from MVP-0 using `wgpu` (Direct3D 12 backend on Windows) plus `glyphon` (which transitively pulls `cosmic-text` for shaping). `wezterm-gui` is not vendored as renderer. Approach C (use `wezterm-gui` as renderer) is fallback only, gated by an explicit ADR-001 update; never silent.
4. **UI framework**: **Iced 0.14.x** (MIT) for all non-hot-path UI. Slint rejected because its default GPLv3 conflicts with BongTerm's permissive open-core intent (PRD v7 §18). Reconsider only if BongTerm relicenses to GPLv3 or pays for a Slint commercial license.
5. **Toolchain**: Rust stable `1.95`, edition `2024`, `rustup` target `x86_64-pc-windows-msvc`. No nightly.
6. **AI backend** for Cmd-K and failed-command explainer: reuse the user's installed Claude Code CLI as a subprocess in non-interactive mode (`claude --print --output-format json …`). If Claude Code is not detected, the feature is disabled with a clear in-UI message. Direct Anthropic/OpenAI API and local-model backends are post-MVP-0 user-configurable swaps.
7. **MVP-0 built-in agent adapters**: Claude Code and Codex CLI only. All other agents (OpenCode, Gemini CLI, Aider, Copilot CLI, local CLIs) are community/imported profiles.
8. **MCP**: one process per enabled server in MVP-0. JobObject resource caps. No shared host pool — deferred to v1.1. No `npx -y` auto-install. Version-pinned commands with integrity hashes only.
9. **Reference hardware**: Ryzen 5 7535HS / 16 GB RAM / NVIDIA RTX 2050 4 GB VRAM (discrete) + Radeon 660M (iGPU fallback) / Win11 24H2. All performance budgets measured on this machine.
10. **Repo visibility**: private until MVP-0 ships, then public. Trademark + brand-perception review precedes public flip.
11. **Distribution**: signed MSIX. Portable ZIP is optional dev-channel only. Winget, Microsoft Store, EV cert deferred to post-`0.1.x`.

---

## 1. Architecture and Repository Layout

### 1.1 Top-level structure

```
bongterm/
├─ vendor/
│  └─ wezterm/                     # git submodule, MIT, pinned to immutable tag
├─ crates/
│  ├─ bongterm-app/                # binary entrypoint, Iced root window, composition
│  ├─ bongterm-ui/                 # Iced views, side panels, palette, dashboards
│  ├─ bongterm-render/             # MVP-0 renderer: wgpu + glyphon, dirty regions, atlas
│  ├─ bongterm-term/               # adapter over wezterm-term + termwiz; isolates upstream churn
│  ├─ bongterm-pty/                # ConPTY host; portable-pty + windows-rs
│  ├─ bongterm-mux/                # tabs, panes, layouts; sits over wezterm-mux primitives
│  ├─ bongterm-blocks/             # command-block boundary detection + confidence model
│  ├─ bongterm-agents/             # AgentAdapter trait + Claude Code + Codex CLI adapters
│  ├─ bongterm-mcp/                # MCP supervision (1 proc/server), logs, schema, JobObject caps
│  ├─ bongterm-ledger/             # resource ledger: facts only — CPU/RSS/VRAM/IO sampling
│  ├─ bongterm-process-control/    # JobObject enforcement, kill/restart, admission enforcement
│  ├─ bongterm-secrets-api/        # SecretRef, SecretStore trait, redaction-safe types
│  ├─ bongterm-vault-windows/      # DPAPI / Windows Credential Manager implementation
│  ├─ bongterm-security/           # PolicyEvaluator, redactor, dangerous-command detector
│  ├─ bongterm-storage-api/        # repository traits, DTOs, migration runner
│  ├─ bongterm-storage-sqlite/     # SQLite (WAL) implementation + sidecar chunk reader/writer
│  ├─ bongterm-settings/           # typed config snapshots, JSON5 schema, schemars-generated JSON schema
│  ├─ bongterm-devassist/          # MVP-0 developer-UX features: ai/, history/, snippets/, patterns/, jobs/
│  ├─ bongterm-diagnostics/        # crash dumps, perf snapshots, redacted export bundles
│  └─ bongterm-test-kit/           # workspace-internal test scaffolding + conformance suites (not published)
├─ benches/                        # criterion benches (parser, render, scrollback, ledger)
├─ tests/
│  ├─ integration/                 # cross-crate flows
│  ├─ fixtures/
│  │  ├─ osc/                      # OSC 7/8/52 + prompt-framework samples
│  │  ├─ shells/                   # PowerShell, CMD, Bash, WSL, Git Bash transcripts
│  │  ├─ agents/                   # synthetic Claude Code + Codex transcripts
│  │  ├─ secrets/                  # synthetic token / private key formats
│  │  ├─ prompt_injection/         # poisoned README, diff, log, MCP-result samples
│  │  ├─ fuzz_corpora/             # committed seed corpora for cargo-fuzz
│  │  └─ migrations/               # paired (input, expected output) for schema bumps
│  └─ accessibility/               # Narrator smoke harness
├─ docs/
│  ├─ PRD/                         # v5, v6, v7, resolution matrix (existing)
│  ├─ superpowers/specs/           # this document
│  ├─ adr/                         # architecture decision records
│  ├─ runbook/                     # release / signing / SmartScreen / EDR / crash response
│  ├─ dogfood/                     # private dogfood journal during Stage A
│  ├─ security/                    # threat model summary + security whitepaper
│  └─ troubleshooting/             # Optimus, WSL2 modes, prompt-framework collisions
├─ tools/
│  ├─ xtask/                       # cargo-xtask binary (multiple sub-binaries)
│  └─ spikes/                      # Wave 0 / Wave 1 spike scaffolds (deleted post-ADR)
├─ packaging/
│  └─ msix/                        # AppxManifest.xml, assets, signing scripts
├─ .github/
│  └─ workflows/                   # GitHub-hosted PR-blocking CI
├─ .gitmodules
├─ .gitignore
├─ Cargo.toml                      # workspace root
├─ Cargo.lock
├─ rust-toolchain.toml             # pin stable 1.95
├─ deny.toml                       # cargo-deny: licenses, sources, advisories, bans
├─ CLAUDE.md                       # session-start protocol (already present)
├─ orca.md                         # generated by writing-plans step (not yet present)
├─ README.md                       # private until MVP-0 public flip
├─ SECURITY.md
├─ THIRD_PARTY_NOTICES.md          # generated artifact, committed for review
└─ CHANGELOG.md
```

Total: **20 crates** (19 product crates + 1 workspace-internal test kit).

### 1.2 Crate ownership matrix

Each crate owns one reason to change. The matrix below is binding; the `cargo xtask check-deps` enforcer rejects PRs that violate it.

| Crate | Owns | Must not own |
|---|---|---|
| `bongterm-app` | Composition root, runtime startup, top-level wiring | Domain logic, direct platform calls outside `bongterm-pty`/`bongterm-vault-windows` |
| `bongterm-ui` | Iced views, view models, presentation state, gestures | Direct PTY spawn, direct secret read, direct Git mutation, direct DB write outside repository traits |
| `bongterm-render` | wgpu device, glyph atlas, dirty regions, frame pacing, swap chain | Command semantics, agent state, MCP state, policy, **any direct dependency on `wezterm-term` or `termwiz` types** |
| `bongterm-term` | Adapter over `wezterm-term` + `termwiz`; emits typed terminal events; **owns the `SurfaceSnapshot`, `CellRun`, `CursorState`, `DirtyRegion` types that `bongterm-render` consumes** | Renderer state, agent decisions, settings UI |
| `bongterm-pty` | ConPTY session lifecycle, child process spawning, ring/slab buffers | Parsing, policy, settings UI |
| `bongterm-mux` | Pane/tab/layout model | Renderer internals, agent decisions |
| `bongterm-blocks` | Command-block boundary detection, confidence labels, OSC observation | Renderer mutation, agent execution |
| `bongterm-agents` | `AgentAdapter` trait, Claude Code adapter, Codex CLI adapter, transcript ingestion, lifecycle control | Renderer state, MCP process internals, secret vault implementation |
| `bongterm-mcp` | MCP registry, transport adapters, process supervision, tool routing, audit | Agent UI, renderer, Git |
| `bongterm-ledger` | Sampling + recording: CPU, RSS, VRAM, IO, handles, network endpoints, tokens, cost | Policy decisions, enforcement, UI |
| `bongterm-process-control` | JobObject creation, attachment, caps, kill, restart, admission enforcement | Reporting (delegates to ledger), policy decisions |
| `bongterm-secrets-api` | `SecretRef`, `SecretStore` trait, redaction-safe types | Concrete storage implementation |
| `bongterm-vault-windows` | DPAPI / Windows Credential Manager implementation of `SecretStore` | Policy decisions, UI |
| `bongterm-security` | `PolicyEvaluator`, redactor corpus, dangerous-command pattern matcher, workspace trust | Renderer, parser, vault implementation |
| `bongterm-storage-api` | Repository traits, DTOs, migration runner contract | Concrete database driver |
| `bongterm-storage-sqlite` | SQLite (`rusqlite` bundled, WAL) implementation, sidecar chunk reader/writer | Business policy |
| `bongterm-settings` | Typed `Settings` snapshots (via `arc-swap`), JSON5 parser/writer, `schemars`-generated `settings.schema.json`, migrations | Global mutable config service |
| `bongterm-devassist` | Cmd-K NL→command, failed-command explainer, smart history, snippets, clickable patterns, background jobs | Hot-path code, direct PTY/agent/MCP internals |
| `bongterm-diagnostics` | Crash dumps (minidump), perf snapshots, redacted export bundles, telemetry-consent flow | Live observability decisions |
| `bongterm-test-kit` | Mocks, conformance suites, deterministic clock, seedable RNG, fixtures helpers | Production code paths |

### 1.3 Dependency rules (enforced by `cargo xtask check-deps`)

The allowed-edges manifest sits at `tools/xtask/allowed-deps.toml`. The pre-commit hook + CI block PRs that introduce edges outside the manifest.

Required invariants:

- `bongterm-term` and `bongterm-render` must not depend on `bongterm-agents`, `bongterm-mcp`, `bongterm-devassist` (including `bongterm-devassist::ai` or any other devassist submodule), settings UI, or any analytics or cloud module.
- `bongterm-render` must not import `wezterm-term`, `termwiz`, or any vendored WezTerm crate directly. It reads cell data only through BongTerm-owned types exported by `bongterm-term` (`SurfaceSnapshot`, `CellRun`, `CursorState`, `DirtyRegion`). This isolation is what makes R1 mitigation hold: a WezTerm API break is absorbed inside `bongterm-term` and never reaches the renderer.
- `bongterm-ui` must not directly call PTY spawn, vault read, Git mutation, or DB write — all such calls go through narrow traits exposed by the relevant infrastructure crate.
- Only `bongterm-app` (composition root) and explicit repository crates may depend on `bongterm-storage-sqlite`. All feature crates depend on `bongterm-storage-api` only.
- All subsystems handling secrets depend on `bongterm-secrets-api` only. `bongterm-vault-windows` is wired in `bongterm-app` and nowhere else.
- `bongterm-process-control` enforces; `bongterm-ledger` measures; `bongterm-security::PolicyEvaluator` decides; `bongterm-ui` explains. None of these crates may take on more than one role.
- The terminal hot path — `bongterm-pty` → `bongterm-term` → `bongterm-blocks` (side channel) → `bongterm-render` — has no upward dependency on agents/MCP/devassist/diagnostics/UI mutation.

### 1.4 Submodule policy

The `vendor/wezterm` submodule is the canonical source of WezTerm libraries. Policy:

1. The submodule must always point to a tag or immutable commit, never `main`.
2. Local patches live as explicit patch files under `vendor/wezterm.patches/` or as fork commits in a BongTerm-owned WezTerm fork repository. Every patch carries an ADR link explaining why.
3. `cargo xtask upstream-sync` produces a Markdown changelog of upstream delta between the currently pinned tag and a target tag. An ADR is required to bump the pin.
4. CI fails when the submodule working tree is dirty.
5. CI fails when the packaged MSIX is missing `THIRD_PARTY_NOTICES.md` or when the SBOM does not reference the vendored WezTerm version.

A "break" against `wezterm-term` is defined as a change that either requires modifications outside `bongterm-term` or costs more than one working day to absorb. The Decision Trigger table in §7 lists the action taken when breaks accumulate.

---

## 2. Stack, Toolchain, Build, Lint, Test

### 2.1 Toolchain

`rust-toolchain.toml`:

```toml
[toolchain]
channel = "1.95"
components = ["rustfmt", "clippy", "llvm-tools-preview"]
targets = ["x86_64-pc-windows-msvc"]
```

Workspace `Cargo.toml`:

```toml
[workspace]
resolver = "2"
members = ["crates/*", "tools/xtask"]

[workspace.package]
edition = "2024"
rust-version = "1.95"
license = "Apache-2.0"
repository = "https://github.com/<owner>/bongterm"
```

**Stable-only product policy.** Product, dev, PR, and release builds use stable Rust only. No `RUSTC_BOOTSTRAP`.

**Fuzzing exception.** `cargo-fuzz` requires a nightly toolchain for libFuzzer instrumentation. A separate pinned nightly toolchain (e.g. `nightly-2026-04-01`) is installed only for the fuzz jobs in `tools/xtask/fuzz/` and the `cargo fuzz run …` invocations in nightly CI. Pinned nightly:
- never produces release artifacts;
- never enters the MSIX package;
- never affects PR-blocking correctness CI;
- is documented in `docs/runbook/fuzzing.md` with the pinned date and bump policy.

The nightly fuzz toolchain pin is bumped only via an ADR.

### 2.2 Major dependencies (workspace-level)

| Crate | Version | Source | Purpose |
|---|---|---|---|
| `termwiz` | matches vendored if present, else `0.23.x` | vendor path preferred, else crates.io | VT/ANSI/OSC parser + Surface + ConPTY caps |
| `wezterm-term` | vendored | `path = "vendor/wezterm/term"` | terminal state machine |
| `wezterm-mux` | vendored | `path = "vendor/wezterm/mux"` | pane/tab primitives |
| `portable-pty` | vendored if present | path or crates.io | ConPTY abstraction |
| `windows` | `0.58.x` | crates.io (Microsoft) | Win32, DPAPI, Credential Manager, JobObject, DXGI |
| `wgpu` | `0.20.x` | crates.io | renderer; D3D12 backend on Windows |
| `glyphon` | `0.6.x` | crates.io | text shaping + atlas over wgpu (uses `cosmic-text` transitively) |
| `iced` | `0.14.x` | crates.io | UI framework, MIT, wgpu + tiny-skia backends |
| `tokio` | `1.x` | crates.io | async runtime |
| `tracing` + `tracing-subscriber` | latest pinned | crates.io | structured logging |
| `serde` + `serde_json` + `json5` | latest pinned | crates.io | settings serialization |
| `rusqlite` (bundled) | `0.32.x` | crates.io | SQLite WAL persistence |
| `schemars` | `0.8.x` | crates.io | settings JSON schema generation |
| `thiserror` | `2.x` | crates.io | library errors |
| `anyhow` | `1.x` | crates.io | binary/app boundary errors only |
| `camino` | `1.x` | crates.io | UTF-8 paths |
| `uuid` | `1.x` | crates.io | IDs for blocks, panes, sessions |
| `time` | `0.3.x` | crates.io | timestamps |
| `parking_lot` | `0.12.x` | crates.io | hot-path locks where proven beneficial |
| `arc-swap` | `1.x` | crates.io | typed settings snapshots without lock churn |
| `dashmap` | `6.x` | crates.io | non-hot-path concurrent maps |
| `criterion` | `0.5.x` | dev-dep | benchmarks |
| `proptest` + `arbitrary` | latest | dev-dep | property tests |
| `cargo-fuzz` | external binary | — | parser fuzzing |
| `insta` | `1.x` | dev-dep | snapshot tests |
| `blake3` | latest pinned | crates.io | sidecar chunk checksums |
| `minidump-writer` | latest pinned | crates.io | crash dumps |
| **Excluded MVP-0** | — | — | — |
| `notify` | — | excluded | re-enters with worktrees in v1 |
| `wasmtime` | — | excluded | re-enters with plugins post-MVP-0 |

The lockfile is committed. Version-bump PRs run the full deny + SBOM + license-notice regeneration.

### 2.3 Release profile

```toml
[profile.release]
lto = "thin"
codegen-units = 1
strip = "symbols"
# Do NOT set panic = "abort" for the main BongTerm release build.
# Section 4 requires unwind semantics so per-pane / renderer-thread / UI-thread
# / app-wide panic boundaries can use `catch_unwind` and retained state to
# isolate, recover, and present a recovery screen.

[profile.dev]
debug = 1
```

BongTerm MVP-0 release builds shall use **unwind** panic semantics so the pane, renderer-thread, UI-thread, and app-wide panic boundaries defined in §4.2 can function. Crash diagnostics are captured by `bongterm-diagnostics` via `minidump-writer` hooked from the panic hook (which fires during unwind) plus a structured-log writer. `panic = "abort"` may be reconsidered only for isolated helper binaries (`tools/xtask`, fuzz targets) or after the crash-isolation model is rewritten to process-level recovery.

### 2.4 Build commands

```powershell
git clone --recurse-submodules <repo>
git submodule update --init --recursive

cargo build
cargo build --release --target x86_64-pc-windows-msvc
cargo run --release --bin bongterm
```

### 2.5 Lint and format

```powershell
cargo fmt --all -- --check
cargo clippy --all-targets --all-features --workspace -- -D warnings
cargo deny check
```

- `rustfmt.toml` at workspace root, opinionated, no overrides per crate.
- `clippy.toml` warn-level lints raised to deny in CI.
- `deny.toml` rejects GPL/AGPL in the core (Apache/MIT/BSD/ISC/Unicode/CC0 only), bans `git`-source deps for non-vendor crates, requires explicit version pinning for `windows`, `wgpu`, `glyphon`, and `iced`.

### 2.6 Test commands

```powershell
cargo test --workspace
cargo test --workspace --test '*'                # integration tests
cargo bench --workspace
cargo fuzz run vt_parser -- -max_total_time=300
cargo insta test --review
cargo test -p bongterm-blocks block_boundary_powershell_high   # single test
```

### 2.7 `cargo-xtask` commands

`tools/xtask` is a Rust binary with multiple sub-binaries (no `make`/`just`/`cargo-make` dependency):

| Command | Purpose |
|---|---|
| `cargo xtask check-deps` | Verifies workspace dependency graph against `tools/xtask/allowed-deps.toml` |
| `cargo xtask check-licenses` | Ensures every packaged artifact ships `THIRD_PARTY_NOTICES.md` |
| `cargo xtask upstream-sync` | Generates Markdown changelog of `vendor/wezterm` delta against pinned tag |
| `cargo xtask sbom` | Generates CycloneDX SBOM from `Cargo.lock` + vendored WezTerm + system deps |
| `cargo xtask bench-report` | Runs criterion + produces release-notes-ready report; `--gate` flag fails on absolute-budget violation |
| `cargo xtask secret-leak-corpus` | Runs known synthetic token corpus through redaction pipeline |
| `cargo xtask prompt-injection-corpus` | Runs poisoned content corpus through agent observer + policy |
| `cargo xtask cleanup-chunks` | Orphan sidecar chunk cleanup |
| `cargo xtask package-msix` | Produces signed MSIX artifact |
| `cargo xtask doctor` | Diagnoses local environment: Windows version, VS Build Tools, Rust toolchain, submodule state, code-signing cert, Windows SDK, MSIX tooling, Defender status, GPU adapter detection, WSL availability |

### 2.8 CI matrix

CI is split across three runner classes; each has a different trust profile.

| CI class | Runner | Jobs |
|---|---|---|
| **PR-blocking correctness** | GitHub-hosted Windows runner | `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test --workspace`, `cargo deny check`, `cargo xtask check-deps`, `cargo xtask check-licenses`, gitleaks secret scan, submodule cleanliness check, `cargo build --release` smoke, MSIX manifest validate, `cargo xtask sbom` validity |
| **Nightly hardening / perf / shell / EDR / accessibility** | Self-hosted runner = the dev machine (RTX 2050 + 7535HS), scheduled idle window | `cargo xtask bench-report --gate`, `cargo fuzz run vt_parser` 30-min budget, pane scaling 1/4/8/12, scrollback 100k/500k/1M, shell smoke (PowerShell, Windows PowerShell, CMD, Git Bash, WSL, SSH), TUI smoke (vim/less/htop equivalent), GPU device-loss recovery, accessibility (Narrator scripted), Defender real-time-on smoke, `cargo xtask secret-leak-corpus`, `cargo xtask prompt-injection-corpus` |
| **Release signing and packaging** | Dedicated clean Windows release machine (NOT the daily dev machine) | MSIX signing, MSIX install/upgrade/uninstall smoke on clean VM, SmartScreen probe |

Rule: self-hosted benchmark failures block performance claims but not all functional PRs. Performance regressions are tracked as their own gate.

### 2.9 Reference hardware budgets

Budgets are measured on the RTX 2050 + 7535HS reference machine. The iGPU column applies when the laptop is unplugged or NVIDIA Optimus mux routes rendering to the Radeon 660M iGPU.

| Axis | RTX 2050 target | iGPU fallback target |
|---|---:|---:|
| Warm startup to first prompt | ≤ 300 ms | same |
| Startup with shell integration | ≤ 800 ms | same |
| Keystroke-to-glyph p99 (60 Hz) | ≤ 16 ms | same |
| Keystroke-to-glyph p99 (120 Hz) | ≤ 8 ms stretch | not required |
| Idle CPU, 1 pane, 60 s window | ≤ 0.1 % | same or slightly relaxed |
| BongTerm core RSS, 1 pane | ≤ 120 MB | same |
| Additional pane RSS overhead | ≤ 25 MB | same |
| Additional pane VRAM overhead | ≤ 8 MB | not budgeted |
| Total VRAM ceiling | ≤ 256 MB | ≤ 128 MB |
| MCP server RSS cap (default) | ≤ 60 MB | same |
| Plugin RSS cap (default, **reserved post-MVP**) | ≤ 40 MB | same |
| Plugin/agent 12-pane stress | functional + perf | functional only, perf relaxed |
| 10 MB stream throughput | ≥ 50 MB/s | ≥ 30 MB/s |
| 4-pane idle battery drain | ≤ 2 %/hour on battery | same |

Software-render fallback is documented and tested but not subject to a perf budget.

**Plugin runtime scope**: the plugin RSS cap above is a reserved post-MVP budget. MVP-0 exposes **no user-installable plugin runtime** (no WASM Tier 1 plugins, no Tier 2 out-of-process native adapters). The resource dashboard criterion (§6.1 #17) shows "plugins: 0 / hidden" in MVP-0 unless a placeholder plugin process exists for testing.

---

## 3. Runtime Model

BongTerm has three concurrent runtime domains, each with explicit ownership. No global `AppContext`.

### 3.1 Terminal hot path (per pane)

The hot path runs `tokio` multi-thread plus a dedicated renderer thread driving wgpu. The UI thread (Iced) is separate.

```
[ConPTY child stdout/stderr bytes]
        ↓
[bongterm-pty: read into reusable ring/slab buffer (8 KiB slabs, object pool)]
        ↓
[bongterm-term: termwiz parser consumes byte slice → emits TermEvent stream]
        ↓
[bongterm-term: wezterm-term state machine applies event → mutates Surface cells]
        ↓
[bongterm-blocks: observes OSC + shell integration markers → BlockEvent stream]
        ↓
[bongterm-term: emits `SurfaceSnapshot` + `DirtyRegion` (BongT-owned types, no WezTerm leakage)]
        ↓ (side channel)            ↓ (hot)
[transcript / index / agent obs]   [bongterm-render: consumes SurfaceSnapshot]
                                          ↓
                                    [bongterm-render: dirty-region collector]
                                          ↓
                                    [wgpu draw queue]
                                          ↓
                                    [glyphon shaping + atlas lookup]
                                          ↓
                                    [swap chain present]
```

Ownership: each pane owns one PTY reader task, one parser/state worker, one Surface. The renderer shares one wgpu device and one glyph atlas across all panes whose `(font, dpi, weight, ligature, color_mode)` tuple matches.

Backpressure: every cross-thread channel is bounded. If the parser falls behind, the PTY reader blocks. If the renderer falls behind, the parser still mutates the Surface and the renderer coalesces dirty regions across missed frames. No unbounded queue exists anywhere on the hot path.

Zero-allocation discipline:

- Ring buffer slabs are reused via a `crossbeam`-based object pool.
- `termwiz::Parser::parse(&mut self, &[u8], callback)` consumes byte slices without materializing `String`.
- Block/OSC events use `SmallVec<[u8; 64]>` for small payloads.
- The renderer reuses vertex/index buffers across frames; the glyph atlas evicts LRU when over budget.
- Allocations-per-MB and copies-per-MB are tracked via `tracing` perf events and asserted in criterion benches.

### 3.2 Queue classification

Side-channel queues are classified to make backpressure behavior explicit:

| Class | Examples | Policy |
|---|---|---|
| **Transcript-lossless** | persistent transcript chunks | May pause persistence under disk pressure; the terminal display continues; an explicit warning is surfaced. Spilling to disk is allowed but bounded. |
| **Audit-critical** | secret access logs, approval decisions, destructive/external-write MCP audit | Fail closed. No new secret resolutions or high-risk tool calls when the audit trail cannot be recorded. |
| **Lossy-observable** | live dashboard samples, resource graph updates, agent status badges | May drop or coalesce events. The UI shows a stale/degraded badge when this happens. |
| **Recomputable** | smart-history index, frecency index, search index | May drop and rebuild later. The palette shows an "Indexing…" status. |

Universal drop-oldest is not used.

### 3.3 Agent runtime

```
[User triggers "Launch Claude Code in pane X"]
        ↓
[bongterm-agents::ClaudeCodeAdapter::discover() → resolve binary, check version, check auth]
        ↓
[bongterm-security::PolicyEvaluator → workspace trust, secret scope, MCP allowlist]
        ↓
[bongterm-vault-windows: resolve ${secret:...} → plaintext (in-memory, scoped)]
        ↓
[bongterm-agents::ClaudeCodeAdapter::build_command(launch_spec) → ProcessSpec]
        ↓
[bongterm-pty::spawn(ProcessSpec) under JobObject from bongterm-process-control]
        ↓
[Output stream forks: terminal grid (visible) + transcript (persistent) + classifier (events)]
        ↓
[Agent events: tool_call, file_read, file_write, approval_request, exit] → UI sidebar
        ↓
[On exit: summarize_exit(exit, transcript) → ledger close → transcript flush]
```

The adapter contract is stateful to handle output spanning chunks:

```rust
pub trait AgentAdapter: Send + Sync {
    fn id(&self) -> AgentId;
    fn display_name(&self) -> &'static str;
    fn discover(&self) -> DiscoveryResult;
    fn auth_state(&self) -> AuthState;
    fn capabilities(&self) -> AgentCapabilities;
    fn build_command(&self, launch: AgentLaunchSpec) -> ProcessSpec;
    fn create_classifier(&self, session: AgentSessionId) -> Box<dyn AgentOutputClassifier>;
    fn summarize_exit(&self, exit: ExitState, transcript: TranscriptRef) -> AgentExitSummary;
}

pub trait AgentOutputClassifier: Send {
    fn ingest(&mut self, chunk: OutputChunk) -> Vec<AgentEvent>;
    fn flush(&mut self) -> Vec<AgentEvent>;
}

pub struct AgentCapabilities {
    pub launch_modes: Vec<LaunchMode>,           // Interactive, Headless, OneShot
    pub control_channel: ControlChannel,         // None, StdinBestEffort, CliCommand, SdkApi
    pub safe_interrupt: CapabilityLevel,         // Unavailable, BestEffort, Reliable
    pub transcript_reliability: Reliability,     // Low, Medium, High, Structured
    pub file_event_detection: DetectionMode,     // None, OutputHeuristic, FsWatcher, ExplicitToolEvents
    pub mcp_support: McpSupport,                 // None, ConfigFile, EnvInjection, Native
    pub structured_output: bool,
    pub supports_json_output: bool,
    pub requires_tty: bool,
}
```

Capabilities a CLI does not support are labeled unavailable in the UI, never simulated.

MVP-0 adapters: `ClaudeCodeAdapter`, `CodexCliAdapter`. Both are detect-and-launch only — no bundling of the underlying CLI binary (PRD v7 §18.1).

The Cmd-K NL→command and failed-command explainer features live in `bongterm-devassist/ai/`. They wrap `ClaudeCodeAdapter` in non-interactive mode (`claude --print --output-format json --prompt "<context>"`). When Claude Code is not present, the feature is disabled with a clear in-UI message and a link to install instructions; nothing is silently substituted.

### 3.4 MCP runtime

```
[MCP server config imported from JSON]
        ↓
[bongterm-security: validate permission summary, integrity hash, source — no `npx -y` allowed]
        ↓
[bongterm-mcp::Supervisor::spawn(config) under JobObject (RSS/CPU caps)]
        ↓
[stdio transport: one process per server; BongTerm proxies tool calls]
        ↓
[On agent launch: ContextOptimizer prunes tool schema → only allowed tools exposed via temporary scoped MCP config (when adapter supports MCP config files)]
        ↓
[Tool calls audited in bongterm-storage-sqlite (audit log) with reference (never value)]
        ↓
[Health check every 30 s; RSS sample every 1–2 s; idle shutdown only when no active agent attached]
```

MVP-0 rules:

- One process per enabled server per workspace. No shared host pool.
- JobObject caps via `bongterm-process-control`: default 60 MB RSS, configurable CPU, configurable child-process count.
- Server crash → restart with exponential backoff (1 s, 5 s, 30 s). Three failures within 5 minutes → marked Unhealthy; will not auto-restart until the user re-enables.
- Idle shutdown applies only to servers not currently attached to an active agent session. Stdio MCP servers required by an active agent remain alive until the session ends or the user explicitly disables them.
- All MCP commands are version-pinned and source-attributed. Package-based MCP servers require explicit install/review before first launch. BongTerm never auto-installs remote code at runtime.
- For agents that expose MCP config files, BongTerm generates a temporary scoped MCP config exposing only permitted tools. For agents that do not support BongTerm-mediated MCP configuration, BongTerm labels MCP governance as unavailable for that adapter.

### 3.5 Resource governance

Responsibilities are explicitly split:

| Module | Role |
|---|---|
| `bongterm-ledger` | Measures and reports facts. CPU, RSS, IO, VRAM, handles, network endpoints, tokens, cost, runtime duration, policy violations. |
| `bongterm-process-control::AdmissionController` | Projects resource impact of a proposed launch. Lives inside `bongterm-process-control` so it has direct access to enforcement primitives. |
| `bongterm-security::PolicyEvaluator` | Returns `Decision::Allow | RequireApproval(reason) | Deny(reason) | Advisory(warn)` based on projection + policy. |
| `bongterm-process-control` | Enforces JobObject caps, kills, restarts. |
| `bongterm-ui` | Explains the situation and surfaces user choice. |

Sampling cadence:

| Metric | Frequency |
|---|---|
| Per-process CPU, RSS, IO, handles | 1 Hz |
| MCP health check | 30 s |
| MCP RSS cap re-check | 1–2 s |
| UI dashboard refresh | 1 Hz |
| Historical rollup | 5 min and hourly |

Per-pane VRAM is labeled in UI as an estimate: process-wide VRAM is measured via DXGI `IDXGIAdapter::QueryVideoMemoryInfo`; per-pane share is approximated by atlas allocation fraction. The dashboard labels these as `process-wide measured` and `pane share estimated`.

Admission control flow before any new pane, background job, agent, MCP server, or plugin: `bongterm-ledger` reports current totals → `AdmissionController` projects the proposed delta → `PolicyEvaluator` decides → if `RequireApproval`, the UI presents a modal with options (launch anyway, launch with lower limits, disable selected MCP/plugin, defer, open diagnostics).

### 3.6 Settings flow

`bongterm-settings` owns schema-validated JSON5 files: `settings.json5`, `keybindings.json5`, `profiles.json5`. Files live in `%LOCALAPPDATA%\BongTerm\`.

`schemars` generates `settings.schema.json` at build time; the generated schema is published in the repo so editors can autocomplete. `arc-swap` holds the current `Settings` snapshot; readers `load_full()` for cheap clones; writers atomically replace via `store`.

Reload behavior in MVP-0:

- Manual reload only. The command palette exposes "Reload Settings". Auto-watch is deferred to v1 (re-enters with `notify` when worktrees land).
- Schema validation runs before apply. On parse failure, the last valid snapshot remains active; the validation error is surfaced with file, line, and column where available.
- Atomic apply. The previous file is backed up to `*.bak.<utc>` before any rewrite. Unknown fields are preserved for forward compatibility.

`bongterm-settings` exposes typed snapshot accessors only; it never becomes a global mutable config service.

### 3.7 Security and approval flow

Every action touching shell execution, file write, external write, or secret access flows through `bongterm-security::PolicyEvaluator`:

```rust
pub enum Decision {
    Allow,
    RequireApproval { reason: String, enforcement: EnforcementLevel },
    Deny { reason: String, enforcement: EnforcementLevel },
    Advisory { warn: String },
}
```

`Advisory` is never labeled "blocked" in the UI (PRD v7 §17.5 enforcement model). The UI surfaces the exact verb permitted by the enforcement level.

Redaction (`bongterm-security::Redactor`) applies to:

- persisted transcripts
- searchable indexes
- exports
- AI context bundles
- diagnostic bundles

Redaction does **not** apply to:

- raw terminal display — the live scrollback preserves process output verbatim for terminal correctness. Silent mutation of visible output would destroy trust in the terminal.

If raw persisted chunks are enabled (opt-in "raw history" mode), they are stored encrypted with a DPAPI-wrapped key, scoped per workspace. `command_blocks.command_text` is redacted by default; raw command text is persisted only when raw history is opted in.

The redactor matches a corpus of token and key formats: AWS access keys, GitHub PATs, OpenAI/Anthropic keys, JWTs, SSH private key headers, generic high-entropy strings. Detection is documented as best-effort; it does not claim completeness.

### 3.8 Persistence layout

One `bongterm.db` per user data directory: `%LOCALAPPDATA%\BongTerm\bongterm.db`. SQLite with `journal_mode=WAL`, `synchronous=NORMAL`, `foreign_keys=ON`.

Initial migration `0001_init.sql` creates:

- `panes`, `sessions`, `workspaces`
- `command_blocks` (id, pane_id, command_text [redacted], exit_code, started_at, ended_at, cwd, shell, branch, worktree_id, confidence, output_chunk_offset)
- `output_chunks` (refs to `chunks/<workspace>/<date>.bin`)
- `transcripts` (per agent run)
- `agent_runs`, `mcp_calls`, `tool_audit`
- `ledger_samples_5min`, `ledger_samples_hourly`
- `secret_audit` (reference + consumer + timestamp + exposure_class only — never value)
- `_schema_version`

Sidecar chunk files live at `%LOCALAPPDATA%\BongTerm\chunks\<workspace>\<date>.bin`. Chunk policy:

- Monotonic chunk IDs.
- Per-chunk `blake3` checksum.
- Compression off by default in MVP-0 (re-evaluate post-MVP-0).
- Retention by both age and total size, configurable per workspace and globally.
- Redaction state metadata in the chunk header.
- Crash recovery scan on startup verifies checksums; torn chunks are trimmed with a user-visible warning.
- `cargo xtask cleanup-chunks` removes orphans.

Source-of-truth split (PRD v7 §38.4):

- **Git is truth for repo/worktree/PR state.** The corresponding SQLite cache is reconstructable from Git if corrupt. (Relevant only when worktrees land in v1.)
- **Transcripts, command history, and resource ledger are local source-of-truth.** They cannot be reconstructed from Git and must be crash-safe. A corrupt primary store is recovered from append-only chunks where possible and is never silently fabricated.

---

## 4. Error Handling, Failure Model, Recovery

### 4.1 Error taxonomy

Four classes; every error carries `DataLossRisk` and `EnforcementLevel` metadata.

```rust
pub enum ErrorClass {
    Recoverable,
    UserActionable,
    FatalIsolated,
    FatalProcess,
}

pub enum DataLossRisk {
    None,
    Possible,
    ConfirmedPartialLoss,
    Unknown,
}

pub enum EnforcementLevel {
    Advisory,
    Enforced,
    Degraded,
    Unavailable,
}
```

UI copy may not imply data preservation unless the relevant flush/checksum/recovery actually succeeded. UI copy may not say "blocked" unless the configured mechanism can technically prevent the action.

### 4.2 Crash isolation boundaries

| Boundary | Behavior |
|---|---|
| **Per-pane** | Panic in `bongterm-term` or `bongterm-blocks` for one pane → that pane shows a red banner "Pane crashed, terminal state preserved, click to restart". Scrollback and transcript are flushed before pane teardown. Other panes are unaffected. |
| **Per-agent** | Adapter panic or agent subprocess crash → the agent sidebar shows "Crashed". Transcript and the last 60 s of ledger samples are preserved. Restart button surfaced. MCP children attached to that agent are killed via the JobObject process tree. |
| **Per-MCP-server** | Server crash → restart with exponential backoff (1 s, 5 s, 30 s). Three failures within five minutes → marked Unhealthy; auto-restart disabled until user action. |
| **Renderer device loss** | DXGI device-removed/reset/TDR/driver-update/hybrid-GPU switch/RDP transition → release device resources, recreate swap chain + glyph atlas + pipeline state, repaint from retained Surface state. No scrollback loss. Three recoveries in 60 seconds → software-rendering fallback with a banner. |
| **Renderer-thread panic** | Render loop or glyph atlas panic is a distinct fatal-isolated class. Freeze rendering, preserve panes/surfaces/transcripts, show a process-level recovery overlay if the UI thread remains alive, attempt one renderer restart from retained Surfaces. Two renderer-thread panics within 60 s escalate to Fatal-process with a crash dump. |
| **UI-thread panic** | If terminal runtime and child processes are alive, attempt graceful shutdown of visible windows, preserve transcripts/ledger, write minidump, restart into the recovery screen. If shutdown cannot be coordinated, escalate to Fatal-process. |
| **App-wide panic** | Caught at the tokio runtime root and the UI thread `catch_unwind` boundary. `minidump-writer` writes `%LOCALAPPDATA%\BongTerm\crashes\<utc>.dmp` plus a structured log. Restart shows the recovery screen. |
| **Runtime watchdog** | A heartbeat task tracks hot-path tasks, renderer heartbeat, and ledger sampler. Stalled subsystems are marked Degraded; diagnostics captured; user offered a restart. The watchdog never kills child shells without user confirmation. |

### 4.3 Recovery screen

On startup after a non-clean exit:

- Banner: "BongTerm restarted after a crash."
- List of affected sessions, panes, and agents with last-known state.
- Per-item actions: **Restore**, **Discard**, **Export diagnostics**.
- "Suspected culprit" line when attributable (e.g. "Agent claude-code in pane 3 crashed last").
- "Start in Safe Mode" option — disables agents, MCP auto-start, plugins, experimental renderer flags; loads default theme; profiles/settings/history read-only.
- Diagnostics are local-only. "Share with author" opens a modal that shows the redaction preview before any send.

Restore semantics: live process reattach when the underlying process is still alive; otherwise transcript-only restoration labeled "process ended".

### 4.4 Storage durability

- SQLite WAL mode plus `PRAGMA integrity_check` at startup and nightly.
- On corrupt-primary detection: attempt recovery from sidecar chunks (rebuild `command_blocks` rows from chunk headers + checksums). Never silently fabricate.
- Sidecar chunk torn-write detection via per-chunk `blake3` checksum at startup scan. Torn tails are trimmed, marked in the user-visible log, never silently dropped.

### 4.5 Backpressure failures

When transcript-lossless queues exceed disk quota:

- Pause transcript persistence for that queue.
- Surface explicit modal: "Transcript persistence paused: disk quota reached. Adjust retention or free space."
- The live terminal display, agent execution, and ledger collection continue.
- Persistence resumes automatically when free space returns; the resume event is audited.

When audit-critical queues cannot persist:

- Fail closed. No new secret resolutions, approval decisions, or destructive/external-write MCP tool calls are permitted until the queue can persist again.
- The UI explains exactly which class of action is paused and why.

Lossy-observable queues that drop coalesce silently into "Data lagging" badges that auto-clear when caught up. Recomputable queues that drop schedule an index rebuild visible in the palette.

### 4.6 Migration failure handling

| Failure | Behavior |
|---|---|
| Settings/profile migration failure | Roll back to backup. Start with last-known-good config if available. If unavailable, start Safe Mode with defaults. |
| Security-policy migration failure | Hard stop or Safe Mode with all risky actions denied. |
| Keybindings/theme migration failure | Start with defaults plus a warning. |
| Transcript/ledger migration failure | Start terminal in read-only/degraded persistence mode. |
| SQLite primary DB corruption | Attempt recovery from chunks. If recovery fails, allow terminal-only mode but disable history, agents, MCP, secret access, and audit-dependent features. |
| Unknown future schema version | Do not migrate. Start read-only or Safe Mode. Never overwrite. |

### 4.7 Process control failures

- JobObject creation fails → MCP or agent launch denied with reason "Job Object unavailable" (typically missing session privilege). The user may launch without a cap if they confirm explicitly — the UI downgrades the enforcement label visibly.
- Process-tree kill fails (kernel hold) → surface to the user with "Process may be unkillable; try Task Manager" and log.
- Cap exceeded mid-run (RSS over JobObject limit) → JobObject kills the child. The ledger reports it as `KilledOverBudget`, not as a crash. The last RSS sample before kill is preserved.

### 4.8 Explicit anti-patterns

- Never show "blocked", "sandboxed", or "prevented" copy unless the actual enforcement layer can prevent the action.
- Never retry secret resolution silently. Missing secrets fail closed and surface; never launch with an empty environment variable.
- Never swallow a renderer device-loss without logging.
- Never silently fabricate transcript or ledger data after corruption recovery.
- Never auto-update config across a schema version newer than ours — back off, surface, never destructively overwrite.

---

## 5. Testing Strategy

Tests are organized into four operational buckets. The 14 underlying layers (unit, contract, property-fast, property-deep, fuzz, snapshot, integration, benchmark, shell smoke, TUI smoke, GPU/device-loss, accessibility, secret/prompt-injection corpora, install/uninstall smoke) live inside these buckets:

| Bucket | Layers | Runner |
|---|---|---|
| **PR-blocking correctness** | unit, contract, property-fast, snapshot, integration, secret-leak-fast, prompt-injection-fast | GitHub-hosted Windows |
| **Nightly hardening** | fuzz, property-deep, shell smoke, TUI smoke, secret corpus full, prompt-injection corpus full | Self-hosted |
| **Self-hosted performance and device** | benchmark gates, GPU/device-loss, pane scaling, accessibility smoke, EDR smoke | Self-hosted |
| **Manual release validation** | MSIX install/upgrade/uninstall, SmartScreen/Defender review, crash recovery drill, accessibility pass, signing verification | Clean release machine |

### 5.1 `bongterm-test-kit`

Workspace-internal crate. Not published. Provides:

- Mocks: `MockTerminalSession`, `MockAgentAdapter`, `MockMcpTransport`, `MockSecretStore`, `MockPolicyEvaluator`, `MockRendererBackend`, `MockProcessGovernor`, `MockStorageRepository`, `MockSettingsProvider`.
- Conformance suites — every concrete impl runs the suite:
  - `terminal_session_conformance::run(session)`
  - `renderer_backend_conformance::run(backend)`
  - `agent_adapter_conformance::run(adapter)`
  - `mcp_transport_conformance::run(transport)`
  - `secret_store_conformance::run(store)`
  - `policy_evaluator_conformance::run(evaluator)`
  - `process_governor_conformance::run(governor)`
  - `storage_repository_conformance::run(repo)`
  - `settings_provider_conformance::run(provider)`
- **Negative conformance suite** for security:
  - missing secret must fail closed
  - advisory must never be rendered as "blocked"
  - deny decision must not spawn a process
  - approval-required decision must not execute before approval
  - redacted exports must not contain known synthetic tokens
- Deterministic clock + seedable RNG for reproducible runs.
- Fixture loaders for `tests/fixtures/`.

### 5.2 Property test invariants

| Subject | Property |
|---|---|
| VT parser | Never panics on arbitrary bytes; emitted events round-trip parse(serialize(x)) ≡ x for serializable subsets. |
| Surface mutation | Grid dimensions remain within configured bounds for any event sequence. |
| Redactor | Applying redactor twice equals applying once (idempotent); output length is bounded relative to input. |
| Settings migration | `migrate(v_n → v_n+1 → v_n) ≡ migrate(v_n → v_n)` for backward-tolerant fields. |
| Sidecar chunks | Reader decodes any sequence of writer outputs; checksum mismatch triggers recovery with a truncation marker. |
| Attribution merge | Merging high+low confidence does not exceed high; the mixed-author flag is monotonic. |
| Resource ledger | Sample sum ≤ kernel-reported total for the process tree (bounded by sample skew). |

### 5.3 Fuzz targets

Initial set in `crates/bongterm-term/fuzz/`:

- `vt_parser` — feeds arbitrary bytes to the termwiz wrapper; assert no panic
- `osc_8_hyperlink` — OSC 8; spoofing detection must hold
- `osc_52_clipboard` — clipboard escape; must be gated
- `settings_json5` — `bongterm-settings` parser
- `chunk_header` — sidecar chunk reader

Corpora are committed under `tests/fixtures/fuzz_corpora/`. Crashes are added to the corpus on discovery.

### 5.4 Benchmark gates

Benchmarks run with warmup and at least 5 samples. A hard-budget violation fails only after two consecutive runs or a statistically significant regression. Regression > 5 % warns. Regression > 10 % fails unless the metric remains within absolute budget and variance exceeds the configured confidence threshold.

| Bench | Threshold | Class |
|---|---:|---|
| `parser_throughput` | ≥ 60 MB/s (10 % over the 50 MB/s budget) | hard gate |
| `parser_alloc_per_mb` | ≤ 100 allocs/MB MVP-0, ≤ 25 allocs/MB v1 stretch | hard gate (MVP-0 threshold) |
| `parser_copy_per_mb` | ≤ 1 copy/MB | hard gate |
| `render_p99_latency_60hz` | ≤ 14 ms | hard gate |
| `render_p99_latency_120hz` | ≤ 7 ms | soft gate (warn only) |
| `scrollback_materialize_1m` | ≤ 200 ms | hard gate |
| `idle_cpu_1pane_60s` | ≤ 0.1 % | hard gate |
| `4pane_idle_cpu_60s` | ≤ 0.4 % | hard gate |
| `core_rss_1pane` | ≤ 120 MB | hard gate |
| `per_pane_rss` | ≤ 25 MB | hard gate |
| `vram_total_4pane` | ≤ 256 MB (RTX 2050) / ≤ 128 MB (iGPU) | hard gate |
| `stream_10mb_throughput` | ≥ 50 MB/s (RTX 2050) / ≥ 30 MB/s (iGPU) | hard gate |
| `battery_drain_4pane_idle` | ≤ 2 %/hour on battery | soft gate, warn-only MVP-0 |
| `block_detection_latency_p99` | ≤ 5 ms after `command_end` marker | hard gate |
| `block_detection_false_positive_rate` | 0 on fixture corpus | hard gate |
| `glyph_atlas_eviction_4pane` | within VRAM ceiling | hard gate |
| `dirty_region_merge_cost_p99` | ≤ 1 ms | hard gate |
| `render_large_scrollback_seek_p99` | ≤ 50 ms | hard gate |
| `frame_drop_rate_10mb_stream` | ≤ configured threshold | hard gate |

Bench results are published as a Markdown comment on the PR by `cargo xtask bench-report`.

### 5.5 Snapshot tests (`insta`)

- OSC sequence inputs → Surface cell snapshot
- Settings schema migration → output JSON5 snapshot
- Redactor outputs for the known token corpus → redacted snapshot
- Agent transcript classifier outputs for sample CLI sessions
- Error message templates (i18n-ready key + default English)
- Renderer golden frames (canonical states only): cursor styles, selection, block background, agent attribution marker, resource heatmap overlay, ligature on/off, emoji / wide glyph, high-DPI scale, dark/light theme contrast
- Queue/error state snapshots: transcript persistence paused, audit persistence unavailable, renderer device lost, MCP unhealthy after restart backoff, JobObject cap exceeded, settings migration failed

### 5.6 Coverage targets

| Path | Target |
|---|---|
| Core security paths (`bongterm-vault-windows`, `bongterm-security`, secret resolution) | ≥ 90 % line + **branch coverage review for allow/deny/require-approval/advisory** decisions |
| Parser, redactor, settings migration, attribution | ≥ 85 % line |
| Renderer (`bongterm-render`) | No line target — relies on benchmarks + snapshots + GPU smoke |
| UI (`bongterm-ui`) | Unit-test view models and state; no framework-rendering line target |

Security paths include mutation-style negative tests proving known-bad strings/actions are blocked or redacted.

Nightly `cargo llvm-cov` reports coverage. No PR-level coverage gate.

### 5.7 Crash, recovery, and forbidden-abstraction tests

Crash-recovery suite:

- Simulated pane panic
- Simulated renderer device-lost
- Simulated MCP crash loop
- Simulated SQLite busy
- Simulated sidecar torn-write
- Simulated disk quota exceeded

In every case the recovery screen must show correct restore/discard/export actions.

Forbidden-abstraction tests (per PRD v7 §3.2):

- no DLL injection patterns
- no global keyboard hooks
- no undocumented ntdll syscall dependency
- no hidden console scraping
- no auto-installed `npx -y` MCP command
- no child process launched outside the ProcessGovernor for agents or MCP

These start as static/code-review gates and become runtime process-tree checks.

Packaging/security tests:

- `cargo xtask sbom` validates the CycloneDX file
- `THIRD_PARTY_NOTICES.md` is included in the MSIX
- MSIX signature validates
- Unsigned dev builds are clearly labeled
- Packaged app starts without dev env vars

Install/Update/Uninstall smoke:

- Install signed MSIX on a clean Windows user profile
- Launch
- Verify Start Menu shortcut
- Verify settings/user-data directory creation
- Upgrade over previous build
- Uninstall
- Verify no process remains
- Verify user data retention/removal choice is honored

### 5.8 Single-test commands

```powershell
cargo test -p bongterm-blocks
cargo test -p bongterm-blocks block_boundary_powershell_high
cargo test --test agent_lifecycle
$env:PROPTEST_CASES = "5000"; cargo test -p bongterm-security redactor_idempotent
cargo fuzz run vt_parser -- -max_total_time=60
cargo bench -p bongterm-term -- parser_throughput
cargo xtask bench-report --gate
cargo insta review
cargo xtask secret-leak-corpus
cargo xtask prompt-injection-corpus
```

### 5.9 Not tested in MVP-0

- Worktrees (deferred to v1)
- Plugin sandbox (deferred to post-MVP-0)
- Remote / SSH dev-box runners (deferred)
- Localization beyond key extraction
- WSL2 mirrored networking (best-effort, no test gate)
- Windows Server / RDP rendering (no test gate)
- Multi-monitor mixed-DPI beyond a smoke run

---

## 6. MVP-0 Deliverable Definition

### 6.1 Exit criteria (31 gates)

Each row is a checkable acceptance gate with severity. **All P0 gates must be green before MVP-0 ships.** P1 gates may carry documented exceptions for experimental/dev-channel releases only.

| # | Severity | Criterion | Observable via |
|---|:-:|---|---|
| 1 | P0 | PowerShell 7, Windows PowerShell, CMD, Git Bash, WSL default distro, SSH launch correctly. **SSH scope**: MVP-0 supports launching the local `ssh` executable as a shell profile only. MVP-0 does **not** implement a remote-dev runner, persistent remote session daemon, file sync, remote agent runner, or BongTerm-specific SSH protocol. Those are post-MVP-0. | Shell-smoke nightly green on all 6 profiles |
| 2 | P0 | Keystroke-to-glyph p99 ≤ 16 ms on RTX 2050; ≤ 8 ms stretch on 120 Hz | `render_p99_latency_60hz` bench passes; 120 Hz soft-gate green |
| 3 | P1 | High-output stream ≥ 50 MB/s on RTX 2050, ≥ 30 MB/s on iGPU | `stream_10mb_throughput` bench passes both targets |
| 4 | P0 | Cold startup ≤ 300 ms (warm cache, 1 pane); shell-integration ready ≤ 800 ms | Startup bench passes |
| 5 | P0 | Core RSS ≤ 120 MB / 1 pane; per-pane ≤ 25 MB; VRAM ceiling ≤ 256 MB (RTX) / ≤ 128 MB (iGPU) | Resource benches pass |
| 6 | P0 | Idle CPU ≤ 0.1 % / 1 pane / 60 s; ≤ 0.4 % / 4 panes | Idle bench passes |
| 7 | P0 | Tabs, split panes (horizontal + vertical), resize, focus cycle work | Integration test `pane_split_resize_focus` green |
| 8 | P0 | Command blocks render with confidence labels for PS, Bash/WSL; CMD heuristic only | Block-detection bench passes; fixture corpus replay green |
| 9 | P0 | Cmd-K NL→command produces preview-only suggestion via Claude Code subprocess; never auto-executes | E2E test with mock `claude` binary; verifies preview rendered, no spawn until user confirms |
| 10 | P0 | Failed-command explainer triggers on non-zero exit blocks; uses block + transcript context | E2E test simulates failed `cargo build`; verifies explainer button and invocation |
| 11 | P1 | Smart history filters `cwd:`, `branch:`, `agent:`, `exit:`, `time:`, `shell:`, `duration:` work | Unit + integration tests pass |
| 12 | P1 | Snippets with `${param:name}` placeholders work; parameter prompt before run | Integration test `snippet_parameter_prompt` green |
| 13 | P1 | Background jobs run + desktop toast on completion/failure | E2E test launches `sleep 3 && exit 1`; verifies toast |
| 14 | P1 | Clickable error patterns work for Node, Python, Rust, .NET, TS | Pattern corpus snapshot tests green |
| 15 | P0 | Claude Code + Codex CLI profiles launch if installed; sidebar shows status; transcript captured | Adapter conformance + integration tests green |
| 16 | P0 | MCP manual JSON import works; permissions visible; JobObject caps enforced; logs visible; no `npx -y` auto-run allowed | MCP integration tests green; forbidden-abstraction test green |
| 17 | P0 | Resource dashboard shows BongT + shell + conhost + agent + MCP + plugin attribution per pane/session | Manual smoke + integration test `dashboard_attribution_count` |
| 18 | P0 | Narrator reads active terminal text, scrollback, command blocks, tabs/panes, main controls | Accessibility smoke nightly green |
| 19 | P0 | Telemetry off by default; diagnostic export shows redaction preview before any send | Integration test `diagnostic_export_no_send_without_consent` green |
| 20 | P0 | Signed MSIX installer + package validates + uninstall clean | Install/Update/Uninstall smoke green |
| 21 | P0 | Zero EDR-hostile techniques: no injection, no hooks, no hidden console scraping, no undocumented syscalls | Forbidden-abstraction test green; Defender real-time smoke green |
| 22 | P0 | Dogfood gate (§6.2) passes | Self-report log review |
| 23 | P0 | Secret-leak corpus regression = 0 known leaks | Full corpus run green |
| 24 | P0 | Prompt-injection corpus: agent never executes destructive action from poisoned content without approval | Corpus run green |
| 25 | P1 | Renderer device-loss recovery: forced DXGI device-removed → automatic recovery without scrollback loss | Device-loss smoke green |
| 26 | P0 | Crash/recovery: simulated pane panic + renderer panic + MCP crash loop → recovery screen correct | Crash-recovery suite green |
| 27 | P0 | No P0 or P1 terminal correctness defect open | Defect tracker review |
| 28 | P0 | Settings/profile/keybindings load + validation failure + backup + Safe Mode fallback work | `settings_migration_and_last_known_good` integration test |
| 29 | P0 | SQLite WAL + sidecar chunk recovery handles torn write + checksum mismatch + corrupt DB without silent transcript fabrication | `storage_recovery_suite` green |
| 30 | P0 | `cargo-deny`, `THIRD_PARTY_NOTICES`, SBOM, vendored WezTerm attribution all valid | `cargo xtask sbom` + `check-licenses` green |
| 31 | P0 | BongTerm never auto-installs agent CLIs or MCP servers; never `npx -y` | `forbidden-install-policy` test green |

25 P0 gates, 6 P1 gates.

### 6.2 Solo-dev dogfood gate

PRD v7 §20.2 asks for 10 internal/trusted developers as the dogfood pool. For a solo developer that translates into a two-stage gate.

**Stage A — Self-dogfood (mandatory)**

- BongTerm is the default terminal for **30 consecutive working days**.
- Any fallback to another terminal must be logged in `docs/dogfood/<date>.md` with reason, duration, and whether it represents a BongTerm blocker. ("No other terminal" is impractical when BongTerm itself is broken; the discipline is to log, not to abstain.)
- At least one agent task (Claude Code or Codex CLI) runs each working day.
- Stage A daily/weekly workload coverage minimums:
  - ≥ 1 long-running command per week
  - ≥ 1 failed-command explainer use per week
  - ≥ 1 Cmd-K use per week
  - ≥ 1 shell switch across the week
  - ≥ 1 agent run per working day
  - ≥ 1 MCP server session per week if MCP ships in MVP-0
  - ≥ 1 crash/recovery drill per week (simulated, not waiting for real ones)
- Zero P0 / P1 terminal correctness bugs open at end of cycle.
- Zero confirmed secret leaks across all dogfood transcripts.

**Stage B — Trusted-circle dogfood (3–5 people, 14 days)**

- Recruit 3–5 developers (friends, ex-coworkers, niche developer communities such as r/rust, r/PowerShell, r/commandline) willing to use BongTerm as primary or secondary terminal.
- Provide signed dev-channel MSIX + private feedback channel (Discord/Matrix).
- Each user completes at least one agent workflow.
- Aggregate findings; no public-facing defect.

**Dogfood requirement by release type**

| Release type | Dogfood requirement |
|---|---|
| Private `0.1.0-mvp0-rc.X` | Stage A required |
| Public experimental `0.1.0-mvp0` | Stage A required; Stage B optional but disclaimer required on landing page |
| Public non-experimental `0.1.x` | Stage A + Stage B required |

### 6.3 Versioning

| Version | Meaning |
|---|---|
| `0.0.x-dev` | Pre-MVP-0 internal builds; no exit criteria gate |
| `0.1.0-mvp0-rc.X` | MVP-0 release candidates |
| `0.1.0-mvp0` | MVP-0 public experimental |
| `0.1.x` | MVP-0 patches; Stage A + Stage B required for public non-experimental |
| `0.2.0` | v1 (worktrees, attachments, devcontainer, branch graph, replay editor, cross-shell translator, HTTP/REST pane) |
| `0.3.x` | v1.1 (MCP shared host pool, plugin marketplace gate, DB query pane, runbook mode) |
| `1.0.0` | Feature-stable, EV-signed, enterprise-ready |

Feature-flagged hidden experimental scaffolding does not change the release version. Only user-facing enabled-by-default features force reclassification.

### 6.4 Release artifact set for `0.1.0-mvp0`

GitHub release artifacts:

- `BongTerm-0.1.0-mvp0-x64.msix` (signed)
- `BongTerm-0.1.0-mvp0-x64.msix.cer`
- `BongTerm-0.1.0-mvp0-x64.sha256`
- `checksums.txt` + `checksums.txt.sig`
- `attestation.intoto.jsonl` (SLSA provenance)
- `THIRD_PARTY_NOTICES.md`
- `sbom.cdx.json` (CycloneDX)
- `benchmark-report.md` (results on the RTX 2050 reference)
- `CHANGELOG.md`
- `known-issues.md`
- `SECURITY.md`
- `INSTALL.md` (signature verification + SmartScreen guidance)

Portable ZIP is dev-channel optional only — it must not become the primary distribution path because it weakens update, signing, and SmartScreen behavior. Winget, Microsoft Store submission, and EV cert are deferred to post-`0.1.x`.

### 6.5 Explicit out-of-MVP-0 (binding)

These features cannot enter `0.1.0-mvp0` regardless of progress:

- Worktrees + agent attribution UI (v1)
- Drag-drop file attachment (v1)
- Devcontainer runner (v1)
- Branch graph, replay editor, cross-shell translator (v1)
- HTTP/REST pane, DB query pane (v1.1)
- MCP shared host pool (v1.1)
- Plugin marketplace and WASM plugins beyond placeholder (post-v1)
- Runbook/notebook mode (post-v1)
- PR comment browser (post-v1)
- Markdown review suite (post-v1)
- Command Lens (post-v1)
- Durable session daemon (post-v1)
- Windows 10 / Windows Server / RDP certification (post-MVP-0 best-effort only)
- WSL2 mirrored networking deep test (post-MVP-0)
- Localization (key extraction acceptable in MVP-0; full L10n post-MVP-0)
- True mid-session agent steering (Capability Level 5) — never promised
- Cloud sync, team collaboration (no plan)

### 6.6 Ship-when checklist

`v0.1.0-mvp0` ships only when:

- **For public experimental `0.1.0-mvp0`**: all P0 §6.1 gates green for ≥ 7 consecutive nightly runs; any P1 exceptions documented in `known-issues.md` with rationale and timeline.
- **For public non-experimental `0.1.x`**: all P0 **and** P1 §6.1 gates green for ≥ 7 consecutive nightly runs; no exceptions.
- §6.2 Stage A complete; Stage B complete or explicit experimental-disclaimer accepted.
- Public GitHub repo flipped from private.
- Trademark search complete (USPTO + EUIPO + Indian TM DB + GitHub/npm/crates/domain availability).
- **Brand perception review complete.** The name "BongTerm" carries a "bong" connotation (drug paraphernalia) in some markets. Document the brand decision in `docs/adr/0002-product-name.md` before public launch. Not a blocker by itself; documented decision required.
- SmartScreen warm-up plan in `docs/runbook/smartscreen.md`.
- Security disclosure inbox monitored (address in `SECURITY.md`).
- Release runbook executed end-to-end on the dedicated clean release machine.
- Code-signing certificate tested on a clean VM.
- Install/uninstall tested on a clean Windows user profile.
- `SECURITY.md` includes supported versions and vulnerability intake path.
- Privacy notice exists (even though telemetry is off by default).
- `known-issues.md` published.
- Rollback plan exists for a bad release.
- GitHub release draft reviewed against the release checklist.

---

## 7. Risks, De-risking Spikes, Decision Triggers

Solo dev = single owner. All risks owned by the developer. No team allocation.

### 7.1 Scoring legend

```
Probability:  L = 1, M = 2, H = 3
Impact:       L = 1, M = 2, H = 3
P×I:          1–2 = watch
              3–4 = active mitigation
              6–9 = top planning risk
Confidence (optional): Low / Medium / High
```

### 7.2 Risk register

| # | Risk | P | I | P×I | Mitigation | Trigger to escalate |
|---|---|:-:|:-:|:-:|---|---|
| R1 | `wezterm-term` API breaks across submodule version bumps | M | H | 6 | Pin submodule to immutable commit/tag; `bongterm-term` is the only crate touching `wezterm-term`, behind a stable internal trait; ADR per submodule bump | A bump requires changes outside `bongterm-term` or costs more than one day to absorb |
| R2 | wgpu + glyphon on RTX 2050 + Optimus laptop fails p99 latency budget | M | H | 6 | Spike S1; pin discrete GPU mode during bench runs; document Optimus quirks; allow software-render fallback for repro | S1 p99 > 16 ms after warmup and two optimization passes; > 24 ms triggers Approach C unless a single proven bottleneck has a bounded fix |
| R3 | Glyph atlas eviction policy under-budgets VRAM at 4+ panes | M | M | 4 | Spike S2; explicit LRU + size cap in `bongterm-render`; per-pane VRAM is estimated, global ceiling enforced | 4-pane VRAM total > 256 MB on RTX 2050 (or > 128 MB on iGPU) or atlas eviction causes visible jank |
| R4 | Claude Code CLI changes non-interactive output format breaking Cmd-K + explainer | M | M | 4 | `ClaudeCodeAdapter` isolated; classifier is stateful + versioned; capability detection per launch; UI labels degraded gracefully | New `claude` minor version emits unparseable output → adapter falls back to "Cmd-K unavailable" + warning |
| R5 | Codex CLI auth model changes (subscription / device flow / API key swaps) | M | M | 4 | `CodexCliAdapter` capability detection; `auth_state()` reports honestly; install/configure flow surfaced | `auth_state()` returns Unknown on a supported Codex version |
| R6 | ConPTY OSC ordering remains unreliable for prompt frameworks | H | M | 6 | Confidence labels per shell/framework; fallback heuristic mode; compatibility matrix; per-framework fixture tests | More than 30 % of dogfood blocks land in Low/Unsupported confidence |
| R7 | Iced + wgpu integration on Windows has DPI/IME edge cases | M | M | 4 | Spike S3 — IME composition over wgpu-rendered terminal cell; per-monitor DPI v2 smoke; fallback to native HWND-hosted wgpu with Iced for chrome only | S3 fails; trusted-circle dogfood reports broken CJK input |
| R8 | SmartScreen reputation cold-start hurts adoption for new-cert builds | H | M | 6 | OV cert for MVP-0, evaluate EV post-`0.1.x`; `docs/runbook/smartscreen.md` warm-up plan; checksum + verify-signature instructions in `INSTALL.md` | First 30 days post-public-launch: more than 50 % users hit a SmartScreen warning |
| R9 | Windows Defender / enterprise EDR flags BongT for process supervision patterns | M | H | 6 | Zero injection/hooks/scraping (forbidden-abstraction tests gate); EDR-friendly smoke nightly with Defender real-time on; security whitepaper covers ConPTY/JobObject/PolicyEvaluator/secret model; allowlist guidance | Any Defender quarantine or repeat warning blocks release until resolved |
| R10 | Indirect prompt injection via terminal output / MCP result / poisoned diff triggers destructive action | M | H | 6 | All agent-ingested content untrusted (PRD v7 §35.5); approval gates at brokered or stronger enforcement; per-agent MCP allowlist; prompt-injection corpus nightly; UI labels every approval with enforcement level | Corpus regression; user reports destructive action without approval |
| R11 | Secret leak in transcript/export/diagnostics despite redactor | M | H | 6 | Redactor regression corpus; raw terminal display preserved but persistence/export/context redacted; raw-history mode encrypted + opt-in; nightly corpus gate | Any synthetic token leak in the nightly export pipeline |
| R12 | SQLite primary corruption loses local source-of-truth | L | H | 3 | WAL + `synchronous=NORMAL` + nightly `PRAGMA integrity_check`; recovery from append-only chunks with blake3; never fabricate; read-only fallback on unrecoverable corrupt | Recovery from chunks succeeds < 95 % in fault-injection tests |
| R13 | Glyphon API churn — pre-1.0 library, breaking changes likely | M | M | 4 | Pin glyphon minor; `bongterm-render` isolates glyphon usage; ADR per upgrade; migration to `swash` + custom atlas if churn becomes blocker | Breaking change costs > 3 days to absorb |
| R14 | Iced pre-1.0 churn | M | M | 4 | Pin Iced minor; UI code patterns kept narrow; reconsider versus Slint (paid) or `egui` if churn becomes blocker | Breaking change costs > 5 days |
| R15 | MCP server ecosystem fragmentation (stdio vs HTTP vs streamable) | M | M | 4 | MVP-0 = stdio only; one process per server; HTTP loopback adapter deferred to v1.1 with shared host pool | More than 3 user-reported MCP servers BongTerm cannot run via stdio during dogfood |
| R16 | Solo dev burnout / scope creep | M | H | 6 | Hard MVP-0 cutline (§6.5); orca.md tasks struck on completion (no back-fill); say no to PRs that add deferred features; weekly cutline review; WIP cap = 2 active implementation branches; no feature PRs while any P0 gate is red for more than 48 h; parking-lot file for ideas not in active scope | Two consecutive weeks of zero P0 progress → mandatory scope review + 3-day cooling-off; then resume, cut P1, or pause |
| R17 | Trademark / brand-perception risk for "BongTerm" | L | M | 2 | Brand review precedes public launch (per §6.6); document decision in `docs/adr/0002-product-name.md`; rename option preserved | Trademark search blocks usage OR perception review concludes high-friction in target geographies |
| R18 | Dogfood Stage B circle hard to recruit | M | L | 2 | Stage B optional for experimental release; recruit via niche developer communities at Stage A completion | < 2 committed users by day 21 of the Stage B window |
| R19 | Open-source license friction (Iced MIT + WezTerm MIT + BongT Apache-2.0 interaction) | L | M | 2 | `deny.toml` bans GPL/AGPL in core; `cargo-deny` license check in CI; `THIRD_PARTY_NOTICES.md` auto-generated; legal review at Stage B start | License audit produces a compliance gap |
| R20 | wgpu D3D12 backend regression on Optimus discrete-to-iGPU transition | L | M | 2 | Device-loss recovery (§4.2); fallback iGPU bench target; Optimus mux behavior documented in `docs/troubleshooting/optimus.md` | User report of black screen on unplug |
| R21 | Packaging / MSIX / signing failure on clean machines | M | M | 4 | Clean-VM release smoke; signed MSIX validation; install/upgrade/uninstall test gate | Release candidate fails clean VM install or upgrade |
| R22 | Append-only sidecar chunks grow without effective retention/compaction | M | M | 4 | Retention quotas; orphan cleanup (`cargo xtask cleanup-chunks`); storage dashboard; fault-injection tests | Dogfood data dir exceeds configured quota OR cleanup deletes live references |
| R23 | Narrator/UIA smoke passes but real screen-reader UX remains poor | M | M | 4 | MVP baseline limited to active text, scrollback, block nav, tabs/panes; trusted-circle accessibility feedback if available | Narrator cannot reliably read active terminal text or command blocks |

**Top planning risks (P×I = 6)**: R1, R2, R6, R8, R9, R10, R11, R16.

### 7.3 De-risking spikes

Spikes are time-boxed proofs-of-feasibility. Failing a spike triggers a revised approach via ADR, not project abandonment. Spikes live in `tools/spikes/` and are deleted after their ADR is written.

**Wave 0 — architecture-gating (must finish before renderer/UI implementation proceeds):**

| Spike | Time-box | Resolves | Exit artifact |
|---|:-:|---|---|
| S1 — `wgpu + glyphon` p99 latency on RTX 2050 / Optimus | 2 weeks | R2 | Bench numbers + ADR-002 commits to Approach B implementation envelope or triggers ADR-001 fallback |
| S2 — VRAM ceiling under 4-pane stress with shared atlas + LRU | 1 week | R3 | Bench + ADR-003 atlas eviction policy |
| S3a — Iced + `bongterm-render` device integration shape | 4 days | R7 (architecture) | Decision among (a) Iced `Shader` widget hosting BongTerm wgpu rendering as a custom primitive inside an Iced view (single device, complex), (b) multi-window: terminal pane as a native HWND wgpu window with Iced owning chrome around it (two devices, focus/DPI/IME handoff), (c) render-to-texture: BongTerm produces a texture per frame exposed to Iced as `Image` (high copy cost, simple). Multi-pane scaling tested. Exit: ADR-004a |
| S3b — IME composition on selected S3a shape | 3 days | R7 (input) | Working CJK input demo on the shape chosen in S3a, including candidate-window positioning relative to caret, compose/cancel/commit semantics, surrogate pairs, grapheme clusters. Exit: ADR-004b |
| S4 — `wezterm-term` API stability survey | 3 days | R1 | Git-log review of last 12 months of `wezterm-term` changes + ADR-005 submodule policy |

**Wave 1 — release-gating (may run in parallel with MVP-0 implementation):**

| Spike | Time-box | Resolves | Exit artifact |
|---|:-:|---|---|
| S5 — Claude Code non-interactive output reliability across last 3 versions | 3 days | R4 | Output format compatibility matrix + ADR-006 |
| S6 — Codex CLI auth flow end-to-end | 2 days | R5 | Adapter conformance fixtures + ADR-007 |
| S7 — Defender + EDR-friendly process supervision smoke | 3 days | R9 | Defender clean log + ADR-008 process-tree pattern |
| S8 — Prompt-injection corpus seed (≥ 30 hand-crafted scenarios) | 3 days | R10 | Committed corpus + ADR-009 approval-gate policy |

Total spike cost: ~5–6 weeks of Wave 0 + ~13 days of Wave 1 (which overlaps implementation).

### 7.4 Decision triggers

If a trigger fires, the required action is mandatory — not optional review.

| Trigger | Required action |
|---|---|
| S1 fails: p99 > 16 ms after warmup + two optimization passes | Freeze renderer work; run ADR review |
| S1 catastrophic: p99 > 24 ms | Switch to Approach C (use vendored `wezterm-gui` as renderer instead of BongTerm-owned `bongterm-render`) unless a single proven bottleneck has a bounded fix; ADR-001 updated |
| 4-pane VRAM total > 256 MB on RTX 2050 (or > 128 MB on iGPU) OR atlas eviction causes visible jank | Redesign atlas eviction; tighten admission control |
| `wezterm-term` API breaks twice in MVP-0 development cycle (a "break" = changes outside `bongterm-term` or > 1 day to absorb) | Publish a private fork crate; abandon raw submodule path |
| Claude Code non-interactive output unreliable across 2+ versions | Cmd-K + explainer become "supported on Claude Code N only" with version pin; or move to direct Anthropic API with user-supplied key |
| Codex CLI auth flow fundamentally changes | Codex adapter demoted to "community-supported only"; MVP-0 ships with Claude Code as the sole built-in |
| Any Defender quarantine or repeat Defender warning | Blocks release until resolved; EDR mitigation sprint takes priority |
| Secret-leak corpus regression detects any leak | Release blocked; redactor sprint takes priority |
| Prompt-injection corpus regression detects a destructive-action escape | Release blocked; approval-gate sprint takes priority |
| Solo-dev burnout signals (two weeks zero P0 progress) | Mandatory scope review + 3-day cooling-off; then resume, cut P1 scope, or pause |
| SmartScreen blocks > 50 % first-30-day users | EV cert decision accelerated; reputation warm-up plan executed |
| Two consecutive nightly performance budget violations without explainable cause | Freeze new feature work; perf-only sprint until green |

### 7.5 Not on the risk register

These are out-of-scope for MVP-0 risk tracking and should not consume planning attention. Tracking thoughts on these is a burnout/scope-creep signal (R16):

- WSL2 mirrored networking edge cases
- Windows Server compatibility
- macOS / Linux ports
- IDE integration
- Cloud sync
- Team collaboration

---

## 8. ADR Index (seed list)

Each spike produces an ADR. The seed list below is created during Phase 0:

- **ADR-001** — Renderer strategy for MVP-0 (status: Accepted; documented in §0)
- **ADR-002** — wgpu + glyphon latency envelope (created by S1)
- **ADR-003** — Glyph atlas eviction policy (created by S2)
- **ADR-004a** — Iced + `bongterm-render` device integration shape (created by S3a)
- **ADR-004b** — IME composition on selected device shape (created by S3b)
- **ADR-005** — WezTerm submodule policy and pin rotation (created by S4)
- **ADR-006** — Claude Code adapter output format pinning (created by S5)
- **ADR-007** — Codex CLI auth flow handling (created by S6)
- **ADR-008** — Defender / EDR-friendly process-tree pattern (created by S7)
- **ADR-009** — Approval-gate policy against prompt injection (created by S8)

Subsequent ADRs are created whenever a locked decision in this spec is challenged.

---

## 9. UX Contract for MVP-0

The earlier critique flagged missing UX artifacts. This spec resolves engineering risk but the UI cannot be built off prose alone. Before implementation of any user-facing surface begins (Phase 1 onward), the following minimum UX contract must exist as committed artifacts under `docs/ux/`. Fidelity may be low — pen-and-paper sketches scanned to image, or hand-drawn ASCII layouts are acceptable. The goal is to prevent implementation drift, not produce a design portfolio.

Required before implementation:

1. **Main window layout sketch** — title bar, tab strip, side panels (agent sidebar, resource dashboard, settings overlay), terminal surface region, command palette overlay position.
2. **Command palette behavior** — input field, result list, filter syntax (matches §6.1 #11 smart-history filters), keyboard model, escape behavior.
3. **Pane / tab model sketch** — horizontal/vertical split, resize handles, focus indicator, pane title bar (process name + cwd + confidence-label badge for command blocks), maximize/zoom behavior.
4. **Agent sidebar sketch** — status header, current command, files-touched list, transcript link, lifecycle controls (stop / kill / restart with summarized context / export), approval queue presentation, resource-usage strip.
5. **Resource dashboard sketch** — per-process row (BongT / shell / conhost / agent / MCP / plugin attribution), CPU/RSS/VRAM columns, sparkline live indicator, drill-down behavior, "stale/degraded" badge styling for lossy-observable queues.
6. **Error / recovery screen sketch** — banner, affected-items list, per-item Restore/Discard/Export-diagnostics actions, "Suspected culprit" line, "Start in Safe Mode" entry.
7. **First-launch onboarding flow** — shell pick, optional Windows Terminal profile import, theme/contrast pick, shell-integration enable/disable, telemetry consent (off by default), Claude Code / Codex CLI detection result, secret-storage explanation, resource-budget defaults summary.
8. **Keyboard shortcut table** — minimum bindings: command palette (Ctrl+Shift+P), Cmd-K (Ctrl+K), new tab (Ctrl+Shift+T), split pane (Alt+Shift+D), close pane (Ctrl+Shift+W), find in pane (Ctrl+F), smart history (Ctrl+R), explain last failed (Ctrl+Shift+E), open resource dashboard (Ctrl+Shift+R), attach context (Ctrl+Shift+A), toggle background jobs (Ctrl+Shift+J).
9. **Notification taxonomy** — when does BongT toast vs banner vs sidebar badge vs modal? Mapped to events: background job done/failed, agent waiting for approval, resource budget exceeded (modal if launch-blocking; otherwise dashboard warning), dangerous command detected (inline confirmation), MCP server crashed, telemetry export requested (modal with redaction preview).
10. **Design tokens** — minimum: typography scale (3–5 sizes), spacing scale (4 / 8 / 12 / 16 / 24 / 32), radius scale (0 / 4 / 8), motion durations (instant / 120 ms / 240 ms), color tokens (terminal fg/bg/selection/cursor + semantic status: success/warn/danger/info + danger/production-mode tokens + focus-ring tokens), high-contrast mapping, reduced-motion alternatives.

This is a **gate**, not a wish list. Phase 1 implementation cannot start until items 1–10 exist as committed artifacts.

---

## 10. Next Step

After this spec is approved, invoke `superpowers:writing-plans` to produce the ordered implementation plan. The plan output populates `orca.md` at the repo root per the session-start protocol in `CLAUDE.md`.

`orca.md` is created only once. Each completed task is removed from the file in-place. The plan covers all work required to ship `0.1.0-mvp0`, in the following expected order:

1. Phase 0: repo scaffold, submodule import, `cargo xtask doctor`, Wave 0 spikes (S1, S2, S3a, S3b, S4), foundational crates (`bongterm-pty`, `bongterm-term`, `bongterm-test-kit`), plus `bongterm-render` **scaffold only** (crate exists, exposes traits/types, compiles, but no product renderer implementation). Product implementation of `bongterm-render` begins only after ADR-002 (S1 latency envelope), ADR-003 (S2 atlas eviction), ADR-004a (S3a device integration shape), and ADR-004b (S3b IME on selected shape) are accepted.
2. Phase 1: usable terminal — profiles, settings, panes/tabs, palette, command blocks, resource dashboard.
3. Phase 2: agent observability — adapter framework + Claude Code + Codex CLI built-ins, transcript capture, file-change tracking, approvals, replay.
4. Phase 3: developer-UX — Cmd-K, explainer, smart history, snippets, background jobs, clickable patterns.
5. Phase 4: MCP supervision (one-process-per-server), Context Optimizer v1, secrets vault, redaction, dangerous-command policy.
6. Phase 5: hardening — UIA accessibility, IME, DPI, MSIX signing, diagnostics, opt-in crash reporting, parser fuzzing wired into CI; Wave 1 spikes (S5, S6, S7, S8) run during this phase if not earlier.
7. Phase 6: dogfood Stage A → Stage B → public flip.

The plan's structure must mirror this spec section by section so the two stay synchronized.

---

*End of spec.*
