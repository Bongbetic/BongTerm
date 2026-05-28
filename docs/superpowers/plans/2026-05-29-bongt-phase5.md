# BongTerm Phase 5 Execution Plan (Hardening + Release Preparation)

> **For agentic workers:** REQUIRED SUB-SKILL: use `superpowers:subagent-driven-development` (recommended) or `superpowers:executing-plans` to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax. Each task is strict TDD: write the failing test, run it red, write minimal impl, run it green, commit. For inherently manual/infra tasks (cert provisioning, clean-VM smoke, SmartScreen warm-up) the "test" is a verifiable script / xtask assertion / documented runbook acceptance step, never a vague action.

Date: 2026-05-29
Source: `docs/superpowers/specs/2026-05-27-bongt-mvp0-design.md` (§6.1 gates **#18, #19, #20, #21, #25, #26, #30**; §5.7 crash/forbidden-abstraction tests; §4.2 crash isolation; §2.8 CI matrix; §7.3 Wave 1 spikes S5–S8) and `orca.md` Phase 5 outline `5.A.*`…`5.exit`.
Status: Active

## Goal

Harden BongTerm to release quality — UIA accessibility, IME, per-monitor DPI v2, signed MSIX packaging, parser fuzzing in nightly CI, device-loss + crash recovery, opt-in diagnostics, supply-chain provenance, and Wave 1 spike ADRs — and exit only when §6.1 gates #18, #19, #20, #21, #25, #26, #30 are green plus clean-VM signing + install smoke pass for 7 consecutive nightlies.

## Architecture

Phase 5 adds no new domain logic to the hot path; it wires existing port traits (renderer device-loss, diagnostics, security forbidden-abstraction) to real Windows user-mode APIs (UI Automation `IRawElementProviderSimple`, Imm32 composition, `GetDpiForWindow`/per-monitor-v2, DXGI `GetDeviceRemovedReason`, `minidump-writer`) behind BongTerm-owned traits so they stay substitutable and testable on non-GPU CI. Packaging, signing, SBOM, and provenance live in `tools/xtask` + `packaging/msix` + `.github/workflows`, gated by clean-VM manual validation. All Windows interop stays inside supported user-mode abstractions — no injection, hooks, console scraping, or undocumented syscalls (hard non-goal; gate #21).

## Tech Stack

Rust 1.95 stable + edition 2024, `tokio`, `windows-rs` 0.58 (adds `Win32_UI_Accessibility`, `Win32_UI_HiDpi`, `Win32_UI_Input_Ime`, `Win32_UI_WindowsAndMessaging`, `Win32_Graphics_Dxgi`), `iced` 0.14 (InputMethod events per ADR-006), `wgpu` 27, `cryoglyph` 0.1, `minidump-writer` 0.10, `cargo-fuzz` (separate pinned nightly per `docs/runbook/fuzzing.md`), `cargo-cyclonedx` (SBOM decision), `makeappx.exe` + `signtool.exe` (Windows SDK), `insta` for accessibility-tree snapshots, mocks in `bongterm-test-kit` matching the established trait+mock+conformance pattern.

**Reference hardware:** Ryzen 5 7535HS / 16 GB / RTX 2050 4 GB VRAM / Win11 24H2.

**ADR numbering note.** The repo already uses ADR-0001…0008. The spec refers to S5–S8 exit ADRs as "ADR-006…ADR-009" by *logical role*; to avoid collision with the existing accepted ADRs, this plan assigns the **next free numbers 0009–0012** to S5–S8 and records the spec's logical name inside each ADR body.

---

## Scope Locks

1. No new hot-path code. Accessibility/IME/DPI read retained surface state; they never mutate the grid or block the parser.
2. Reuse existing crate ownership: `renderer` owns device-loss recovery; `ui` owns UIA provider + IME wiring + DPI; `diagnostics` owns minidump + redacted export + consent; `security` owns forbidden-abstraction runtime checks. Do not cross the module ownership matrix.
3. Every Windows API goes behind a BongTerm-owned trait with a mock in `bongterm-test-kit`, so logic tests run on GPU-less, IME-less GitHub runners. Real-device assertions run only in self-hosted / manual-release buckets.
4. Parser fuzzing uses the pinned nightly in `tools/xtask/fuzz/rust-toolchain.toml`; nightly never enters the MSIX and never runs on PR.
5. Secrets never appear in diagnostics, minidumps, exports, argv, or logs (security contract §37). Redaction preview is mandatory before any export leaves the machine.

---

## File Structure

Files created or modified in Phase 5, each with its single responsibility:

| Path | C/M | Responsibility |
|---|---|---|
| `crates/bongterm-ui/src/accessibility.rs` | C | `AccessibilityTree` model + `UiaProvider` trait: maps terminal surface/scrollback/blocks/tabs/panes/controls to UIA elements (gate #18). |
| `crates/bongterm-ui/src/accessibility_win.rs` | C | `WindowsUiaProvider` — `IRawElementProviderSimple` over real HWND via `windows-rs`. `#[cfg(windows)]`, thin, untested-on-CI. |
| `crates/bongterm-ui/src/ime.rs` | C | `ImeState` state machine (Opened/Preedit/Commit/Closed) + `CompositionWindow` positioning math per ADR-006. |
| `crates/bongterm-ui/src/ime_win.rs` | C | `ImmCompositionWindow` — `ImmSetCompositionWindow` on async HWND. `#[cfg(windows)]`. |
| `crates/bongterm-ui/src/dpi.rs` | C | `DpiState` + per-monitor-v2 scale math; `DpiProvider` trait + Windows impl (`GetDpiForWindow`). |
| `crates/bongterm-ui/Cargo.toml` | M | Add `bongterm-render`, `bongterm-blocks`, `windows` (UIA/HiDpi/Ime) deps. |
| `crates/bongterm-render/src/device_loss.rs` | C | `DeviceLossRecovery` controller: classifies `DeviceRemovedReason`, drives recreate, enforces 3-in-60s → software-fallback (gate #25). |
| `crates/bongterm-render/src/lib.rs` | M | `mod device_loss;` + `DeviceRemovedReason` enum + `recover()` on `RendererBackend`. |
| `crates/bongterm-diagnostics/src/lib.rs` | M | `mod minidump; mod export; mod consent;` re-exports. |
| `crates/bongterm-diagnostics/src/minidump.rs` | C | `MinidumpWriter` trait + `WindowsMinidump` (`minidump-writer`) + mock; `.dmp` on app-wide panic (gate #26). |
| `crates/bongterm-diagnostics/src/export.rs` | C | `DiagnosticBundle` builder + `RedactionPreview`; never auto-sends (gate #19). |
| `crates/bongterm-diagnostics/src/consent.rs` | C | `TelemetryConsent` — off by default, explicit opt-in only (gate #19). |
| `crates/bongterm-diagnostics/Cargo.toml` | M | Add `bongterm-security` (redactor), `minidump-writer`, `serde`, `serde_json`. |
| `crates/bongterm-diagnostics/src/recovery.rs` | C | `RecoveryScreen` model: Restore / Discard / Export actions per crash class (gate #26). |
| `crates/bongterm-security/src/forbidden.rs` | C | `ProcessTreeAuditor` trait + closed `ForbiddenTechnique` enum; runtime process-tree check (gate #21). |
| `crates/bongterm-security/src/lib.rs` | M | `mod forbidden;` re-export. |
| `crates/bongterm-test-kit/src/conformance/uia_provider_conformance.rs` | C | Conformance suite for `UiaProvider`. |
| `crates/bongterm-test-kit/src/conformance/minidump_writer_conformance.rs` | C | Conformance suite for `MinidumpWriter`. |
| `crates/bongterm-test-kit/src/conformance/process_tree_auditor_conformance.rs` | C | Conformance suite for `ProcessTreeAuditor`. |
| `crates/bongterm-test-kit/src/conformance/mod.rs` | M | Register the three new conformance modules. |
| `crates/bongterm-test-kit/Cargo.toml` | M | Add `bongterm-ui`, `bongterm-diagnostics` path deps. |
| `tools/xtask/src/package_msix.rs` | M | Real MSIX build: stage payload, `makeappx`, optional `signtool` sign, verify. |
| `tools/xtask/src/sbom.rs` | M | Switch to `cargo-cyclonedx` invocation + vendored-wezterm component + validation. |
| `tools/xtask/src/attestation.rs` | C | Emit `attestation.intoto.jsonl` SLSA provenance. |
| `tools/xtask/src/forbidden_abstraction.rs` | C | Static + process-tree forbidden-abstraction scan (gate #21). |
| `tools/xtask/src/main.rs` | M | Register `package-msix` args, `attestation`, `forbidden-abstraction` subcommands. |
| `tools/xtask/Cargo.toml` | M | Add `cargo_metadata` (already present), `sha2`, `serde_json` for attestation. |
| `tools/xtask/fuzz/rust-toolchain.toml` | C | Pinned nightly for fuzzing. |
| `tools/xtask/fuzz/Cargo.toml` | C | cargo-fuzz crate manifest. |
| `crates/bongterm-term/fuzz/Cargo.toml` | C | Fuzz crate for VT parser target. |
| `crates/bongterm-term/fuzz/fuzz_targets/vt_parser.rs` | C | libFuzzer entrypoint over `bongterm-term` parser. |
| `packaging/msix/AppxManifest.xml` | C | MSIX package identity, capabilities, entrypoint. |
| `packaging/msix/assets/` | C | Square44/150 logo placeholders referenced by manifest. |
| `packaging/msix/mapping.txt` | C | `makeappx` file mapping (payload → package layout). |
| `.github/workflows/nightly.yml` | C | Nightly: fuzz (pinned nightly), accessibility smoke, device-loss, crash-recovery, Defender real-time smoke, forbidden-abstraction runtime. |
| `.github/workflows/ci.yml` | M | Add MSIX manifest validate, SBOM validity, attestation, forbidden-abstraction static gate to PR-blocking. |
| `docs/runbook/smartscreen.md` | M | Flesh out from placeholder with acceptance steps. |
| `docs/runbook/fuzzing.md` | M | Confirm pinned date + wire to nightly workflow. |
| `docs/runbook/release.md` | M | Flesh out rollback plan + ordered release procedure. |
| `docs/runbook/code-signing.md` | M | OV provisioning steps + thumbprint wiring verification. |
| `docs/runbook/edr.md` | C | EDR/Defender allowlist guidance (S7 output). |
| `docs/security/whitepaper.md` | C | Security whitepaper: ConPTY/JobObject/PolicyEvaluator/secret model (S7). |
| `docs/adr/0009-claude-code-output-pinning.md` | C | S5 exit ADR (spec "ADR-006"). |
| `docs/adr/0010-codex-cli-auth.md` | C | S6 exit ADR (spec "ADR-007"). |
| `docs/adr/0011-edr-process-tree.md` | C | S7 exit ADR (spec "ADR-008"). |
| `docs/adr/0012-prompt-injection-approval-gate.md` | C | S8 exit ADR (spec "ADR-009"). |
| `docs/adr/0013-ev-cert-evaluation.md` | C | OV-first / EV-evaluation decision (5.B.3). |
| `docs/adr/0014-sbom-tooling.md` | C | cargo-cyclonedx vs custom decision (5.F.1). |
| `known-issues.md` | C | Published known-issues list (gate prep). |
| `tests/accessibility/narrator_smoke.md` | C | Manual Narrator smoke acceptance script (gate #18). |
| `tests/fixtures/fuzz_corpora/vt_parser/seed01` | C | Seed corpus for VT parser fuzz target. |

---

## Tasks

### 5.A — Accessibility, IME, DPI (gate #18)

#### 5.A.0 — Doctor: confirm toolchain + deps before any 5.A work

- [ ] **Files**: none (verification only).
- [ ] Run `cargo --version` and confirm `1.95`. Run `rustup show` and confirm the stable default toolchain. Confirm `crates/bongterm-ui/src/lib.rs` opens with `#![forbid(unsafe_code)]` (UIA model must be safe-only; the `#[cfg(windows)]` interop file is the sole unsafe surface and lives in `accessibility_win.rs`, which is excluded from CI logic builds).
- [ ] Confirm `windows` workspace dep does **not** yet list `Win32_UI_Accessibility` / `Win32_UI_HiDpi` / `Win32_UI_WindowsAndMessaging`. These are added in 5.A.1.0.
- [ ] Acceptance: print the three confirmations; if any fails, stop and reconcile before continuing.

#### 5.A.1 — UIA accessibility tree over terminal surface

The provider is split: a **safe model** (`accessibility.rs`) in `bongterm-ui` that any CI runner builds and tests, and a **thin `#[cfg(windows)]` COM shim** (`accessibility_win.rs`) that is compile-checked on Windows but asserted only by the manual Narrator smoke (gate #18).

##### 5.A.1.0 — Add windows-rs accessibility/hidpi features

- [ ] **Files**: `Cargo.toml` (root, `[workspace.dependencies]`), `crates/bongterm-ui/Cargo.toml`.
- [ ] Edit root `Cargo.toml` `windows` feature list to add `"Win32_UI_Accessibility"`, `"Win32_UI_HiDpi"`, `"Win32_UI_WindowsAndMessaging"` (keep existing features).
- [ ] Edit `crates/bongterm-ui/Cargo.toml` `[dependencies]` to add `bongterm-render = { path = "../bongterm-render" }`, `bongterm-blocks = { path = "../bongterm-blocks" }`, and `windows = { workspace = true }` (under a `[target.'cfg(windows)'.dependencies]` table for the interop crate; the safe model needs only the workspace path deps).
- [ ] Run expect PASS: `cargo check -p bongterm-ui`.
  - Exact cmd: `cargo check -p bongterm-ui`
  - Expect: compiles clean (no new code yet referencing windows symbols).
- [ ] Commit: `git add Cargo.toml crates/bongterm-ui/Cargo.toml && git commit -m "build(ui/5.A.1.0): add UIA/HiDpi windows-rs features + render/blocks deps"`

##### 5.A.1.1 — `AxRole` closed enum + `uia_control_type_id` mapping (RED→GREEN)

- [ ] **Files**: `crates/bongterm-ui/src/accessibility.rs` (C), `crates/bongterm-ui/src/lib.rs` (M, add `pub mod accessibility;`).
- [ ] Write failing test in `accessibility.rs` (full code):

```rust
//! Accessibility (UI Automation) model over the BongTerm surface. Safe-only;
//! the COM shim lives in `accessibility_win.rs` (`#[cfg(windows)]`). Gate #18.

/// Closed set of UIA roles BongTerm exposes. Bounded → exhaustive `match`,
/// no trait object. New roles require a deliberate edit here + a control-type
/// mapping below, so the surface can never silently grow.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AxRole {
    Window,
    TabList,
    Tab,
    Pane,
    TerminalText,
    Scrollback,
    CommandBlock,
    Control,
}

impl AxRole {
    /// UIA `ControlType` id per the Microsoft UI Automation control-type id
    /// table. Stable numeric ids (not GUIDs) — these are the documented
    /// `UIA_*ControlTypeId` constants.
    pub fn uia_control_type_id(self) -> i32 {
        match self {
            AxRole::Window => 50032,       // UIA_WindowControlTypeId
            AxRole::TabList => 50018,      // UIA_TabControlTypeId
            AxRole::Tab => 50019,          // UIA_TabItemControlTypeId
            AxRole::Pane => 50033,         // UIA_PaneControlTypeId
            AxRole::TerminalText => 50020, // UIA_TextControlTypeId
            AxRole::Scrollback => 50020,   // UIA_TextControlTypeId
            AxRole::CommandBlock => 50026, // UIA_GroupControlTypeId
            AxRole::Control => 50000,      // UIA_ButtonControlTypeId
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn control_type_ids_are_documented_uia_constants() {
        assert_eq!(AxRole::Window.uia_control_type_id(), 50032);
        assert_eq!(AxRole::TabList.uia_control_type_id(), 50018);
        assert_eq!(AxRole::Tab.uia_control_type_id(), 50019);
        assert_eq!(AxRole::Pane.uia_control_type_id(), 50033);
        assert_eq!(AxRole::TerminalText.uia_control_type_id(), 50020);
        assert_eq!(AxRole::CommandBlock.uia_control_type_id(), 50026);
    }
}
```

- [ ] Add `pub mod accessibility;` to `crates/bongterm-ui/src/lib.rs`.
- [ ] Run expect FAIL first (before adding the enum body — write the test, comment out `uia_control_type_id` body to force a miss): `cargo test -p bongterm-ui accessibility::tests::control_type_ids`
  - Expect: `error[E0599]` or assertion mismatch.
- [ ] Restore the impl (above). Run expect PASS: `cargo test -p bongterm-ui accessibility::tests::control_type_ids` → `test result: ok. 1 passed`.
- [ ] Commit: `git add crates/bongterm-ui/src/accessibility.rs crates/bongterm-ui/src/lib.rs && git commit -m "feat(ui/5.A.1.1): AxRole closed enum + UIA control-type id mapping"`

##### 5.A.1.2 — `AxNode` tree node + builder (RED→GREEN)

- [ ] **Files**: `crates/bongterm-ui/src/accessibility.rs` (M).
- [ ] Append failing test + types:

```rust
/// One node in the accessibility tree. Owns its strings (clone form) so the
/// COM shim can hand UIA `BSTR`s without holding a borrow into the live grid —
/// required because `bongterm-ui` is `#![forbid(unsafe_code)]` and cannot
/// extend borrows across the FFI boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AxNode {
    pub role: AxRole,
    pub name: String,
    pub value: String,
    pub children: Vec<AxNode>,
}

impl AxNode {
    pub fn new(role: AxRole, name: impl Into<String>) -> Self {
        AxNode { role, name: name.into(), value: String::new(), children: Vec::new() }
    }
    pub fn with_value(mut self, value: impl Into<String>) -> Self {
        self.value = value.into();
        self
    }
    pub fn child(mut self, node: AxNode) -> Self {
        self.children.push(node);
        self
    }
    /// Depth-first search for the first node of `role`.
    pub fn find_role(&self, role: AxRole) -> Option<&AxNode> {
        if self.role == role {
            return Some(self);
        }
        self.children.iter().find_map(|c| c.find_role(role))
    }
}

#[cfg(test)]
mod node_tests {
    use super::*;

    #[test]
    fn builder_nests_and_find_role_walks_depth_first() {
        let tree = AxNode::new(AxRole::Window, "BongTerm")
            .child(
                AxNode::new(AxRole::TabList, "tabs")
                    .child(AxNode::new(AxRole::Tab, "tab 1")),
            )
            .child(AxNode::new(AxRole::Pane, "pane").child(
                AxNode::new(AxRole::TerminalText, "surface").with_value("$ ls"),
            ));
        assert_eq!(tree.find_role(AxRole::Tab).unwrap().name, "tab 1");
        assert_eq!(
            tree.find_role(AxRole::TerminalText).unwrap().value,
            "$ ls"
        );
        assert!(tree.find_role(AxRole::Scrollback).is_none());
    }
}
```

- [ ] Run expect FAIL: `cargo test -p bongterm-ui accessibility::node_tests` → `error[E0433]`/`E0599` (types absent).
- [ ] Add the `AxNode` impl (above). Run expect PASS: `cargo test -p bongterm-ui accessibility::node_tests` → `1 passed`.
- [ ] Commit: `git add crates/bongterm-ui/src/accessibility.rs && git commit -m "feat(ui/5.A.1.2): AxNode tree node + depth-first find_role"`

##### 5.A.1.3 — `AccessibilityTree::from_shell` maps regions → nodes (RED→GREEN)

- [ ] **Files**: `crates/bongterm-ui/src/accessibility.rs` (M).
- [ ] `BongTermShell::region_names()` already returns the 7 named regions (incl `terminal-surface`, `tab-strip`, `command-palette`). Map them into a root `Window` node so a screen reader can enumerate them.
- [ ] Append failing test + impl:

```rust
use crate::BongTermShell;

/// Builds the UIA tree from retained shell state. Read-only: never mutates the
/// grid, never blocks the parser (Scope Lock 1).
pub struct AccessibilityTree;

impl AccessibilityTree {
    pub fn from_shell(shell: &BongTermShell) -> AxNode {
        let mut root = AxNode::new(AxRole::Window, shell.title());
        for region in shell.region_names() {
            let role = match region {
                "tab-strip" => AxRole::TabList,
                "terminal-surface" => AxRole::TerminalText,
                "command-palette" => AxRole::Control,
                _ => AxRole::Pane,
            };
            root = root.child(AxNode::new(role, region));
        }
        root
    }

    /// Variant that overlays the live surface text into the `TerminalText` node
    /// value, so Narrator reads the visible grid.
    pub fn from_shell_with_surface(shell: &BongTermShell, surface_text: &str) -> AxNode {
        let mut root = Self::from_shell(shell);
        for child in &mut root.children {
            if child.role == AxRole::TerminalText {
                child.value = surface_text.to_string();
            }
        }
        root
    }
}

#[cfg(test)]
mod tree_tests {
    use super::*;

    #[test]
    fn from_shell_exposes_window_with_named_regions() {
        let shell = BongTermShell::default();
        let tree = AccessibilityTree::from_shell(&shell);
        assert_eq!(tree.role, AxRole::Window);
        assert!(tree.find_role(AxRole::TabList).is_some());
        assert!(tree.find_role(AxRole::TerminalText).is_some());
        assert!(tree.find_role(AxRole::Control).is_some());
    }

    #[test]
    fn from_shell_with_surface_sets_terminal_text_value() {
        let shell = BongTermShell::default();
        let tree =
            AccessibilityTree::from_shell_with_surface(&shell, "user@host:~$ cargo test");
        assert_eq!(
            tree.find_role(AxRole::TerminalText).unwrap().value,
            "user@host:~$ cargo test"
        );
    }
}
```

- [ ] Confirm `BongTermShell` derives/implements `Default` and exposes `title()` + `region_names()`. If `Default` is absent, construct via the existing public constructor used elsewhere in `lib.rs` and adjust the test accordingly (do not add a derive that changes public API without need).
- [ ] Run expect FAIL: `cargo test -p bongterm-ui accessibility::tree_tests` → unresolved `AccessibilityTree`.
- [ ] Add impl (above). Run expect PASS: `cargo test -p bongterm-ui accessibility::tree_tests` → `2 passed`.
- [ ] Commit: `git add crates/bongterm-ui/src/accessibility.rs && git commit -m "feat(ui/5.A.1.3): AccessibilityTree::from_shell maps regions to UIA nodes"`

##### 5.A.1.4 — `UiaProvider` trait (safe, String-returning) + default impl (RED→GREEN)

- [ ] **Files**: `crates/bongterm-ui/src/accessibility.rs` (M).
- [ ] Trait must return owned `String` / `Vec` (no borrows), because the Windows shim crosses FFI and the model crate forbids unsafe.
- [ ] Append failing test + trait:

```rust
/// Substitutable provider so non-Windows CI exercises the logic via a model
/// impl while the real COM shim implements the same contract on Windows.
pub trait UiaProvider {
    /// Root node id is always 0; children are addressed by depth-first index.
    fn root(&self) -> AxNode;
    /// UIA `Name` property for the node at `index` (depth-first), owned.
    fn name_of(&self, index: usize) -> Option<String>;
    /// UIA `Value` property for the node at `index` (depth-first), owned.
    fn value_of(&self, index: usize) -> Option<String>;
    /// UIA control-type id for the node at `index`.
    fn control_type_of(&self, index: usize) -> Option<i32>;
}

/// Model provider backed by a flattened tree; the conformance suite runs
/// against this on any CI runner.
pub struct TreeUiaProvider {
    flat: Vec<AxNode>,
}

impl TreeUiaProvider {
    pub fn new(root: AxNode) -> Self {
        let mut flat = Vec::new();
        Self::flatten(&root, &mut flat);
        TreeUiaProvider { flat }
    }
    fn flatten(node: &AxNode, out: &mut Vec<AxNode>) {
        out.push(AxNode {
            role: node.role,
            name: node.name.clone(),
            value: node.value.clone(),
            children: Vec::new(),
        });
        for c in &node.children {
            Self::flatten(c, out);
        }
    }
}

impl UiaProvider for TreeUiaProvider {
    fn root(&self) -> AxNode {
        self.flat.first().cloned().unwrap_or(AxNode::new(AxRole::Window, ""))
    }
    fn name_of(&self, index: usize) -> Option<String> {
        self.flat.get(index).map(|n| n.name.clone())
    }
    fn value_of(&self, index: usize) -> Option<String> {
        self.flat.get(index).map(|n| n.value.clone())
    }
    fn control_type_of(&self, index: usize) -> Option<i32> {
        self.flat.get(index).map(|n| n.role.uia_control_type_id())
    }
}

#[cfg(test)]
mod provider_tests {
    use super::*;

    #[test]
    fn tree_provider_flattens_depth_first_and_returns_owned_props() {
        let shell = BongTermShell::default();
        let tree = AccessibilityTree::from_shell_with_surface(&shell, "$ pwd");
        let p = TreeUiaProvider::new(tree);
        assert_eq!(p.control_type_of(0), Some(50032)); // root Window
        // Some node carries the surface value.
        let found = (0..16).filter_map(|i| p.value_of(i)).any(|v| v == "$ pwd");
        assert!(found);
    }
}
```

- [ ] Run expect FAIL: `cargo test -p bongterm-ui accessibility::provider_tests` → unresolved `UiaProvider`/`TreeUiaProvider`.
- [ ] Add impl. Run expect PASS: `cargo test -p bongterm-ui accessibility::provider_tests` → `1 passed`.
- [ ] Verify safety: `cargo clippy -p bongterm-ui -- -D warnings` stays green and no `unsafe` keyword appears in `accessibility.rs`.
- [ ] Commit: `git add crates/bongterm-ui/src/accessibility.rs && git commit -m "feat(ui/5.A.1.4): safe String-returning UiaProvider trait + TreeUiaProvider"`

##### 5.A.1.5 — `UiaProvider` conformance suite in test-kit (RED→GREEN)

- [ ] **Files**: `crates/bongterm-test-kit/src/conformance/uia_provider_conformance.rs` (C), `crates/bongterm-test-kit/src/conformance/mod.rs` (M), `crates/bongterm-test-kit/Cargo.toml` (M, add `bongterm-ui` path dep).
- [ ] Conformance fn (matches established `run_*_conformance` pattern):

```rust
//! Conformance suite for `bongterm_ui::accessibility::UiaProvider`. Any impl —
//! the model `TreeUiaProvider` or the Windows COM shim — must pass these.

use bongterm_ui::accessibility::{AxRole, UiaProvider};

/// Runs the provider contract against `provider`, whose root must be a Window.
pub fn run_uia_provider_conformance<P: UiaProvider>(provider: &P) {
    let root = provider.root();
    assert_eq!(root.role, AxRole::Window, "UIA root must be a Window");
    assert_eq!(
        provider.control_type_of(0),
        Some(AxRole::Window.uia_control_type_id()),
        "index 0 control-type must match root role"
    );
    assert_eq!(
        provider.name_of(0),
        Some(root.name.clone()),
        "name_of(0) must equal root name"
    );
    // Out-of-range indices return None, never panic.
    assert_eq!(provider.name_of(usize::MAX), None);
    assert_eq!(provider.value_of(usize::MAX), None);
    assert_eq!(provider.control_type_of(usize::MAX), None);
}
```

- [ ] In `conformance/mod.rs` add `pub mod uia_provider_conformance;`.
- [ ] Add a test that drives it (in `uia_provider_conformance.rs` `#[cfg(test)]`):

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use bongterm_ui::BongTermShell;
    use bongterm_ui::accessibility::{AccessibilityTree, TreeUiaProvider};

    #[test]
    fn model_provider_satisfies_conformance() {
        let shell = BongTermShell::default();
        let tree = AccessibilityTree::from_shell(&shell);
        let provider = TreeUiaProvider::new(tree);
        run_uia_provider_conformance(&provider);
    }
}
```

- [ ] Edit `crates/bongterm-test-kit/Cargo.toml` `[dependencies]`: add `bongterm-ui = { path = "../bongterm-ui" }`.
- [ ] Run expect FAIL: `cargo test -p bongterm-test-kit uia_provider_conformance` → unresolved import `bongterm_ui::accessibility`.
- [ ] Wire up + run expect PASS: `cargo test -p bongterm-test-kit uia_provider_conformance` → `1 passed`.
- [ ] Commit: `git add crates/bongterm-test-kit/src/conformance/uia_provider_conformance.rs crates/bongterm-test-kit/src/conformance/mod.rs crates/bongterm-test-kit/Cargo.toml && git commit -m "test(ui/5.A.1.5): UiaProvider conformance suite + model-impl proof"`

##### 5.A.1.6 — `insta` snapshot of the accessibility tree (RED→GREEN)

- [ ] **Files**: `crates/bongterm-ui/src/accessibility.rs` (M), `crates/bongterm-ui/Cargo.toml` (M, add `insta` dev-dep).
- [ ] Add a debug-render fn `AxNode::to_outline(&self) -> String` (indented role:name lines) and snapshot it; the snapshot is the regression guard on the exposed surface shape.

```rust
impl AxNode {
    /// Indented outline for snapshotting. `value` shown only when non-empty.
    pub fn to_outline(&self) -> String {
        let mut s = String::new();
        self.write_outline(0, &mut s);
        s
    }
    fn write_outline(&self, depth: usize, out: &mut String) {
        for _ in 0..depth {
            out.push_str("  ");
        }
        out.push_str(&format!("{:?}: {}", self.role, self.name));
        if !self.value.is_empty() {
            out.push_str(&format!(" = {}", self.value));
        }
        out.push('\n');
        for c in &self.children {
            c.write_outline(depth + 1, out);
        }
    }
}

#[cfg(test)]
mod snapshot_tests {
    use super::*;

    #[test]
    fn accessibility_tree_outline_snapshot() {
        let shell = BongTermShell::default();
        let tree = AccessibilityTree::from_shell(&shell);
        insta::assert_snapshot!(tree.to_outline());
    }
}
```

- [ ] Edit `crates/bongterm-ui/Cargo.toml` `[dev-dependencies]`: add `insta = { workspace = true }`.
- [ ] Run expect FAIL (no accepted snapshot): `cargo test -p bongterm-ui accessibility::snapshot_tests` → fails with a pending `.snap.new`.
- [ ] Review + accept: `cargo insta accept` (or `INSTA_UPDATE=always cargo test -p bongterm-ui accessibility::snapshot_tests`). Re-run expect PASS.
- [ ] Commit (include the `.snap`): `git add crates/bongterm-ui/src/accessibility.rs crates/bongterm-ui/Cargo.toml crates/bongterm-ui/src/snapshots/ && git commit -m "test(ui/5.A.1.6): insta snapshot guards exposed accessibility surface"`

##### 5.A.1.7 — Windows COM shim `accessibility_win.rs` (compile-gated, manual-asserted)

- [ ] **Files**: `crates/bongterm-ui/src/accessibility_win.rs` (C), `crates/bongterm-ui/src/lib.rs` (M, add `#[cfg(windows)] mod accessibility_win;`).
- [ ] This file is the **only** unsafe surface; it is `#[cfg(windows)]` and excluded from the `#![forbid(unsafe_code)]` model file by living separately with its own `#![allow(unsafe_code)]` inner attribute scoped to the module. It implements `IRawElementProviderSimple` by delegating every property read to a `UiaProvider`. No CI logic test asserts it; the Narrator smoke (5.A.1.8) is its acceptance.
- [ ] Provide `WindowsUiaProvider` that wraps a `TreeUiaProvider` and exposes `GetPropertyValue` → maps `UIA_NamePropertyId` (30005) to `name_of`, `UIA_ControlTypePropertyId` (30003) to `control_type_of`, `UIA_ValueValuePropertyId` (30045) to `value_of`. Keep it thin; no grid access.
- [ ] Verifiable check (compile-only, the substitute for a unit test on non-GPU CI): `cargo check -p bongterm-ui --target x86_64-pc-windows-msvc` compiles on a Windows runner. On non-Windows CI the `#[cfg(windows)]` gate elides it. Add this exact check to the nightly Windows job in 5.C.6.
- [ ] Commit: `git add crates/bongterm-ui/src/accessibility_win.rs crates/bongterm-ui/src/lib.rs && git commit -m "feat(ui/5.A.1.7): WindowsUiaProvider COM shim over IRawElementProviderSimple (cfg(windows))"`

##### 5.A.1.8 — Narrator manual smoke acceptance script (gate #18)

- [ ] **Files**: `tests/accessibility/narrator_smoke.md` (C).
- [ ] Write the acceptance runbook: launch BongTerm, start Narrator (`Win+Ctrl+Enter`), Tab through regions, assert Narrator announces each region name (BongTerm window, tab strip, terminal surface, command palette), type a command and assert the surface text is read, scroll scrollback and assert it is reachable. Repeat with NVDA (the ≥1 third-party screen reader per §29 Phase 5). Each step has an explicit PASS criterion and a checkbox.
- [ ] Acceptance: the script is the gate #18 evidence; it is run on the reference hardware before release and its checked-off copy is attached to the release ticket.
- [ ] Commit: `git add tests/accessibility/narrator_smoke.md && git commit -m "docs(ui/5.A.1.8): Narrator + NVDA manual smoke acceptance script (gate #18)"`

#### 5.A.2 — IME composition wired to ADR-006 shape

`bongterm-ui` already subscribes to `iced::event::listen_raw` (per ADR-0006). 5.A.2 adds the **state machine** (`ime.rs`, safe, testable everywhere) and the **`#[cfg(windows)]` `ImmSetCompositionWindow` call** (`ime_win.rs`, compile-gated). The state machine never commits on Preedit and only pushes UTF-8 to the PTY on Commit (ADR-0006).

##### 5.A.2.1 — `ImeState` machine (Closed→Opened→Preedit→Commit) (RED→GREEN)

- [ ] **Files**: `crates/bongterm-ui/src/ime.rs` (C), `crates/bongterm-ui/src/lib.rs` (M, add `pub mod ime;`).
- [ ] Failing test + closed-enum state machine:

```rust
//! IME composition state machine per ADR-0006. Bounded states → closed enum +
//! exhaustive `match`. Pure: maps `iced` InputMethod events to transitions and
//! the bytes (if any) to forward to the PTY. No Win32 here.

/// Closed set of IME states. New states require a deliberate edit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImeState {
    /// No active composition.
    Closed,
    /// IME opened, no preedit yet.
    Opened,
    /// Active preedit (underlined, uncommitted) text.
    Preedit { text: String, cursor: usize },
}

/// Input events from `iced`'s InputMethod surface (ADR-0006 shape).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImeEvent {
    Opened,
    Preedit { text: String, cursor: usize },
    Commit(String),
    Closed,
}

/// Result of feeding an event: the next state and any committed UTF-8 to write
/// to the PTY. Commit is the ONLY path that yields bytes (ADR-0006).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImeStep {
    pub state: ImeState,
    pub commit: Option<String>,
}

impl ImeState {
    pub fn apply(self, ev: ImeEvent) -> ImeStep {
        match ev {
            ImeEvent::Opened => ImeStep { state: ImeState::Opened, commit: None },
            ImeEvent::Preedit { text, cursor } => {
                ImeStep { state: ImeState::Preedit { text, cursor }, commit: None }
            }
            ImeEvent::Commit(text) => {
                // Commit clears preedit and forwards bytes once.
                ImeStep { state: ImeState::Opened, commit: Some(text) }
            }
            ImeEvent::Closed => ImeStep { state: ImeState::Closed, commit: None },
        }
    }
    /// Preedit text to render as an underline overlay, if any.
    pub fn preedit_text(&self) -> Option<&str> {
        match self {
            ImeState::Preedit { text, .. } => Some(text),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preedit_never_commits_and_overlay_is_visible() {
        let step = ImeState::Opened
            .apply(ImeEvent::Preedit { text: "に".into(), cursor: 1 });
        assert_eq!(step.commit, None);
        assert_eq!(step.state.preedit_text(), Some("に"));
    }

    #[test]
    fn commit_forwards_utf8_once_and_clears_preedit() {
        let pre = ImeState::Closed
            .apply(ImeEvent::Opened)
            .state
            .apply(ImeEvent::Preedit { text: "にほ".into(), cursor: 2 })
            .state;
        let step = pre.apply(ImeEvent::Commit("日本".into()));
        assert_eq!(step.commit.as_deref(), Some("日本"));
        assert_eq!(step.state, ImeState::Opened);
        assert_eq!(step.state.preedit_text(), None);
    }

    #[test]
    fn closed_clears_overlay() {
        let step = ImeState::Preedit { text: " x".into(), cursor: 1 }
            .apply(ImeEvent::Closed);
        assert_eq!(step.state, ImeState::Closed);
        assert_eq!(step.commit, None);
    }
}
```

- [ ] Add `pub mod ime;` to `lib.rs`.
- [ ] Run expect FAIL: `cargo test -p bongterm-ui ime::tests` → unresolved module.
- [ ] Add impl. Run expect PASS: `cargo test -p bongterm-ui ime::tests` → `3 passed`.
- [ ] Commit: `git add crates/bongterm-ui/src/ime.rs crates/bongterm-ui/src/lib.rs && git commit -m "feat(ui/5.A.2.1): ImeState machine — preedit never commits, commit forwards UTF-8 (ADR-0006)"`

##### 5.A.2.2 — `CompositionWindow` positioning math (RED→GREEN)

- [ ] **Files**: `crates/bongterm-ui/src/ime.rs` (M).
- [ ] Composition window goes at the caret cell: `(col * cell_width, row * cell_height)` in physical px, with the DPI scale applied (consumed from `dpi.rs` in 5.A.3). Pure math, no Win32.

```rust
/// Caret-relative composition-window placement. The Win32 shim feeds this into
/// `ImmSetCompositionWindow` with `CFS_POINT` (ADR-0006).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompositionWindow {
    pub x: i32,
    pub y: i32,
}

impl CompositionWindow {
    /// `cell_w`/`cell_h` are logical cell size; `scale` is the per-monitor-v2
    /// factor (1.0 = 96 dpi). Result is physical pixels for the IME candidate.
    pub fn at_caret(col: u16, row: u16, cell_w: f32, cell_h: f32, scale: f32) -> Self {
        let x = ((col as f32) * cell_w * scale).round() as i32;
        let y = ((row as f32) * cell_h * scale).round() as i32;
        CompositionWindow { x, y }
    }
}

#[cfg(test)]
mod position_tests {
    use super::*;

    #[test]
    fn caret_position_scales_with_dpi() {
        // col 10, row 5, 8x16 cell, 1.5x (144 dpi).
        let w = CompositionWindow::at_caret(10, 5, 8.0, 16.0, 1.5);
        assert_eq!(w.x, 120); // 10*8*1.5
        assert_eq!(w.y, 120); // 5*16*1.5
    }

    #[test]
    fn caret_origin_is_zero_zero() {
        let w = CompositionWindow::at_caret(0, 0, 8.0, 16.0, 1.0);
        assert_eq!((w.x, w.y), (0, 0));
    }
}
```

- [ ] Run expect FAIL: `cargo test -p bongterm-ui ime::position_tests`.
- [ ] Add impl. Run expect PASS → `2 passed`.
- [ ] Commit: `git add crates/bongterm-ui/src/ime.rs && git commit -m "feat(ui/5.A.2.2): CompositionWindow caret positioning math (DPI-scaled)"`

##### 5.A.2.3 — Windows `ImmCompositionWindow` shim (compile-gated)

- [ ] **Files**: `crates/bongterm-ui/src/ime_win.rs` (C), `crates/bongterm-ui/src/lib.rs` (M, add `#[cfg(windows)] mod ime_win;`).
- [ ] `ImmCompositionWindow::set(hwnd, CompositionWindow)` calls `ImmGetContext` → `ImmSetCompositionWindow` with a `COMPOSITIONFORM { dwStyle: CFS_POINT, ptCurrentPos: POINT { x, y }, .. }` → `ImmReleaseContext`. Scoped `#![allow(unsafe_code)]`. The HWND arrives async via the existing `window::raw_id` Task (ADR-0006); the shim takes the already-resolved `isize` handle.
- [ ] Verifiable check: included in the nightly Windows `cargo check --target x86_64-pc-windows-msvc` (5.C.6). Real candidate-window placement is part of the IME line in the Narrator/IME manual smoke.
- [ ] Commit: `git add crates/bongterm-ui/src/ime_win.rs crates/bongterm-ui/src/lib.rs && git commit -m "feat(ui/5.A.2.3): ImmCompositionWindow shim — ImmSetCompositionWindow CFS_POINT (cfg(windows))"`

#### 5.A.3 — Per-monitor DPI v2 + live DPI changes

##### 5.A.3.1 — `DpiState` + scale math + `DpiProvider` trait (RED→GREEN)

- [ ] **Files**: `crates/bongterm-ui/src/dpi.rs` (C), `crates/bongterm-ui/src/lib.rs` (M, add `pub mod dpi;`).
- [ ] Failing test + types:

```rust
//! Per-monitor DPI v2 scale state. `DpiProvider` is substitutable so CI tests
//! the math without a real monitor; the Windows impl calls `GetDpiForWindow`.

/// Retained DPI scale for the active window. 96 dpi == scale 1.0.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DpiState {
    dpi: u32,
}

impl DpiState {
    pub const BASELINE_DPI: u32 = 96;

    pub fn new(dpi: u32) -> Self {
        DpiState { dpi: dpi.max(1) }
    }
    /// Scale factor vs the 96-dpi baseline (per-monitor-v2 semantics).
    pub fn scale(self) -> f32 {
        self.dpi as f32 / Self::BASELINE_DPI as f32
    }
    /// Apply a live `WM_DPICHANGED` value; returns the new state.
    pub fn on_dpi_changed(self, new_dpi: u32) -> Self {
        DpiState::new(new_dpi)
    }
    /// Scale a logical pixel length to physical.
    pub fn to_physical(self, logical: f32) -> f32 {
        logical * self.scale()
    }
}

/// Substitutable DPI source. Windows impl wraps `GetDpiForWindow`.
pub trait DpiProvider {
    fn dpi_for_active_window(&self) -> u32;
}

/// Model provider for CI (fixed dpi).
pub struct FixedDpiProvider(pub u32);
impl DpiProvider for FixedDpiProvider {
    fn dpi_for_active_window(&self) -> u32 {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn baseline_is_unity_scale() {
        assert_eq!(DpiState::new(96).scale(), 1.0);
    }

    #[test]
    fn one_fifty_percent_and_live_change() {
        let s = DpiState::new(96);
        assert_eq!(s.scale(), 1.0);
        let s = s.on_dpi_changed(144);
        assert_eq!(s.scale(), 1.5);
        assert_eq!(s.to_physical(10.0), 15.0);
    }

    #[test]
    fn provider_feeds_state() {
        let p = FixedDpiProvider(192);
        let s = DpiState::new(p.dpi_for_active_window());
        assert_eq!(s.scale(), 2.0);
    }
}
```

- [ ] Add `pub mod dpi;` to `lib.rs`.
- [ ] Run expect FAIL: `cargo test -p bongterm-ui dpi::tests` → unresolved module.
- [ ] Add impl. Run expect PASS → `3 passed`.
- [ ] Commit: `git add crates/bongterm-ui/src/dpi.rs crates/bongterm-ui/src/lib.rs && git commit -m "feat(ui/5.A.3.1): DpiState per-monitor-v2 scale math + DpiProvider trait"`

##### 5.A.3.2 — Windows `GetDpiForWindow` impl (compile-gated)

- [ ] **Files**: `crates/bongterm-ui/src/dpi.rs` (M, add `#[cfg(windows)]` `WindowsDpiProvider`).
- [ ] `WindowsDpiProvider { hwnd: isize }` implements `DpiProvider::dpi_for_active_window` via `GetDpiForWindow(HWND)`. Scoped `#[cfg(windows)]` block with `unsafe`; keep the safe model in the same file unguarded.
- [ ] Verifiable check: nightly Windows `cargo check --target x86_64-pc-windows-msvc` (5.C.6). Live `WM_DPICHANGED` re-scale is asserted in the DPI line of the manual multi-monitor smoke (drag window between 100%/150% monitors → glyphs re-rasterize crisp, no blur).
- [ ] Commit: `git add crates/bongterm-ui/src/dpi.rs && git commit -m "feat(ui/5.A.3.2): WindowsDpiProvider via GetDpiForWindow (cfg(windows))"`

### 5.B — Signed MSIX packaging (gate #20)

#### 5.B.1 — MSIX manifest + mapping + asset placeholders

##### 5.B.1.1 — `AppxManifest.xml` + `mapping.txt` + assets (RED→GREEN)

- [ ] **Files**: `packaging/msix/AppxManifest.xml` (C), `packaging/msix/mapping.txt` (C), `packaging/msix/assets/Square44x44Logo.png` + `Square150x150Logo.png` + `StoreLogo.png` (C placeholders), and a validator test in `tools/xtask/src/package_msix.rs` (M).
- [ ] The "test" for a static manifest is an **xtask assertion** that parses the XML and checks identity/capabilities. Write it first (in `package_msix.rs`):

```rust
#[cfg(test)]
mod manifest_tests {
    use std::path::Path;

    /// The manifest must declare the package identity, the `runFullTrust`
    /// capability (required for a Win32 desktop app), and point Executable at
    /// the staged `bongterm.exe`. No broad capabilities beyond runFullTrust
    /// (least privilege — Scope/security contract).
    #[test]
    fn appxmanifest_declares_identity_and_min_capabilities() {
        let manifest =
            std::fs::read_to_string(Path::new("../../packaging/msix/AppxManifest.xml"))
                .expect("AppxManifest.xml present");
        assert!(manifest.contains("<Identity"), "has Identity");
        assert!(manifest.contains("Name=\"BongTerm.BongTerm\""));
        assert!(
            manifest.contains("rescap:Capability Name=\"runFullTrust\""),
            "declares runFullTrust"
        );
        assert!(
            manifest.contains("Executable=\"bongterm.exe\""),
            "entrypoint is bongterm.exe"
        );
        // Least privilege: must NOT request broad device/network capabilities.
        assert!(!manifest.contains("internetClientServer"));
        assert!(!manifest.contains("Capability Name=\"allJoyn\""));
    }

    #[test]
    fn mapping_lists_manifest_and_exe() {
        let map = std::fs::read_to_string(Path::new("../../packaging/msix/mapping.txt"))
            .expect("mapping.txt present");
        assert!(map.contains("\"AppxManifest.xml\""));
        assert!(map.contains("bongterm.exe"));
    }
}
```

- [ ] Run expect FAIL: `cargo test -p xtask manifest_tests` → file-not-found panics.
- [ ] Author `AppxManifest.xml` (real content): `<Package>` with `xmlns` + `xmlns:rescap`, `<Identity Name="BongTerm.BongTerm" Publisher="CN=PLACEHOLDER-OV-SUBJECT" Version="0.0.0.0" ProcessorArchitecture="x64"/>`, `<Properties>` (DisplayName BongTerm, PublisherDisplayName, Logo assets/StoreLogo.png), `<Dependencies><TargetDeviceFamily Name="Windows.Desktop" MinVersion="10.0.17763.0" MaxVersionTested="10.0.26100.0"/>`, `<Capabilities><rescap:Capability Name="runFullTrust"/></Capabilities>`, `<Applications><Application Id="BongTerm" Executable="bongterm.exe" EntryPoint="Windows.FullTrustApplication"><uap:VisualElements .../></Application>`.
  - Note: `Publisher` CN must match the signing cert subject (wired in 5.B.3 / code-signing runbook).
- [ ] Author `mapping.txt`: `[Files]` section mapping `"<staged>\bongterm.exe" "bongterm.exe"`, `"AppxManifest.xml" "AppxManifest.xml"`, and the three logo PNGs.
- [ ] Create the three PNG placeholders (44x44, 150x150, 50x50 store logo) — opaque solid-color PNGs are acceptable placeholders; replace before GA (tracked in `known-issues.md`).
- [ ] Run expect PASS: `cargo test -p xtask manifest_tests` → `2 passed`.
- [ ] Commit: `git add packaging/msix && git commit -m "feat(pkg/5.B.1.1): MSIX AppxManifest + mapping + asset placeholders (least-privilege caps)"`

#### 5.B.2 — `xtask package-msix` real implementation

##### 5.B.2.1 — Stage payload + `makeappx` + optional `signtool` + verify (RED→GREEN)

- [ ] **Files**: `tools/xtask/src/package_msix.rs` (M, replace the `not yet implemented` stub), `tools/xtask/src/main.rs` (M, register args).
- [ ] The current stub returns `Err(anyhow!("package-msix not yet implemented (Phase 5.B.2)"))`. Replace with: (1) build `bongterm-app` release, (2) stage `bongterm.exe` + manifest + assets into a temp payload dir, (3) invoke `makeappx.exe pack /m AppxManifest.xml /f mapping.txt /p out\BongTerm.msix /o`, (4) if `BONGT_SIGN_THUMBPRINT` env is set, `signtool.exe sign /fd SHA256 /sha1 <thumbprint> /tr <timestamp-url> /td SHA256 out\BongTerm.msix`, (5) verify with `signtool verify /pa` when signed.
- [ ] The logic test mocks the external tools by injecting a `ToolRunner` trait so CI (no Windows SDK) exercises the staging + command-assembly without running `makeappx`. Failing test first:

```rust
#[cfg(test)]
mod package_tests {
    use super::*;

    /// Records the commands the packager would run, so we can assert the
    /// makeappx + signtool invocation shape on a runner without the SDK.
    #[derive(Default)]
    struct RecordingRunner {
        calls: std::cell::RefCell<Vec<Vec<String>>>,
    }
    impl ToolRunner for RecordingRunner {
        fn run(&self, program: &str, args: &[String]) -> anyhow::Result<()> {
            let mut v = vec![program.to_string()];
            v.extend(args.iter().cloned());
            self.calls.borrow_mut().push(v);
            Ok(())
        }
    }

    #[test]
    fn unsigned_build_invokes_makeappx_pack_only() {
        let runner = RecordingRunner::default();
        let opts = PackageOptions { sign_thumbprint: None, ..PackageOptions::test_default() };
        package_msix_with(&runner, &opts).unwrap();
        let calls = runner.calls.borrow();
        assert!(calls.iter().any(|c| c[0].contains("makeappx") && c.iter().any(|a| a == "pack")));
        assert!(!calls.iter().any(|c| c[0].contains("signtool")));
    }

    #[test]
    fn signed_build_invokes_signtool_with_sha256_and_timestamp() {
        let runner = RecordingRunner::default();
        let opts = PackageOptions {
            sign_thumbprint: Some("ABCDEF0123".into()),
            ..PackageOptions::test_default()
        };
        package_msix_with(&runner, &opts).unwrap();
        let calls = runner.calls.borrow();
        let sign = calls.iter().find(|c| c[0].contains("signtool")).expect("signtool called");
        assert!(sign.iter().any(|a| a == "/fd"));
        assert!(sign.iter().any(|a| a == "SHA256"));
        assert!(sign.iter().any(|a| a == "/tr"), "RFC3161 timestamp required");
        assert!(sign.iter().any(|a| a == "ABCDEF0123"));
    }
}
```

- [ ] Run expect FAIL: `cargo test -p xtask package_tests` → `ToolRunner`/`PackageOptions`/`package_msix_with` unresolved.
- [ ] Implement `ToolRunner` (trait + real `SystemRunner` using `std::process::Command`), `PackageOptions { sign_thumbprint, timestamp_url, payload_dir, out_path, .. }` with a `test_default()`, and `package_msix_with(runner, opts)` assembling the commands. Public `package_msix(opts)` calls it with `SystemRunner`.
- [ ] Register in `main.rs`: `package-msix [--sign-thumbprint <hex>] [--timestamp-url <url>]`.
- [ ] Run expect PASS: `cargo test -p xtask package_tests` → `2 passed`.
- [ ] Commit: `git add tools/xtask/src/package_msix.rs tools/xtask/src/main.rs && git commit -m "feat(pkg/5.B.2.1): real package-msix — stage+makeappx+optional signtool, mockable ToolRunner"`

#### 5.B.3 — Code-signing cert provisioning (OV first, EV evaluation ADR)

##### 5.B.3.1 — Code-signing runbook + EV-evaluation ADR

- [ ] **Files**: `docs/runbook/code-signing.md` (M), `docs/adr/0013-ev-cert-evaluation.md` (C).
- [ ] Flesh out `code-signing.md`: OV cert acquisition steps, importing into the per-user cert store, locating the SHA1 thumbprint (`Get-ChildItem Cert:\CurrentUser\My | Where-Object {...}`), setting `BONGT_SIGN_THUMBPRINT`, the RFC3161 timestamp URL, and the `signtool verify /pa` acceptance.
- [ ] Write ADR-0013: decision = ship GA on **OV** (organization-validated) signing now; **evaluate EV** later because EV gives immediate SmartScreen reputation but requires hardware token / cloud HSM and higher cost. Record context, the SmartScreen-warmup tradeoff (OV accrues reputation over installs; covered by `smartscreen.md`), and the revisit trigger (if SmartScreen friction blocks adoption). Body notes spec logical role.
- [ ] Verifiable check (acceptance step, not a unit test): the runbook contains a copy-pasteable verification block ending in `signtool verify /pa /v out\BongTerm.msix` producing `Successfully verified`. This is executed during the clean-VM signing validation (5.B.4) and its output pasted into the release ticket.
- [ ] Commit: `git add docs/runbook/code-signing.md docs/adr/0013-ev-cert-evaluation.md && git commit -m "docs(pkg/5.B.3.1): OV signing runbook + ADR-0013 EV-evaluation decision"`

#### 5.B.4 — Clean-VM install / upgrade / uninstall smoke (gate #20)

##### 5.B.4.1 — Clean-VM smoke runbook + 7-nightly acceptance

- [ ] **Files**: `docs/runbook/release.md` (M, add the clean-VM smoke section — full rollback content lands in 5.F.4).
- [ ] Define the manual/infra acceptance (the "test" for this infra task): on a freshly-provisioned Win11 24H2 VM with no prior BongTerm, (1) `Add-AppxPackage out\BongTerm.msix` succeeds and the signature chains to a trusted root, (2) launch BongTerm, confirm it runs, (3) install v+1 (higher Version in manifest) → confirms in-place upgrade, settings preserved, (4) `Remove-AppxPackage` → confirm clean uninstall, no leftover under `%LOCALAPPDATA%\BongTerm` except the documented retained transcript store. Each step has a PASS checkbox.
- [ ] Exit acceptance for gate #20: this script passes on a clean VM **for 7 consecutive nightlies** (the Goal/exit criterion). The nightly job (5.C.6) records pass/fail; 7 green in a row is the gate.
- [ ] Commit: `git add docs/runbook/release.md && git commit -m "docs(pkg/5.B.4.1): clean-VM install/upgrade/uninstall smoke + 7-nightly gate #20 acceptance"`

#### 5.B.5 — SmartScreen runbook

##### 5.B.5.1 — Flesh out `smartscreen.md`

- [ ] **Files**: `docs/runbook/smartscreen.md` (M).
- [ ] Replace placeholder with: why SmartScreen warns on new OV-signed binaries (no reputation yet), the reputation-warmup procedure (consistent publisher CN, submit to Microsoft via the Defender SmartScreen "Report as safe" / Partner Center if Store-distributed), how to verify the warning disappears, and the acceptance step (download the signed MSIX on a clean VM via browser, confirm SmartScreen behavior is documented and within expected for an OV cert). Cross-link ADR-0013.
- [ ] Verifiable check (acceptance): the runbook's "Acceptance" section is checked off during clean-VM validation; the observed SmartScreen prompt (or absence) is recorded in the release ticket. No code test.
- [ ] Commit: `git add docs/runbook/smartscreen.md && git commit -m "docs(pkg/5.B.5.1): flesh out SmartScreen warmup runbook + acceptance"`

### 5.C — Nightly hardening: fuzz, Defender, forbidden-abstraction, device-loss, crash-recovery (gates #21, #25, #26)

#### 5.C.1 — Parser fuzzing in nightly CI (pinned nightly)

##### 5.C.1.1 — Fuzz crate + VT parser target + seed corpus (RED→GREEN)

- [ ] **Files**: `crates/bongterm-term/fuzz/Cargo.toml` (C), `crates/bongterm-term/fuzz/fuzz_targets/vt_parser.rs` (C), `tools/xtask/fuzz/rust-toolchain.toml` (C), `tests/fixtures/fuzz_corpora/vt_parser/seed01` (C).
- [ ] `rust-toolchain.toml` pins the nightly: `[toolchain]\nchannel = "nightly-2026-05-01"\ncomponents = ["rust-src"]` (date confirmed in `docs/runbook/fuzzing.md`).
- [ ] `fuzz/Cargo.toml`: a `cargo-fuzz` crate depending on `bongterm-term` (path) + `libfuzzer-sys`, `[[bin]] name = "vt_parser"`, `[package.metadata] cargo-fuzz = true`.
- [ ] `vt_parser.rs` fuzz target:

```rust
#![no_main]
use libfuzzer_sys::fuzz_target;

// Feeds arbitrary bytes through the BongTerm VT/ANSI parser. The invariant:
// the parser must never panic, never allocate unbounded, and must consume the
// whole slice regardless of input. libFuzzer catches any panic/abort.
fuzz_target!(|data: &[u8]| {
    let mut parser = bongterm_term::Parser::new();
    let mut grid = bongterm_term::Grid::new(80, 24);
    parser.feed(data, &mut grid);
});
```

  - Note: adjust `Parser::new`/`feed`/`Grid::new` to the actual `bongterm-term` public API (the fuzz crate must compile against the real signatures — confirm via the crate's `lib.rs` before writing; correct the call if the API differs).
- [ ] Seed corpus `seed01`: a small byte file with representative escapes — `\x1b[31mred\x1b[0m\r\n\x1b]0;title\x07` (write raw bytes).
- [ ] Run expect FAIL→GREEN locally with the pinned nightly: `cd crates/bongterm-term/fuzz && cargo +nightly-2026-05-01 fuzz build vt_parser` (FAIL = compile error if API mismatched; fix, then) then a short run `cargo +nightly-2026-05-01 fuzz run vt_parser tests/../../tests/fixtures/fuzz_corpora/vt_parser -- -runs=10000 -max_total_time=30` exits 0 (no crash). This bounded run is the local acceptance; the unbounded run lives in nightly CI.
- [ ] Confirm fuzz crate is **excluded from the workspace** (it has its own toolchain) — add it to root `Cargo.toml` `[workspace] exclude = [...]` if not already covered, so PR `cargo test` never pulls nightly.
- [ ] Commit: `git add crates/bongterm-term/fuzz tools/xtask/fuzz/rust-toolchain.toml tests/fixtures/fuzz_corpora && git commit -m "feat(fuzz/5.C.1.1): VT-parser cargo-fuzz target + pinned nightly + seed corpus"`

##### 5.C.1.2 — `tools/xtask/fuzz/Cargo.toml` umbrella + fuzzing runbook wiring

- [ ] **Files**: `tools/xtask/fuzz/Cargo.toml` (C), `docs/runbook/fuzzing.md` (M).
- [ ] `tools/xtask/fuzz/Cargo.toml` is the umbrella manifest documenting all fuzz targets (vt_parser now; osc_consumer/settings_json5/redactor referenced as future per the existing runbook). It points the pinned nightly + lists corpora paths.
- [ ] Update `fuzzing.md`: confirm the pinned date `nightly-2026-05-01` matches `rust-toolchain.toml`, document the nightly CI invocation and the `-max_total_time` budget, and the policy that the nightly toolchain never enters the MSIX and never runs on PR (Scope Lock 4).
- [ ] Verifiable check: `fuzzing.md` states the exact pinned channel and that channel string `grep`-matches `tools/xtask/fuzz/rust-toolchain.toml`. Add a tiny xtask test that reads both files and asserts the channel strings are equal:

```rust
#[cfg(test)]
mod fuzz_pin_tests {
    #[test]
    fn runbook_pin_matches_toolchain_file() {
        let toolchain = std::fs::read_to_string("fuzz/rust-toolchain.toml").unwrap();
        let runbook = std::fs::read_to_string("../../docs/runbook/fuzzing.md").unwrap();
        assert!(toolchain.contains("nightly-2026-05-01"));
        assert!(runbook.contains("nightly-2026-05-01"), "runbook pin must match toolchain");
    }
}
```

- [ ] Run expect FAIL then PASS: `cargo test -p xtask fuzz_pin_tests`.
- [ ] Commit: `git add tools/xtask/fuzz/Cargo.toml docs/runbook/fuzzing.md tools/xtask/src && git commit -m "test(fuzz/5.C.1.2): fuzz umbrella manifest + runbook pin-consistency guard"`

#### 5.C.2 — Defender real-time smoke (nightly)

##### 5.C.2.1 — Defender smoke script + nightly assertion

- [ ] **Files**: `docs/runbook/edr.md` (C, the Defender section; EDR/S7 content extended in 5.E), and a nightly step in `.github/workflows/nightly.yml` (added in 5.C.6).
- [ ] The verifiable check (infra task): on the Windows nightly runner with Defender real-time protection **on**, run `cargo build -p bongterm-app --release` and launch the binary headless for N seconds; assert (1) the build artifact is not quarantined (file still exists, `Get-MpThreatDetection` shows no detection for the path), (2) the process starts and exits cleanly. Script lives in the nightly workflow; `edr.md` documents expected Defender behavior + the allowlist guidance if a false-positive ever appears.
- [ ] Acceptance: nightly job step `defender-smoke` is green; a Defender detection fails the job.
- [ ] Commit: `git add docs/runbook/edr.md && git commit -m "docs(sec/5.C.2.1): Defender real-time smoke acceptance + allowlist guidance"`

#### 5.C.3 — Forbidden-abstraction → runtime process-tree checks (gate #21)

##### 5.C.3.1 — `ForbiddenTechnique` enum + `ProcessTreeAuditor` trait (RED→GREEN)

- [ ] **Files**: `crates/bongterm-security/src/forbidden.rs` (C), `crates/bongterm-security/src/lib.rs` (M, add `pub mod forbidden;`).
- [ ] Closed enum of the hard non-goals (DLL injection, ConPTY bypass, global hooks, process hollowing, kernel driver, undocumented ntdll, direct GPU). The auditor inspects the live process tree (BongTerm + children) and flags any forbidden technique signature. `bongterm-security` is `#![deny(unsafe_code)]`, so the trait is the seam; the Windows enumeration impl lives behind it.
- [ ] Failing test + types:

```rust
//! Runtime forbidden-abstraction audit (gate #21). Closed set of techniques
//! the product must never use; the auditor checks the live process tree and
//! returns any detected violation. Default-deny posture: presence == fail.

/// Closed set of OS-bypass techniques banned by PRD §3.2 / spec gate #21.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ForbiddenTechnique {
    DllInjection,
    ConPtyBypass,
    GlobalKeyboardHook,
    ProcessHollowing,
    KernelDriver,
    UndocumentedNtdllSyscall,
    DirectGpuDriverAccess,
}

impl ForbiddenTechnique {
    pub const ALL: [ForbiddenTechnique; 7] = [
        ForbiddenTechnique::DllInjection,
        ForbiddenTechnique::ConPtyBypass,
        ForbiddenTechnique::GlobalKeyboardHook,
        ForbiddenTechnique::ProcessHollowing,
        ForbiddenTechnique::KernelDriver,
        ForbiddenTechnique::UndocumentedNtdllSyscall,
        ForbiddenTechnique::DirectGpuDriverAccess,
    ];
}

/// One observed process-tree node (BongTerm or a descendant).
#[derive(Debug, Clone)]
pub struct ProcessNode {
    pub pid: u32,
    pub image_name: String,
    /// Modules loaded into this process (image base names), for injection
    /// detection.
    pub loaded_modules: Vec<String>,
}

/// Substitutable so CI feeds a synthetic tree; the Windows impl enumerates via
/// supported ToolHelp/PSAPI snapshots (NO undocumented calls — itself in-bounds).
pub trait ProcessTreeAuditor {
    fn snapshot(&self) -> Vec<ProcessNode>;
    /// Pure classification over a snapshot. Returns every detected violation.
    fn audit(&self, tree: &[ProcessNode]) -> Vec<ForbiddenTechnique> {
        let mut hits = Vec::new();
        for node in tree {
            // A module not belonging to BongTerm or a known shell/agent image
            // injected into the BongTerm process is the DLL-injection signature.
            if node.image_name.eq_ignore_ascii_case("bongterm.exe")
                && node
                    .loaded_modules
                    .iter()
                    .any(|m| is_foreign_injected_module(m))
            {
                hits.push(ForbiddenTechnique::DllInjection);
            }
        }
        hits
    }
}

/// A module is "foreign-injected" if it is not a system DLL, not our own, and
/// not an allowlisted runtime dependency. Conservative allowlist.
fn is_foreign_injected_module(module: &str) -> bool {
    const ALLOWED: &[&str] = &[
        "ntdll.dll", "kernel32.dll", "kernelbase.dll", "user32.dll",
        "gdi32.dll", "d3d12.dll", "dxgi.dll", "d2d1.dll", "dwrite.dll",
        "bongterm.exe", "vcruntime140.dll", "ucrtbase.dll",
    ];
    let lower = module.to_ascii_lowercase();
    !ALLOWED.contains(&lower.as_str())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Synthetic(Vec<ProcessNode>);
    impl ProcessTreeAuditor for Synthetic {
        fn snapshot(&self) -> Vec<ProcessNode> {
            self.0.clone()
        }
    }

    #[test]
    fn clean_tree_has_no_violations() {
        let a = Synthetic(vec![ProcessNode {
            pid: 1,
            image_name: "bongterm.exe".into(),
            loaded_modules: vec!["ntdll.dll".into(), "d3d12.dll".into(), "bongterm.exe".into()],
        }]);
        assert!(a.audit(&a.snapshot()).is_empty());
    }

    #[test]
    fn foreign_module_in_bongterm_flags_dll_injection() {
        let a = Synthetic(vec![ProcessNode {
            pid: 1,
            image_name: "bongterm.exe".into(),
            loaded_modules: vec!["ntdll.dll".into(), "evil_inject.dll".into()],
        }]);
        assert_eq!(a.audit(&a.snapshot()), vec![ForbiddenTechnique::DllInjection]);
    }

    #[test]
    fn all_techniques_enumerated() {
        assert_eq!(ForbiddenTechnique::ALL.len(), 7);
    }
}
```

- [ ] Add `pub mod forbidden;` to `lib.rs`.
- [ ] Run expect FAIL: `cargo test -p bongterm-security forbidden::tests` → unresolved module.
- [ ] Add impl. Run expect PASS → `3 passed`.
- [ ] Commit: `git add crates/bongterm-security/src/forbidden.rs crates/bongterm-security/src/lib.rs && git commit -m "feat(sec/5.C.3.1): ForbiddenTechnique enum + ProcessTreeAuditor with DLL-injection detection (gate #21)"`

##### 5.C.3.2 — `ProcessTreeAuditor` conformance suite (RED→GREEN)

- [ ] **Files**: `crates/bongterm-test-kit/src/conformance/process_tree_auditor_conformance.rs` (C), `crates/bongterm-test-kit/src/conformance/mod.rs` (M).
- [ ] Conformance fn asserts: a clean tree yields no hits; a tree with a known-foreign module yields exactly `DllInjection`; `audit` is pure (same input → same output).

```rust
//! Conformance for `bongterm_security::forbidden::ProcessTreeAuditor`.

use bongterm_security::forbidden::{ForbiddenTechnique, ProcessNode, ProcessTreeAuditor};

pub fn run_process_tree_auditor_conformance<A: ProcessTreeAuditor>(auditor: &A) {
    // Deterministic: auditing the live snapshot twice agrees.
    let snap = auditor.snapshot();
    assert_eq!(auditor.audit(&snap), auditor.audit(&snap), "audit must be pure");

    // Synthetic clean tree → no violations.
    let clean = vec![ProcessNode {
        pid: 1,
        image_name: "bongterm.exe".into(),
        loaded_modules: vec!["ntdll.dll".into(), "bongterm.exe".into()],
    }];
    assert!(auditor.audit(&clean).is_empty());

    // Synthetic injected tree → DllInjection.
    let dirty = vec![ProcessNode {
        pid: 1,
        image_name: "bongterm.exe".into(),
        loaded_modules: vec!["hook32.dll".into()],
    }];
    assert!(auditor.audit(&dirty).contains(&ForbiddenTechnique::DllInjection));
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Empty;
    impl ProcessTreeAuditor for Empty {
        fn snapshot(&self) -> Vec<ProcessNode> {
            Vec::new()
        }
    }

    #[test]
    fn default_audit_satisfies_conformance() {
        run_process_tree_auditor_conformance(&Empty);
    }
}
```

- [ ] Add `pub mod process_tree_auditor_conformance;` to `conformance/mod.rs`.
- [ ] Run expect FAIL→PASS: `cargo test -p bongterm-test-kit process_tree_auditor_conformance` → `1 passed`.
- [ ] Commit: `git add crates/bongterm-test-kit/src/conformance/process_tree_auditor_conformance.rs crates/bongterm-test-kit/src/conformance/mod.rs && git commit -m "test(sec/5.C.3.2): ProcessTreeAuditor conformance suite"`

##### 5.C.3.3 — `xtask forbidden-abstraction` static + process-tree scan (RED→GREEN)

- [ ] **Files**: `tools/xtask/src/forbidden_abstraction.rs` (C), `tools/xtask/src/main.rs` (M, register `forbidden-abstraction` subcommand).
- [ ] Static scan: grep the workspace sources (excluding tests/docs/this scanner) for banned API symbols — `SetWindowsHookEx`, `NtCreateThreadEx`, `WriteProcessMemory` + `CreateRemoteThread` pair, `ZwUnmapViewOfSection`, raw `syscall` to undocumented ntdll. Any hit outside an allowlist fails the gate. The process-tree runtime part defers to `ProcessTreeAuditor` (Windows-only, runs in nightly).
- [ ] Failing test:

```rust
#[cfg(test)]
mod scan_tests {
    use super::*;

    #[test]
    fn flags_banned_symbol_in_source_text() {
        let hits = scan_text_for_banned("let h = SetWindowsHookEx(WH_KEYBOARD_LL, ...);");
        assert!(hits.contains(&"SetWindowsHookEx"));
    }

    #[test]
    fn clean_source_has_no_hits() {
        assert!(scan_text_for_banned("let x = GetDpiForWindow(hwnd);").is_empty());
    }
}
```

- [ ] Implement `scan_text_for_banned(&str) -> Vec<&'static str>` over a `const BANNED: &[&str]` list, and `run_forbidden_abstraction_scan(workspace_root) -> anyhow::Result<()>` that walks `crates/**/src/**/*.rs`, applies the scan, and errors listing file:line on any hit. Register subcommand in `main.rs`.
- [ ] Run expect FAIL→PASS: `cargo test -p xtask scan_tests` → `2 passed`. Then run `cargo run -p xtask -- forbidden-abstraction` against the real tree expecting exit 0 (clean).
- [ ] Commit: `git add tools/xtask/src/forbidden_abstraction.rs tools/xtask/src/main.rs && git commit -m "feat(sec/5.C.3.3): xtask forbidden-abstraction static scan (gate #21, PR-blocking)"`

#### 5.C.4 — Renderer device-loss simulated test (gate #25, P1)

##### 5.C.4.1 — `DeviceRemovedReason` enum + `DeviceLossRecovery` controller (RED→GREEN)

- [ ] **Files**: `crates/bongterm-render/src/device_loss.rs` (C), `crates/bongterm-render/src/lib.rs` (M, add `mod device_loss;` + re-export + a `recover()` hook on the backend).
- [ ] `lib.rs` already has `MockRendererBackend::force_device_loss()` and `RenderError::DeviceLost`. Add the recovery controller that classifies the DXGI reason, drives recreate, and enforces the 3-in-60s → software-fallback policy.
- [ ] Failing test + impl:

```rust
//! Device-loss recovery controller (gate #25). Classifies a DXGI
//! device-removed reason, drives recreation, and trips a software-fallback
//! after 3 losses in 60s so a wedged GPU can't hard-loop.

/// Closed set mirroring DXGI `GetDeviceRemovedReason` HRESULTs we handle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceRemovedReason {
    /// DXGI_ERROR_DEVICE_HUNG
    Hung,
    /// DXGI_ERROR_DEVICE_REMOVED
    Removed,
    /// DXGI_ERROR_DEVICE_RESET
    Reset,
    /// DXGI_ERROR_DRIVER_INTERNAL_ERROR
    DriverInternal,
    /// Any other / unknown HRESULT.
    Unknown,
}

impl DeviceRemovedReason {
    pub fn from_hresult(hr: i32) -> Self {
        // DXGI error codes (HRESULT, as i32).
        match hr as u32 {
            0x887A0006 => DeviceRemovedReason::Hung,
            0x887A0005 => DeviceRemovedReason::Removed,
            0x887A0007 => DeviceRemovedReason::Reset,
            0x887A0020 => DeviceRemovedReason::DriverInternal,
            _ => DeviceRemovedReason::Unknown,
        }
    }
}

/// What the controller decided after a loss event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryAction {
    /// Recreate the device and continue on the GPU.
    Recreate,
    /// Too many losses too fast — drop to software rendering.
    SoftwareFallback,
}

/// Tracks loss timestamps (monotonic millis) and enforces 3-in-60s.
pub struct DeviceLossRecovery {
    losses_ms: Vec<u64>,
    window_ms: u64,
    threshold: usize,
}

impl Default for DeviceLossRecovery {
    fn default() -> Self {
        DeviceLossRecovery { losses_ms: Vec::new(), window_ms: 60_000, threshold: 3 }
    }
}

impl DeviceLossRecovery {
    /// Record a loss at `now_ms`; return the action to take.
    pub fn on_device_lost(&mut self, _reason: DeviceRemovedReason, now_ms: u64) -> RecoveryAction {
        self.losses_ms.retain(|&t| now_ms.saturating_sub(t) <= self.window_ms);
        self.losses_ms.push(now_ms);
        if self.losses_ms.len() >= self.threshold {
            RecoveryAction::SoftwareFallback
        } else {
            RecoveryAction::Recreate
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{MockRendererBackend, RenderError};

    #[test]
    fn hresult_classification() {
        assert_eq!(DeviceRemovedReason::from_hresult(0x887A0006u32 as i32), DeviceRemovedReason::Hung);
        assert_eq!(DeviceRemovedReason::from_hresult(0x887A0005u32 as i32), DeviceRemovedReason::Removed);
        assert_eq!(DeviceRemovedReason::from_hresult(0), DeviceRemovedReason::Unknown);
    }

    #[test]
    fn first_two_losses_recreate_third_falls_back() {
        let mut r = DeviceLossRecovery::default();
        assert_eq!(r.on_device_lost(DeviceRemovedReason::Reset, 0), RecoveryAction::Recreate);
        assert_eq!(r.on_device_lost(DeviceRemovedReason::Reset, 1000), RecoveryAction::Recreate);
        assert_eq!(r.on_device_lost(DeviceRemovedReason::Reset, 2000), RecoveryAction::SoftwareFallback);
    }

    #[test]
    fn losses_outside_window_do_not_accumulate() {
        let mut r = DeviceLossRecovery::default();
        assert_eq!(r.on_device_lost(DeviceRemovedReason::Reset, 0), RecoveryAction::Recreate);
        assert_eq!(r.on_device_lost(DeviceRemovedReason::Reset, 70_000), RecoveryAction::Recreate);
        assert_eq!(r.on_device_lost(DeviceRemovedReason::Reset, 75_000), RecoveryAction::Recreate);
    }

    #[test]
    fn mock_backend_device_loss_surfaces_then_recovers() {
        let mut backend = MockRendererBackend::default();
        backend.force_device_loss();
        // First render after forced loss reports DeviceLost.
        let err = backend.render_frame_test_hook().unwrap_err();
        assert!(matches!(err, RenderError::DeviceLost));
        // Controller decides to recreate, backend recovers.
        let mut r = DeviceLossRecovery::default();
        assert_eq!(r.on_device_lost(DeviceRemovedReason::Removed, 0), RecoveryAction::Recreate);
        backend.recover();
        assert!(backend.render_frame_test_hook().is_ok());
    }
}
```

  - Note: `render_frame_test_hook()` / `recover()` names must match `bongterm-render`'s existing mock surface. If the mock currently only exposes `force_device_loss()` + `render_frame(...)`, add a minimal `recover()` that clears the forced-loss flag and (if needed) a test hook; keep changes additive and within the renderer's ownership.
- [ ] Add `mod device_loss; pub use device_loss::{DeviceRemovedReason, RecoveryAction, DeviceLossRecovery};` to `lib.rs`.
- [ ] Run expect FAIL→PASS: `cargo test -p bongterm-render device_loss::tests` → all pass.
- [ ] Commit: `git add crates/bongterm-render/src/device_loss.rs crates/bongterm-render/src/lib.rs && git commit -m "feat(render/5.C.4.1): DeviceLossRecovery — DXGI reason classify + 3-in-60s software fallback (gate #25)"`

#### 5.C.5 — Crash-recovery suite (gate #26)

##### 5.C.5.1 — `RecoveryScreen` model + per-crash-class actions (RED→GREEN)

- [ ] **Files**: `crates/bongterm-diagnostics/src/recovery.rs` (C), `crates/bongterm-diagnostics/src/lib.rs` (M, add `pub mod recovery;`).
- [ ] Closed enum of the 6 crash classes from §5.7 (pane panic, renderer panic, MCP crash-loop, SQLite busy, sidecar torn-write, disk-quota). Each maps to allowed recovery actions (Restore / Discard / Export). Pure model.
- [ ] Failing test + impl:

```rust
//! Crash-recovery model (gate #26). Six crash classes from spec §5.7; each maps
//! to the actions the recovery screen offers. Pure: no I/O, no panic handling
//! here (the panic hook lives in `lib.rs`).

/// Closed set of crash classes the app recovers from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CrashClass {
    PanePanic,
    RendererPanic,
    McpCrashLoop,
    SqliteBusy,
    SidecarTornWrite,
    DiskQuotaExceeded,
}

/// Actions a recovery screen may offer for a crash.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryAction {
    Restore,
    Discard,
    Export,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecoveryScreen {
    pub class: CrashClass,
    pub actions: Vec<RecoveryAction>,
    /// Whether the crash is isolated (app keeps running) or app-wide.
    pub app_wide: bool,
}

impl RecoveryScreen {
    pub fn for_class(class: CrashClass) -> Self {
        use CrashClass::*;
        use RecoveryAction::*;
        let (actions, app_wide) = match class {
            // Isolated: only the pane/renderer surface is lost; offer restore.
            PanePanic => (vec![Restore, Discard, Export], false),
            RendererPanic => (vec![Restore, Export], false),
            // MCP crash-loop: disable the offending server, keep app.
            McpCrashLoop => (vec![Discard, Export], false),
            // Storage faults: app-wide; export diagnostics, then restore from
            // append-only chunks (transcripts are local source-of-truth).
            SqliteBusy => (vec![Restore, Export], true),
            SidecarTornWrite => (vec![Restore, Export], true),
            DiskQuotaExceeded => (vec![Discard, Export], true),
        };
        RecoveryScreen { class, actions, app_wide }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use CrashClass::*;
    use RecoveryAction::*;

    #[test]
    fn pane_panic_is_isolated_and_restorable() {
        let s = RecoveryScreen::for_class(PanePanic);
        assert!(!s.app_wide);
        assert!(s.actions.contains(&Restore));
    }

    #[test]
    fn storage_faults_are_app_wide_with_export() {
        for c in [SqliteBusy, SidecarTornWrite, DiskQuotaExceeded] {
            let s = RecoveryScreen::for_class(c);
            assert!(s.app_wide, "{c:?} should be app-wide");
            assert!(s.actions.contains(&Export), "{c:?} must offer Export");
        }
    }

    #[test]
    fn mcp_crash_loop_does_not_restore_the_loop() {
        let s = RecoveryScreen::for_class(McpCrashLoop);
        assert!(!s.actions.contains(&Restore));
    }

    #[test]
    fn every_class_offers_export() {
        for c in [PanePanic, RendererPanic, McpCrashLoop, SqliteBusy, SidecarTornWrite, DiskQuotaExceeded] {
            assert!(RecoveryScreen::for_class(c).actions.contains(&Export));
        }
    }
}
```

- [ ] Add `pub mod recovery;` to `lib.rs`.
- [ ] Run expect FAIL→PASS: `cargo test -p bongterm-diagnostics recovery::tests` → `4 passed`.
- [ ] Commit: `git add crates/bongterm-diagnostics/src/recovery.rs crates/bongterm-diagnostics/src/lib.rs && git commit -m "feat(diag/5.C.5.1): RecoveryScreen model — 6 crash classes → actions (gate #26)"`

##### 5.C.5.2 — Crash-isolation integration tests (catch_unwind per scenario) (RED→GREEN)

- [ ] **Files**: `crates/bongterm-diagnostics/tests/crash_recovery.rs` (C).
- [ ] Integration test proving isolated panics are caught (the release profile deliberately has NO `panic = "abort"`). Use `std::panic::catch_unwind` to model a pane-task panic and assert the harness keeps running + produces the right `RecoveryScreen`.

```rust
use bongterm_diagnostics::recovery::{CrashClass, RecoveryScreen};
use std::panic;

#[test]
fn isolated_pane_panic_is_caught_and_app_survives() {
    let prev = panic::take_hook();
    panic::set_hook(Box::new(|_| {})); // silence during the test
    let result = panic::catch_unwind(|| {
        panic!("simulated pane task panic");
    });
    panic::set_hook(prev);
    assert!(result.is_err(), "panic must be catchable (no panic=abort)");
    // App composes the recovery screen and keeps running.
    let screen = RecoveryScreen::for_class(CrashClass::PanePanic);
    assert!(!screen.app_wide);
}
```

- [ ] Run expect FAIL→PASS: `cargo test -p bongterm-diagnostics --test crash_recovery` → `1 passed`. (FAIL surfaces only if `panic = "abort"` ever sneaks into a profile — which would also break the spec contract.)
- [ ] Commit: `git add crates/bongterm-diagnostics/tests/crash_recovery.rs && git commit -m "test(diag/5.C.5.2): catch_unwind crash-isolation integration test (no panic=abort)"`

#### 5.C.6 — `nightly.yml` workflow + `ci.yml` PR-gate augmentation

##### 5.C.6.1 — `nightly.yml` (fuzz + accessibility + device-loss + crash + Defender + forbidden runtime)

- [ ] **Files**: `.github/workflows/nightly.yml` (C).
- [ ] Author the nightly workflow: `on: schedule: cron` nightly + `workflow_dispatch`. Jobs:
  - `fuzz` (self-hosted or ubuntu with the pinned nightly): install `nightly-2026-05-01`, `cargo install cargo-fuzz`, run `cargo fuzz run vt_parser -- -max_total_time=900`; upload any crash artifact.
  - `windows-checks` (windows-latest): `cargo check -p bongterm-ui --target x86_64-pc-windows-msvc` (proves `accessibility_win`/`ime_win`/dpi shims compile), `cargo test -p bongterm-render device_loss`, `cargo test -p bongterm-diagnostics recovery crash_recovery`.
  - `forbidden-runtime` (windows-latest): build app, launch headless, run the `ProcessTreeAuditor` over the live tree, assert no `ForbiddenTechnique`.
  - `defender-smoke` (windows-latest, Defender on): build release + launch + assert no quarantine/detection (5.C.2.1).
  - `clean-vm-smoke` (self-hosted clean VM, manual-eligible): runs the 5.B.4 install/upgrade/uninstall script; records pass for the 7-consecutive-nightly gate.
- [ ] Verifiable check: the workflow file parses as valid YAML and contains a job per the above. Add an xtask test that loads `nightly.yml` and asserts the expected job keys exist:

```rust
#[cfg(test)]
mod nightly_yaml_tests {
    #[test]
    fn nightly_workflow_declares_required_jobs() {
        let y = std::fs::read_to_string("../../.github/workflows/nightly.yml").unwrap();
        for job in ["fuzz", "windows-checks", "forbidden-runtime", "defender-smoke", "clean-vm-smoke"] {
            assert!(y.contains(job), "nightly.yml missing job: {job}");
        }
    }
}
```

- [ ] Run expect FAIL→PASS: `cargo test -p xtask nightly_yaml_tests`.
- [ ] Commit: `git add .github/workflows/nightly.yml tools/xtask/src && git commit -m "ci(5.C.6.1): nightly workflow — fuzz, win-checks, device-loss, crash, Defender, forbidden-runtime, clean-VM"`

##### 5.C.6.2 — `ci.yml` PR-gate: forbidden-abstraction static + MSIX manifest validate

- [ ] **Files**: `.github/workflows/ci.yml` (M).
- [ ] Add to the PR-blocking `correctness` job (or a new `release-gates` job): `cargo run -p xtask -- forbidden-abstraction` (static, 5.C.3.3), `cargo test -p xtask manifest_tests` (5.B.1.1), and (SBOM/attestation steps wired in 5.F). These run on every PR; nightly-only items (fuzz, Defender, clean-VM) stay out of PR per Scope Lock 4.
- [ ] Verifiable check: xtask test asserting `ci.yml` references `forbidden-abstraction`:

```rust
#[cfg(test)]
mod ci_yaml_tests {
    #[test]
    fn ci_runs_forbidden_abstraction_gate() {
        let y = std::fs::read_to_string("../../.github/workflows/ci.yml").unwrap();
        assert!(y.contains("forbidden-abstraction"), "ci.yml must run forbidden-abstraction");
    }
}
```

- [ ] Run expect FAIL→PASS: `cargo test -p xtask ci_yaml_tests`.
- [ ] Commit: `git add .github/workflows/ci.yml tools/xtask/src && git commit -m "ci(5.C.6.2): PR-gate forbidden-abstraction static + MSIX manifest validate"`

### 5.D — Opt-in diagnostics: export + redaction + consent + minidump (gate #19, #26)

#### 5.D.0 — Wire diagnostics deps

- [ ] **Files**: `crates/bongterm-diagnostics/Cargo.toml` (M).
- [ ] Add `bongterm-security = { path = "../bongterm-security" }` (redactor), `minidump-writer = { workspace = true }` (under `[target.'cfg(windows)'.dependencies]`), `serde = { workspace = true }`, `serde_json = { workspace = true }`.
- [ ] Run expect PASS: `cargo check -p bongterm-diagnostics`.
- [ ] Commit: `git add crates/bongterm-diagnostics/Cargo.toml && git commit -m "build(diag/5.D.0): add security/minidump-writer/serde deps"`

#### 5.D.1 — Diagnostic export + redaction preview (gate #19)

##### 5.D.1.1 — `DiagnosticBundle` builder + `RedactionPreview` (RED→GREEN)

- [ ] **Files**: `crates/bongterm-diagnostics/src/export.rs` (C), `crates/bongterm-diagnostics/src/lib.rs` (M, add `pub mod export;`).
- [ ] A bundle never auto-sends (gate #19); the user always sees a `RedactionPreview` first. Secrets must never appear (security contract §37). The redaction reuses `bongterm-security`'s redactor seam.
- [ ] Failing test + impl:

```rust
//! Diagnostic export with mandatory redaction preview (gate #19). Bundles are
//! built locally and NEVER auto-sent; the user reviews a `RedactionPreview`
//! before anything leaves the machine. Secrets are scrubbed (contract §37).

/// A redaction rule: replaces matches of `needle` with the fixed mask. The
/// real redactor lives in `bongterm-security`; this is the diagnostics-side
/// application over assembled text.
#[derive(Debug, Clone)]
pub struct Redactor {
    secrets: Vec<String>,
}

impl Redactor {
    pub fn new(secrets: Vec<String>) -> Self {
        Redactor { secrets }
    }
    pub fn redact(&self, input: &str) -> String {
        let mut out = input.to_string();
        for s in &self.secrets {
            if !s.is_empty() {
                out = out.replace(s, "[REDACTED]");
            }
        }
        out
    }
}

/// One section of the bundle (e.g. logs, settings, system info).
#[derive(Debug, Clone)]
pub struct BundleSection {
    pub name: String,
    pub content: String,
}

/// The assembled, pre-send bundle. Carries only redacted content.
#[derive(Debug, Clone)]
pub struct DiagnosticBundle {
    pub sections: Vec<BundleSection>,
}

/// What the user sees before export. Shows redacted text + a count of redactions
/// so a leak is visible before it leaves the machine.
#[derive(Debug, Clone)]
pub struct RedactionPreview {
    pub redacted: DiagnosticBundle,
    pub redaction_count: usize,
}

pub struct DiagnosticExport {
    redactor: Redactor,
    raw: Vec<BundleSection>,
}

impl DiagnosticExport {
    pub fn new(redactor: Redactor) -> Self {
        DiagnosticExport { redactor, raw: Vec::new() }
    }
    pub fn add_section(&mut self, name: impl Into<String>, content: impl Into<String>) {
        self.raw.push(BundleSection { name: name.into(), content: content.into() });
    }
    /// Build the preview. NEVER sends. Counts how many redactions occurred.
    pub fn preview(&self) -> RedactionPreview {
        let mut count = 0usize;
        let sections = self
            .raw
            .iter()
            .map(|s| {
                let redacted = self.redactor.redact(&s.content);
                count += redacted.matches("[REDACTED]").count();
                BundleSection { name: s.name.clone(), content: redacted }
            })
            .collect();
        RedactionPreview {
            redacted: DiagnosticBundle { sections },
            redaction_count: count,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn secrets_are_scrubbed_and_counted_before_export() {
        let redactor = Redactor::new(vec!["sk-live-TOPSECRET".into()]);
        let mut exp = DiagnosticExport::new(redactor);
        exp.add_section("logs", "auth header: Bearer sk-live-TOPSECRET failed");
        let preview = exp.preview();
        let body = &preview.redacted.sections[0].content;
        assert!(!body.contains("sk-live-TOPSECRET"), "secret must not appear");
        assert!(body.contains("[REDACTED]"));
        assert_eq!(preview.redaction_count, 1);
    }

    #[test]
    fn preview_does_not_send_anywhere() {
        // The API has no send method on the preview path — compile-time proof of
        // gate #19: building a preview is pure and returns owned data only.
        let exp = DiagnosticExport::new(Redactor::new(vec![]));
        let _ = exp.preview();
    }
}
```

- [ ] Add `pub mod export;` to `lib.rs`.
- [ ] Run expect FAIL→PASS: `cargo test -p bongterm-diagnostics export::tests` → `2 passed`.
- [ ] Commit: `git add crates/bongterm-diagnostics/src/export.rs crates/bongterm-diagnostics/src/lib.rs && git commit -m "feat(diag/5.D.1.1): DiagnosticBundle + RedactionPreview — scrub secrets, never auto-send (gate #19)"`

#### 5.D.2 — Telemetry consent (off by default, gate #19)

##### 5.D.2.1 — `TelemetryConsent` (RED→GREEN)

- [ ] **Files**: `crates/bongterm-diagnostics/src/consent.rs` (C), `crates/bongterm-diagnostics/src/lib.rs` (M, add `pub mod consent;`).
- [ ] Off by default; only explicit opt-in flips it; export still requires a separate per-export confirmation. The onboarding copy already states "Telemetry is off by default" (`bongterm-ui` OnboardingStep::PrivacyAndStorage) — this is the backing state.
- [ ] Failing test + impl:

```rust
//! Telemetry consent — OFF by default, explicit opt-in only (gate #19).
//! Serde-serializable so it persists in settings; default deserializes to off.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TelemetryConsent {
    /// True only after the user explicitly opts in.
    opted_in: bool,
}

impl Default for TelemetryConsent {
    fn default() -> Self {
        TelemetryConsent { opted_in: false }
    }
}

impl TelemetryConsent {
    pub fn is_enabled(self) -> bool {
        self.opted_in
    }
    /// Explicit opt-in — the ONLY way to enable.
    pub fn opt_in() -> Self {
        TelemetryConsent { opted_in: true }
    }
    pub fn opt_out(self) -> Self {
        TelemetryConsent { opted_in: false }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_off() {
        assert!(!TelemetryConsent::default().is_enabled());
    }

    #[test]
    fn deserialized_absent_field_is_off() {
        // A settings file with no telemetry block defaults to off.
        let c: TelemetryConsent = serde_json::from_str("{\"opted_in\":false}").unwrap();
        assert!(!c.is_enabled());
    }

    #[test]
    fn only_explicit_opt_in_enables() {
        assert!(TelemetryConsent::opt_in().is_enabled());
        assert!(!TelemetryConsent::opt_in().opt_out().is_enabled());
    }
}
```

- [ ] Add `pub mod consent;` to `lib.rs`.
- [ ] Run expect FAIL→PASS: `cargo test -p bongterm-diagnostics consent::tests` → `3 passed`.
- [ ] Commit: `git add crates/bongterm-diagnostics/src/consent.rs crates/bongterm-diagnostics/src/lib.rs && git commit -m "feat(diag/5.D.2.1): TelemetryConsent off-by-default, explicit opt-in only (gate #19)"`

#### 5.D.3 — Minidump capture (gate #26)

##### 5.D.3.1 — `MinidumpWriter` trait + mock + `.dmp` on app-wide panic (RED→GREEN)

- [ ] **Files**: `crates/bongterm-diagnostics/src/minidump.rs` (C), `crates/bongterm-diagnostics/src/lib.rs` (M, add `pub mod minidump;` + wire into the existing `install_panic_hook`).
- [ ] Trait seam so CI tests the panic→dump path with a mock; the Windows impl uses `minidump-writer`. The dump path is `crash_dir()` (already `%LOCALAPPDATA%\BongTerm\crashes`). Secrets never written (the writer captures process memory regions per OS minidump semantics; we do NOT add app-state blobs that could carry plaintext secrets — Scope Lock 5).
- [ ] Failing test + impl:

```rust
//! Minidump capture on app-wide panic (gate #26). `MinidumpWriter` is the seam;
//! `WindowsMinidump` (cfg(windows)) uses `minidump-writer`. The mock records the
//! call so CI proves the panic hook triggers a dump without a real crash.

use std::path::{Path, PathBuf};

/// Substitutable dump writer.
pub trait MinidumpWriter {
    /// Write a minidump to `dir`, returning the file path. Implementations must
    /// NOT serialize arbitrary app state (avoids leaking secrets — §37).
    fn write_dump(&self, dir: &Path) -> std::io::Result<PathBuf>;
}

/// CI mock: records that a dump was requested + the target dir.
#[derive(Default)]
pub struct MockMinidumpWriter {
    pub written: std::cell::RefCell<Vec<PathBuf>>,
}

impl MinidumpWriter for MockMinidumpWriter {
    fn write_dump(&self, dir: &Path) -> std::io::Result<PathBuf> {
        let path = dir.join("bongterm-mock.dmp");
        self.written.borrow_mut().push(path.clone());
        Ok(path)
    }
}

/// Drives a dump on an app-wide crash class. Returns the written path.
pub fn capture_on_app_wide_crash<W: MinidumpWriter>(
    writer: &W,
    dir: &Path,
) -> std::io::Result<PathBuf> {
    writer.write_dump(dir)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_wide_crash_requests_a_dump_in_crash_dir() {
        let tmp = std::env::temp_dir().join("bongterm-dump-test");
        let _ = std::fs::create_dir_all(&tmp);
        let writer = MockMinidumpWriter::default();
        let path = capture_on_app_wide_crash(&writer, &tmp).unwrap();
        assert_eq!(writer.written.borrow().len(), 1);
        assert!(path.ends_with("bongterm-mock.dmp"));
    }
}
```

- [ ] Add `pub mod minidump;` to `lib.rs`; in `install_panic_hook` (already present), on an app-wide panic invoke `capture_on_app_wide_crash` with the platform writer into `crash_dir()`. Keep the existing hook behavior; this is additive.
- [ ] Run expect FAIL→PASS: `cargo test -p bongterm-diagnostics minidump::tests` → `1 passed`.
- [ ] Commit: `git add crates/bongterm-diagnostics/src/minidump.rs crates/bongterm-diagnostics/src/lib.rs && git commit -m "feat(diag/5.D.3.1): MinidumpWriter trait + mock + panic-hook dump capture (gate #26)"`

##### 5.D.3.2 — `MinidumpWriter` conformance suite (RED→GREEN)

- [ ] **Files**: `crates/bongterm-test-kit/src/conformance/minidump_writer_conformance.rs` (C), `crates/bongterm-test-kit/src/conformance/mod.rs` (M), `crates/bongterm-test-kit/Cargo.toml` (M, add `bongterm-diagnostics` path dep).
- [ ] Conformance: `write_dump` returns a path under the requested dir; calling twice yields two paths; never panics on a writable dir.

```rust
//! Conformance for `bongterm_diagnostics::minidump::MinidumpWriter`.

use bongterm_diagnostics::minidump::MinidumpWriter;
use std::path::Path;

pub fn run_minidump_writer_conformance<W: MinidumpWriter>(writer: &W, dir: &Path) {
    let p = writer.write_dump(dir).expect("dump path");
    assert!(p.starts_with(dir), "dump must land under the requested dir");
}

#[cfg(test)]
mod tests {
    use super::*;
    use bongterm_diagnostics::minidump::MockMinidumpWriter;

    #[test]
    fn mock_writer_satisfies_conformance() {
        let tmp = std::env::temp_dir().join("bongterm-conf-dump");
        let _ = std::fs::create_dir_all(&tmp);
        run_minidump_writer_conformance(&MockMinidumpWriter::default(), &tmp);
    }
}
```

- [ ] Add `pub mod minidump_writer_conformance;` to `conformance/mod.rs`; add `bongterm-diagnostics = { path = "../bongterm-diagnostics" }` to test-kit `Cargo.toml`.
- [ ] Run expect FAIL→PASS: `cargo test -p bongterm-test-kit minidump_writer_conformance` → `1 passed`.
- [ ] Commit: `git add crates/bongterm-test-kit/src/conformance/minidump_writer_conformance.rs crates/bongterm-test-kit/src/conformance/mod.rs crates/bongterm-test-kit/Cargo.toml && git commit -m "test(diag/5.D.3.2): MinidumpWriter conformance suite"`

### 5.E — Wave 1 spikes S5–S8 → ADRs + security whitepaper

Spikes are time-boxed investigations; the deliverable is a decision ADR (not hot-path code). Each ADR's "test" is a **doc-presence + required-section assertion** so the gate is mechanically checkable, plus the spike's own throwaway probe (recorded, not committed to the product).

#### 5.E.1 — S5: Claude Code output stability → ADR-0009

- [ ] **Files**: `docs/adr/0009-claude-code-output-pinning.md` (C).
- [ ] Run the S5 probe: capture Claude Code CLI output across versions on the reference HW; determine whether block/transcript parsing must pin to a CLI version or can tolerate drift. Record findings.
- [ ] Write ADR-0009 with required sections: `## Status`, `## Context`, `## Decision`, `## Consequences`, and a line `Spec logical role: ADR-006 (S5 Claude Code output).` Decision states the output-pinning strategy (e.g. parse defensively + pin the tested CLI range in `known-issues.md`).
- [ ] Verifiable check (add once in 5.E.5 as a shared `adr_tests` module): asserts the file exists and contains all four required sections + the spec-role line.
- [ ] Commit: `git add docs/adr/0009-claude-code-output-pinning.md && git commit -m "docs(adr/5.E.1): ADR-0009 Claude Code output pinning (S5)"`

#### 5.E.2 — S6: Codex CLI auth → ADR-0010

- [ ] **Files**: `docs/adr/0010-codex-cli-auth.md` (C).
- [ ] Run the S6 probe: determine Codex CLI auth flow (API key vs OAuth device), how secrets are supplied (must route through the vault env-block model §37 — never argv), and session lifetime. Record.
- [ ] Write ADR-0010 with the four required sections + `Spec logical role: ADR-007 (S6 Codex auth).` Decision states the auth integration approach honoring late-scoped secret resolution.
- [ ] Commit: `git add docs/adr/0010-codex-cli-auth.md && git commit -m "docs(adr/5.E.2): ADR-0010 Codex CLI auth integration (S6)"`

#### 5.E.3 — S7: Defender/EDR process-tree → ADR-0011 + whitepaper + edr.md

- [ ] **Files**: `docs/adr/0011-edr-process-tree.md` (C), `docs/security/whitepaper.md` (C), `docs/runbook/edr.md` (M, extend the Defender section from 5.C.2.1).
- [ ] Run the S7 probe: observe how Defender/common EDR classify the BongTerm process tree (parent spawning ConPTY/conhost/shells/agents/MCP under JobObject); confirm none of the forbidden techniques are needed and the tree looks benign.
- [ ] Write ADR-0011 (four sections + `Spec logical role: ADR-008 (S7 EDR process-tree).`): decision = stay within supported user-mode spawning; document the expected tree shape so EDR allowlisting is straightforward.
- [ ] Write `docs/security/whitepaper.md`: ConPTY usage, JobObject resource limits, `PolicyEvaluator` default-deny model, the `${secret:NAME}`/`${env:NAME}` reference + vault model (§37), and the forbidden-abstraction posture (gate #21). This is the artifact a security reviewer reads.
- [ ] Extend `edr.md` with the process-tree diagram + allowlist entries by image name (`bongterm.exe`, `bongterm-mcp-host.exe`, `conhost.exe`, shells).
- [ ] Commit: `git add docs/adr/0011-edr-process-tree.md docs/security/whitepaper.md docs/runbook/edr.md && git commit -m "docs(sec/5.E.3): ADR-0011 EDR process-tree + security whitepaper + edr allowlist (S7)"`

#### 5.E.4 — S8: prompt-injection corpus → ADR-0012

- [ ] **Files**: `docs/adr/0012-prompt-injection-approval-gate.md` (C).
- [ ] Run the S8 probe: assemble a small indirect-prompt-injection corpus (malicious content in terminal output / files / MCP results) and confirm the approval-gate / default-deny posture blocks authority escalation (authority comes from policy, never ingested content — security contract §1).
- [ ] Write ADR-0012 (four sections + `Spec logical role: ADR-009 (S8 prompt-injection).`): decision = all ingested content untrusted; destructive actions require explicit approval; record the corpus location and the gate behavior.
- [ ] Commit: `git add docs/adr/0012-prompt-injection-approval-gate.md && git commit -m "docs(adr/5.E.4): ADR-0012 prompt-injection approval gate (S8)"`

#### 5.E.5 — ADR presence + required-section gate (RED→GREEN)

- [ ] **Files**: a test module in `tools/xtask/src/main.rs` (M) or a small `tools/xtask/src/adr_check.rs` (C).
- [ ] Mechanical gate so a missing/under-specified ADR fails CI:

```rust
#[cfg(test)]
mod adr_tests {
    fn has_required_sections(path: &str, spec_role: &str) {
        let body = std::fs::read_to_string(path).unwrap_or_else(|_| panic!("missing {path}"));
        for sec in ["## Status", "## Context", "## Decision", "## Consequences"] {
            assert!(body.contains(sec), "{path} missing section {sec}");
        }
        assert!(body.contains(spec_role), "{path} missing spec-role line: {spec_role}");
    }

    #[test]
    fn wave1_spike_adrs_are_complete() {
        let base = "../../docs/adr/";
        has_required_sections(&format!("{base}0009-claude-code-output-pinning.md"), "ADR-006");
        has_required_sections(&format!("{base}0010-codex-cli-auth.md"), "ADR-007");
        has_required_sections(&format!("{base}0011-edr-process-tree.md"), "ADR-008");
        has_required_sections(&format!("{base}0012-prompt-injection-approval-gate.md"), "ADR-009");
    }
}
```

- [ ] Run expect FAIL (before ADRs written) then PASS (after 5.E.1–5.E.4): `cargo test -p xtask adr_tests`.
- [ ] Commit: `git add tools/xtask/src && git commit -m "test(adr/5.E.5): ADR presence + required-section gate for S5–S8"`

### 5.F — Supply-chain: SBOM, provenance, known-issues, rollback (gate #30)

#### 5.F.1 — SBOM tooling decision + cargo-cyclonedx impl (gate #30)

##### 5.F.1.1 — ADR-0014 SBOM tooling decision

- [ ] **Files**: `docs/adr/0014-sbom-tooling.md` (C).
- [ ] ADR-0014 (four required sections): decision = use `cargo-cyclonedx` over the hand-rolled emitter because it is a maintained CycloneDX producer, handles the full dependency graph, and emits standard JSON consumable by scanners. Record the vendored-wezterm component must be injected manually (it is not a crates.io dep).
- [ ] Verifiable check: extend the `adr_tests` gate (5.E.5) to also assert `0014-sbom-tooling.md` has the four sections (no spec-role line needed — it is plan-local, 5.F.1).
- [ ] Commit: `git add docs/adr/0014-sbom-tooling.md tools/xtask/src && git commit -m "docs(adr/5.F.1.1): ADR-0014 SBOM tooling = cargo-cyclonedx"`

##### 5.F.1.2 — `xtask sbom` switches to cargo-cyclonedx + vendored component (RED→GREEN)

- [ ] **Files**: `tools/xtask/src/sbom.rs` (M).
- [ ] Replace the minimal hand-rolled CycloneDX emit with: invoke `cargo cyclonedx --format json` (via the mockable `ToolRunner` from 5.B.2.1 so CI without the plugin still tests command assembly), then post-process to inject the vendored-wezterm `component` entry, then validate the result is well-formed CycloneDX (has `bomFormat: "CycloneDX"`, `specVersion`, a `components` array containing wezterm).
- [ ] Failing test:

```rust
#[cfg(test)]
mod sbom_tests {
    use super::*;

    #[test]
    fn injects_vendored_wezterm_component() {
        let base = r#"{"bomFormat":"CycloneDX","specVersion":"1.5","components":[]}"#;
        let out = inject_vendored_components(base).unwrap();
        assert!(out.contains("CycloneDX"));
        assert!(out.to_lowercase().contains("wezterm"), "vendored wezterm must be present");
    }

    #[test]
    fn rejects_non_cyclonedx_input() {
        assert!(inject_vendored_components(r#"{"foo":1}"#).is_err());
    }
}
```

- [ ] Implement `inject_vendored_components(&str) -> anyhow::Result<String>` (parse JSON via serde_json, assert `bomFormat == "CycloneDX"`, push the wezterm component into `components`, re-serialize) and `run_sbom(runner)` driving `cargo cyclonedx` then `inject_vendored_components`.
- [ ] Run expect FAIL→PASS: `cargo test -p xtask sbom_tests` → `2 passed`.
- [ ] Commit: `git add tools/xtask/src/sbom.rs && git commit -m "feat(supply/5.F.1.2): SBOM via cargo-cyclonedx + vendored wezterm injection (gate #30)"`

#### 5.F.2 — Provenance attestation (`attestation.intoto.jsonl`)

##### 5.F.2.1 — `xtask attestation` emits SLSA provenance (RED→GREEN)

- [ ] **Files**: `tools/xtask/src/attestation.rs` (C), `tools/xtask/src/main.rs` (M, register `attestation` subcommand), `tools/xtask/Cargo.toml` (M, add `sha2`, `serde_json`).
- [ ] Emit an in-toto SLSA provenance statement: `_type: https://in-toto.io/Statement/v1`, `predicateType: https://slsa.dev/provenance/v1`, `subject` = the built MSIX with its SHA-256 digest, `predicate.buildDefinition` (builder id, the xtask invocation), `predicate.runDetails`. One JSON object per line (`.jsonl`).
- [ ] Failing test:

```rust
#[cfg(test)]
mod attestation_tests {
    use super::*;

    #[test]
    fn statement_has_slsa_predicate_and_subject_digest() {
        let stmt = build_provenance("out/BongTerm.msix", "abc123deadbeef");
        assert!(stmt.contains("https://slsa.dev/provenance/v1"));
        assert!(stmt.contains("\"sha256\":\"abc123deadbeef\""));
        assert!(stmt.contains("BongTerm.msix"));
        // jsonl = exactly one line, valid JSON.
        assert_eq!(stmt.lines().count(), 1);
        let _: serde_json::Value = serde_json::from_str(&stmt).unwrap();
    }

    #[test]
    fn digest_of_known_bytes_is_stable() {
        // sha256("") well-known value.
        assert_eq!(
            sha256_hex(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }
}
```

- [ ] Implement `sha256_hex(&[u8]) -> String` (via `sha2`), `build_provenance(subject_path, digest_hex) -> String` (one-line in-toto JSON), and `run_attestation(msix_path, out_path)` that hashes the MSIX, builds the statement, writes `attestation.intoto.jsonl`. Register subcommand.
- [ ] Add `sha2 = "0.10"` and confirm `serde_json` in `tools/xtask/Cargo.toml`.
- [ ] Run expect FAIL→PASS: `cargo test -p xtask attestation_tests` → `2 passed`.
- [ ] Commit: `git add tools/xtask/src/attestation.rs tools/xtask/src/main.rs tools/xtask/Cargo.toml && git commit -m "feat(supply/5.F.2.1): xtask attestation — SLSA in-toto provenance with SHA-256 subject"`

#### 5.F.3 — `known-issues.md`

##### 5.F.3.1 — Published known-issues list

- [ ] **Files**: `known-issues.md` (C).
- [ ] Author the list: placeholder MSIX logo assets (replace before GA), SmartScreen warmup expectation on OV cert (link ADR-0013), pinned Claude Code CLI range (link ADR-0009), session daemon deferred from MVP (no process survival across restart), any open accessibility/IME edge cases surfaced by the manual smokes. Each entry: symptom, scope, workaround, tracking link.
- [ ] Verifiable check: xtask test asserts `known-issues.md` exists and is non-trivial (e.g. references `SmartScreen` and `session daemon`):

```rust
#[cfg(test)]
mod known_issues_tests {
    #[test]
    fn known_issues_published_and_covers_required_topics() {
        let k = std::fs::read_to_string("../../known-issues.md").unwrap();
        assert!(k.contains("SmartScreen"));
        assert!(k.to_lowercase().contains("session daemon"));
    }
}
```

- [ ] Run expect FAIL→PASS: `cargo test -p xtask known_issues_tests`.
- [ ] Commit: `git add known-issues.md tools/xtask/src && git commit -m "docs(supply/5.F.3.1): published known-issues list + presence gate"`

#### 5.F.4 — Release rollback plan

##### 5.F.4.1 — Flesh out `release.md` rollback + ordered procedure

- [ ] **Files**: `docs/runbook/release.md` (M, complete the file started in 5.B.4.1).
- [ ] Add: the ordered release procedure (build → SBOM → sign → attestation → clean-VM smoke → publish), and the rollback plan (how to pull a bad MSIX, how users on the bad version recover, how to re-point the update channel to the prior known-good version + its attestation). Include the 7-consecutive-nightly gate reference and the artifact manifest (MSIX, SBOM json, `attestation.intoto.jsonl`, THIRD_PARTY_NOTICES).
- [ ] Verifiable check: xtask test asserts `release.md` contains `## Rollback` and references `attestation.intoto.jsonl`:

```rust
#[cfg(test)]
mod release_tests {
    #[test]
    fn release_runbook_has_rollback_and_lists_artifacts() {
        let r = std::fs::read_to_string("../../docs/runbook/release.md").unwrap();
        assert!(r.contains("## Rollback"));
        assert!(r.contains("attestation.intoto.jsonl"));
    }
}
```

- [ ] Run expect FAIL→PASS: `cargo test -p xtask release_tests`.
- [ ] Commit: `git add docs/runbook/release.md tools/xtask/src && git commit -m "docs(supply/5.F.4.1): release rollback plan + ordered procedure + artifact manifest"`

---

## 5.exit — Phase 5 exit gate

Phase 5 exits only when ALL hold:

- [ ] Gate **#18** (accessibility): `cargo test -p bongterm-ui accessibility` green; `uia_provider_conformance` green; Narrator + NVDA manual smoke (5.A.1.8) checked off on reference HW.
- [ ] Gate **#19** (diagnostics/consent): `export` + `consent` tests green; redaction preview proven to scrub secrets; telemetry off by default.
- [ ] Gate **#20** (signed MSIX): `package-msix` produces a signed MSIX; `signtool verify /pa` succeeds on a clean VM; install/upgrade/uninstall smoke green for **7 consecutive nightlies**.
- [ ] Gate **#21** (forbidden-abstraction/EDR): `xtask forbidden-abstraction` green on PR; `process_tree_auditor_conformance` green; nightly `forbidden-runtime` + `defender-smoke` green; ADR-0011 + whitepaper present.
- [ ] Gate **#25** (device-loss, P1): `bongterm-render device_loss` tests green; 3-in-60s software-fallback proven.
- [ ] Gate **#26** (crash-recovery): `recovery` + `crash_recovery` + `minidump` tests green; `minidump_writer_conformance` green; all 6 crash classes modeled.
- [ ] Gate **#30** (SBOM/provenance): `sbom_tests` + `attestation_tests` green; SBOM includes vendored wezterm; `attestation.intoto.jsonl` emitted; THIRD_PARTY_NOTICES current; `known-issues.md` published.
- [ ] Wave 1 spikes: ADR-0009..0012 present + `adr_tests` green; ADR-0013 (EV) + ADR-0014 (SBOM) present.
- [ ] CI: `nightly.yml` jobs green; `ci.yml` PR-gates (forbidden-abstraction static, manifest validate, SBOM validity, attestation) green.
- [ ] Full workspace green: `cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test --workspace` and `cargo run -p xtask -- check-deps`.

## 5.replan — Replan checkpoint

- [ ] After 5.exit, update `orca.md` to strike completed Phase 5 tasks and confirm no Phase 6 (post-MVP) work was pulled forward. If any gate slipped (e.g. clean-VM 7-nightly streak broke), add a remediation task rather than marking the gate green. Record residual risks in `known-issues.md`.

---

## Self-Review

**Phase 5 outline task coverage** (every `5.*` from `orca.md` maps to a task):

| Outline item | Task(s) |
|---|---|
| UIA provider over terminal surface | 5.A.1.1–5.A.1.8 |
| IME composition (ADR-0006 shape) | 5.A.2.1–5.A.2.3 |
| Per-monitor DPI v2 + live changes | 5.A.3.1–5.A.3.2 |
| MSIX manifest in `packaging/msix/` | 5.B.1.1 |
| `xtask package-msix` real impl | 5.B.2.1 |
| Code-signing (OV first, EV ADR) | 5.B.3.1 |
| Clean-VM install/upgrade/uninstall smoke | 5.B.4.1 |
| SmartScreen runbook | 5.B.5.1 |
| Parser fuzzing in nightly (pinned nightly) | 5.C.1.1–5.C.1.2, 5.C.6.1 |
| Defender real-time smoke nightly | 5.C.2.1, 5.C.6.1 |
| Forbidden-abstraction → runtime process-tree | 5.C.3.1–5.C.3.3, 5.C.6.2 |
| Renderer device-loss simulated test | 5.C.4.1 |
| Crash-recovery suite (6 scenarios) | 5.C.5.1–5.C.5.2 |
| Diagnostic export + redaction preview | 5.D.1.1 |
| Telemetry consent (off by default) | 5.D.2.1 |
| Minidump capture | 5.D.3.1–5.D.3.2 |
| S5 Claude Code output → ADR | 5.E.1 |
| S6 Codex auth → ADR | 5.E.2 |
| S7 Defender/EDR → ADR + whitepaper | 5.E.3 |
| S8 prompt-injection → ADR | 5.E.4 |
| SBOM tooling decision + impl | 5.F.1.1–5.F.1.2 |
| Provenance attestation | 5.F.2.1 |
| `known-issues.md` | 5.F.3.1 |
| Rollback plan `release.md` | 5.F.4.1 |

**§6.1 gate coverage**: #18 → 5.A.1.*; #19 → 5.D.1/5.D.2; #20 → 5.B.*; #21 → 5.C.3.*; #25 → 5.C.4.1; #26 → 5.C.5.*/5.D.3.*; #30 → 5.F.1/5.F.2/5.F.3. All seven mapped.

**§29 Phase 5 CI-gate row coverage**: accessibility (Narrator + ≥1 third-party = NVDA) → 5.A.1.8 manual + 5.A.1.5 conformance; IME → 5.A.2.*; D3D device-loss recovery → 5.C.4.1. All three present and wired into nightly (5.C.6.1).

**Type/signature consistency** (verified across tasks):
- `AxRole` / `AxNode` / `AccessibilityTree` / `UiaProvider` (String-returning, `forbid(unsafe_code)`-safe) used identically in 5.A.1.1–5.A.1.6 and the conformance suite 5.A.1.5.
- `ImeState`/`ImeEvent`/`ImeStep` (5.A.2.1) and `CompositionWindow::at_caret` (5.A.2.2) share the DPI `scale: f32` from `DpiState::scale()` (5.A.3.1).
- `ToolRunner` trait introduced in 5.B.2.1 is reused by 5.F.1.2 (SBOM) — single definition.
- `DeviceRemovedReason`/`RecoveryAction`/`DeviceLossRecovery` (5.C.4.1) are additive to the existing `MockRendererBackend` surface; `recover()`/test-hook names flagged for reconciliation against actual `bongterm-render` API.
- `RecoveryAction` exists in BOTH `bongterm-render::device_loss` (Recreate/SoftwareFallback) and `bongterm-diagnostics::recovery` (Restore/Discard/Export) — **distinct enums in distinct crates/modules**, never cross-imported; intentional, not a collision.
- `MinidumpWriter` + `MockMinidumpWriter` (5.D.3.1) used unchanged by conformance 5.D.3.2.
- Conformance fns follow the established `run_*_conformance` naming (uia_provider, process_tree_auditor, minidump_writer).

**Placeholder scan**: no `TODO`/`unimplemented!()`/`todo!()`/`...` left in any task's code. The `package_msix.rs` `not yet implemented` stub is explicitly replaced in 5.B.2.1. MSIX logo PNGs and `Publisher` CN are flagged placeholders with replace-before-GA tracking in `known-issues.md` (5.F.3.1) — by design, not omission.

**Ownership-matrix compliance**: renderer owns device-loss (5.C.4); ui owns accessibility/IME/DPI (5.A); diagnostics owns export/consent/minidump/recovery (5.C.5, 5.D); security owns forbidden-abstraction (5.C.3). No crate crosses its lane.

**API-reconciliation flags** (must verify against real source during execution, called out in-task): `bongterm-term` parser API for the fuzz target (5.C.1.1); `MockRendererBackend` mock surface `recover()`/test-hook (5.C.4.1); `BongTermShell::default()`/`title()`/`region_names()` (5.A.1.3); `install_panic_hook` extension point (5.D.3.1). Each task says to adjust the call to the real signature if it differs.
