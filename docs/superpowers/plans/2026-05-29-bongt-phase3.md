# BongTerm Phase 3 Execution Plan (Developer UX)

Date: 2026-05-29
Source: `docs/superpowers/specs/2026-05-27-bongt-mvp0-design.md` (§6.1 gates #9, #10, #11, #12, #13, #14; §3.3 AI runtime; §3.2 queue classification)
Status: Active

> **For agentic workers**: this plan is written for an autonomous coding agent. Each task is bite-sized and follows strict TDD: write a failing test (full code), run it and confirm the exact failure, write the minimal implementation (full code), run it and confirm pass, then commit with the exact command. Do not skip the run-and-confirm steps. Do not batch tasks. No placeholders — every code block is real, compilable Rust with exact paths. Types and signatures are consistent across tasks; when a later task references a type from an earlier task, it is the same type.

## Goal

Deliver the developer-UX layer — Cmd-K AI assist, failed-command explainer, smart history with filters + frecency, JSON5 snippets with parameter prompts, background jobs with desktop toasts, and clickable error/URL/OSC-8 patterns — all off the terminal hot path, preview-only for AI, and exit only when §6.1 gates #9–#14 are green for 7 consecutive nightly runs.

## Architecture

All Phase 3 features live in `bongterm-devassist`, which owns five submodules (`ai`, `history`, `snippets`, `jobs`, `patterns`) behind narrow port traits; `bongterm-ui` consumes view-model snapshots and never spawns processes, reads secrets, or mutates Git/scrollback directly. AI assist wraps the existing `bongterm-agents::ClaudeCodeAdapter` in non-interactive mode (`claude --print --output-format json`), is strictly preview-only until an explicit Run confirmation, and degrades to a labelled "unavailable" state when Claude Code is absent. Smart-history frecency, snippet libraries, and the AI subprocess all run off the hot path; clickable file:line spans and OSC 8 hyperlinks render as an overlay computed from `SurfaceSnapshot` ranges and never mutate scrollback.

## Tech Stack

- Rust 1.95, edition 2024, `x86_64-pc-windows-msvc`, stable only.
- `bongterm-devassist` new deps (workspace-pinned): `serde`, `serde_json`, `json5`, `thiserror`, `time`, `uuid`, `tracing`, `tokio` (process spawn for `ai`/`jobs`), `regex` (pattern matchers — NEW workspace pin), `rusqlite` is **not** a direct dep (frecency goes through `bongterm-storage-api` traits; SQLite impl stays in `bongterm-storage-sqlite`).
- Desktop toasts: `windows` crate `UI_Notifications` feature (NEW feature flag on the existing `windows` pin) wrapped behind a `Notifier` port so non-Windows test runs use a mock.
- Tests: `cargo test`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt`. Mocks live in `bongterm-test-kit` following the existing inline-mock + conformance pattern.

---

## File Structure

Every file created or modified in Phase 3, with its single responsibility.

### `bongterm-devassist` (feature crate — all Phase 3 logic)

| File | Responsibility |
|---|---|
| `crates/bongterm-devassist/Cargo.toml` | Add deps: `serde`, `serde_json`, `json5`, `thiserror`, `time`, `uuid`, `tracing`, `tokio`, `regex`, `windows`; dev-dep `bongterm-test-kit`. |
| `crates/bongterm-devassist/src/lib.rs` | Crate root; `pub mod ai; pub mod history; pub mod snippets; pub mod jobs; pub mod patterns;` + shared `DevassistError`. |
| `crates/bongterm-devassist/src/ai/mod.rs` | `ai` submodule root: re-exports. |
| `crates/bongterm-devassist/src/ai/runner.rs` | `AiBackend` port trait + `ClaudeCodeAiRunner` (wraps `claude --print --output-format json`); `AiSuggestion`, `AiRequest`, `AiAvailability`. |
| `crates/bongterm-devassist/src/ai/cmdk.rs` | `CmdKSession` state machine: NL→command, **preview-only**, explicit `confirm_run()` gate. |
| `crates/bongterm-devassist/src/ai/explainer.rs` | `Explainer`: builds context from a failed `CommandBlockRow` + transcript tail, requests explanation. |
| `crates/bongterm-devassist/src/history/mod.rs` | `history` submodule root. |
| `crates/bongterm-devassist/src/history/filter.rs` | `HistoryFilter` parser for `cwd:`/`branch:`/`agent:`/`exit:`/`time:`/`shell:`/`duration:`; closed `FilterKind` enum. |
| `crates/bongterm-devassist/src/history/frecency.rs` | `FrecencyScorer` (recency+frequency) + `FrecencyStore` port; `HistoryEntry`. |
| `crates/bongterm-devassist/src/snippets/mod.rs` | `snippets` submodule root. |
| `crates/bongterm-devassist/src/snippets/model.rs` | `Snippet`, `SnippetScope`, JSON5 library load, `${param:name}` parse → `Param` list. |
| `crates/bongterm-devassist/src/snippets/render.rs` | `render_snippet(snippet, &params)` substitution; rejects missing/unknown params. |
| `crates/bongterm-devassist/src/jobs/mod.rs` | `jobs` submodule root. |
| `crates/bongterm-devassist/src/jobs/runner.rs` | `JobRunner` port + `JobSpec`, `JobId`, closed `JobState` enum; `Notifier` port. |
| `crates/bongterm-devassist/src/jobs/list.rs` | `JobList` view-model: register/update/snapshot for the job panel. |
| `crates/bongterm-devassist/src/patterns/mod.rs` | `patterns` submodule root. |
| `crates/bongterm-devassist/src/patterns/matchers.rs` | `PatternMatcher` set for Node/Python/Rust/.NET/TS file:line; closed `PatternKind` enum; `Span`. |
| `crates/bongterm-devassist/src/patterns/url.rs` | URL detection + OSC 8 hyperlink parse; `LinkSpan`, `verify_destination()` (OSC 8 spoof guard). |

### `bongterm-storage-api` (frecency port)

| File | Responsibility |
|---|---|
| `crates/bongterm-storage-api/src/lib.rs` | Add `FrecencyRow`, `FrecencyRepo` trait (record use, top-N by score, filter passthrough). |

### `bongterm-storage-sqlite` (frecency impl)

| File | Responsibility |
|---|---|
| `crates/bongterm-storage-sqlite/src/lib.rs` | `SqliteFrecencyRepo` impl + `0002_frecency.sql` migration string. |

### `bongterm-test-kit` (mocks + conformance)

| File | Responsibility |
|---|---|
| `crates/bongterm-test-kit/src/conformance/mod.rs` | Register `frecency_repo_conformance`, `ai_backend_conformance`. |
| `crates/bongterm-test-kit/src/conformance/frecency_repo_conformance.rs` | `MockFrecencyRepo` + `run_frecency_repo_conformance`. |
| `crates/bongterm-test-kit/src/mocks/ai_backend.rs` | `MockAiBackend` (scripted suggestion / unavailable). |
| `crates/bongterm-test-kit/src/mocks/notifier.rs` | `MockNotifier` (records toasts). |
| `crates/bongterm-test-kit/src/mocks/mod.rs` | `pub mod ai_backend; pub mod notifier;`. |

### `bongterm-ui` (view-model wiring — thin)

| File | Responsibility |
|---|---|
| `crates/bongterm-ui/src/devux/mod.rs` | View-model adapters mapping devassist snapshots to UI state (Cmd-K palette entry, job panel, clickable overlay). No process spawn. |

### Workspace

| File | Responsibility |
|---|---|
| `Cargo.toml` | Add `regex` workspace pin; add `UI_Notifications` to `windows` features. |
| `tools/xtask/allowed-deps.toml` | Add `bongterm-storage-api` edge already covered; confirm `bongterm-devassist` edge set; add `regex`/`windows` as external (no change to inter-crate matrix needed). |

---

## Conventions (read once)

- Commit style matches history: `feat(devassist/3.A.1): <summary>`; chore commits `chore(...)`. One commit per task.
- Crate headers match siblings: `#![deny(unsafe_code)]` (or `#![forbid(unsafe_code)]` where no `windows` FFI), `#![warn(clippy::pedantic)]`, `#![allow(clippy::module_name_repetitions)]`, `#![allow(clippy::missing_errors_doc)]`. The `jobs` and `patterns::url` modules that touch the `windows` crate use `#![deny(unsafe_code)]` at crate root but `#[allow(unsafe_code)]` is NOT used — all `windows` calls used here are safe wrappers; if an `unsafe` block is unavoidable for `UI_Notifications`, isolate it in `jobs/runner.rs` behind a `// SAFETY:` comment and switch the crate attribute to `#![deny(unsafe_code)]` with a localized `#[allow(unsafe_code)]` on that one function (documented).
- Exact run command for a single crate: `cargo test -p bongterm-devassist <test_name>`.
- All new public types derive `Debug`; DTOs persisted or serialized derive `serde::Serialize, serde::Deserialize`.

---

## Task Group 3.A — AI assist (Cmd-K + explainer + fallback) → gates #9, #10

### 3.A.0 — Crate wiring: deps + module skeleton

- [ ] **Files**
  - Modify `Cargo.toml` (workspace): add `regex` pin.
  - Modify `crates/bongterm-devassist/Cargo.toml`: add deps.
  - Modify `crates/bongterm-devassist/src/lib.rs`: declare modules + `DevassistError`.
  - Create `crates/bongterm-devassist/src/ai/mod.rs`, `src/history/mod.rs`, `src/snippets/mod.rs`, `src/jobs/mod.rs`, `src/patterns/mod.rs` (empty module roots).

- [ ] **(1) Failing test** — append to `crates/bongterm-devassist/src/lib.rs`:

```rust
#[cfg(test)]
mod wiring_tests {
    use crate::DevassistError;

    #[test]
    fn error_display_is_nonempty() {
        let e = DevassistError::Backend("claude exited 1".to_string());
        assert!(!format!("{e}").is_empty());
    }

    #[test]
    fn submodules_are_declared() {
        // Compile-time proof the modules exist and are public.
        let _ = crate::ai::MODULE_NAME;
        let _ = crate::history::MODULE_NAME;
        let _ = crate::snippets::MODULE_NAME;
        let _ = crate::jobs::MODULE_NAME;
        let _ = crate::patterns::MODULE_NAME;
    }
}
```

- [ ] **(2) Run, expect FAIL**: `cargo test -p bongterm-devassist wiring_tests`
  - Expected: `error[E0433]: failed to resolve: ... DevassistError` / `could not find 'ai' in the crate root`.

- [ ] **(3) Minimal impl**
  - Workspace `Cargo.toml` — add under `[workspace.dependencies]`:

```toml
regex = "1"
```

  - Add `UI_Notifications` to the `windows` features list in workspace `Cargo.toml` (append to the existing `features = [ ... ]` array):

```toml
    "UI_Notifications",
    "Data_Xml_Dom",
```

  - `crates/bongterm-devassist/Cargo.toml`:

```toml
[package]
name = "bongterm-devassist"
version = "0.0.0"
edition.workspace = true
rust-version.workspace = true
license.workspace = true
publish = false

[dependencies]
bongterm-agents = { path = "../bongterm-agents" }
bongterm-blocks = { path = "../bongterm-blocks" }
bongterm-storage-api = { path = "../bongterm-storage-api" }
serde = { workspace = true }
serde_json = { workspace = true }
json5 = { workspace = true }
thiserror = { workspace = true }
time = { workspace = true }
uuid = { workspace = true }
tracing = { workspace = true }
tokio = { workspace = true }
regex = { workspace = true }
windows = { workspace = true }

[dev-dependencies]
bongterm-test-kit = { path = "../bongterm-test-kit" }
```

  - Replace `crates/bongterm-devassist/src/lib.rs` body:

```rust
//! bongterm-devassist
//!
//! Developer-UX features for BongTerm MVP-0: Cmd-K AI assist, failed-command
//! explainer, smart history, snippets, background jobs, clickable patterns.
//!
//! See `docs/superpowers/specs/2026-05-27-bongt-mvp0-design.md` §1.2 for the
//! ownership matrix entry. This crate MUST NOT touch the terminal hot path.

#![deny(unsafe_code)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::missing_errors_doc)]

pub mod ai;
pub mod history;
pub mod jobs;
pub mod patterns;
pub mod snippets;

/// Errors returned by devassist features.
#[derive(Debug, thiserror::Error)]
pub enum DevassistError {
    #[error("AI backend error: {0}")]
    Backend(String),
    #[error("AI assist unavailable: {0}")]
    Unavailable(String),
    #[error("parse error: {0}")]
    Parse(String),
    #[error("missing parameter: {0}")]
    MissingParam(String),
    #[error("storage error: {0}")]
    Storage(String),
    #[error("job error: {0}")]
    Job(String),
}

#[cfg(test)]
mod wiring_tests {
    use crate::DevassistError;

    #[test]
    fn error_display_is_nonempty() {
        let e = DevassistError::Backend("claude exited 1".to_string());
        assert!(!format!("{e}").is_empty());
    }

    #[test]
    fn submodules_are_declared() {
        let _ = crate::ai::MODULE_NAME;
        let _ = crate::history::MODULE_NAME;
        let _ = crate::snippets::MODULE_NAME;
        let _ = crate::jobs::MODULE_NAME;
        let _ = crate::patterns::MODULE_NAME;
    }
}
```

  - Each module root file (e.g. `crates/bongterm-devassist/src/ai/mod.rs`) starts as:

```rust
//! AI assist submodule (Cmd-K + failed-command explainer).
pub(crate) const MODULE_NAME: &str = "ai";
```

  (Replace `"ai"` with `"history"`, `"snippets"`, `"jobs"`, `"patterns"` respectively in the other four files.)

- [ ] **(4) Run, expect PASS**: `cargo test -p bongterm-devassist wiring_tests` → 2 passed.
- [ ] **(5) Commit**: `git add crates/bongterm-devassist Cargo.toml && git commit -m "feat(devassist/3.A.0): crate wiring — deps, modules, DevassistError"`

---

### 3.A.1 — `AiBackend` port + `MockAiBackend` + `ClaudeCodeAiRunner` skeleton

- [ ] **Files**
  - Create `crates/bongterm-devassist/src/ai/runner.rs`.
  - Modify `crates/bongterm-devassist/src/ai/mod.rs`: `pub mod runner; pub use runner::*;`.
  - Create `crates/bongterm-test-kit/src/mocks/mod.rs` + `crates/bongterm-test-kit/src/mocks/ai_backend.rs`.
  - Modify `crates/bongterm-test-kit/src/lib.rs`: `pub mod mocks;`.

- [ ] **(1) Failing test** — create `crates/bongterm-devassist/src/ai/runner.rs` test module (the test references the mock from test-kit; full impl follows in step 3):

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unavailable_backend_reports_unavailable() {
        let backend = UnavailableBackend::new("Claude Code not installed");
        match backend.availability() {
            AiAvailability::Unavailable { reason } => {
                assert!(reason.contains("not installed"));
            }
            AiAvailability::Available { .. } => panic!("expected unavailable"),
        }
    }

    #[test]
    fn request_carries_context_and_intent() {
        let req = AiRequest {
            intent: AiIntent::NlToCommand,
            user_text: "list files sorted by size".to_string(),
            context: AiContext {
                cwd: "C:\\proj".to_string(),
                shell: "pwsh".to_string(),
                failed_command: None,
                transcript_tail: String::new(),
            },
        };
        assert_eq!(req.intent, AiIntent::NlToCommand);
        assert!(req.user_text.contains("size"));
    }
}
```

- [ ] **(2) Run, expect FAIL**: `cargo test -p bongterm-devassist ai::runner`
  - Expected: `cannot find type 'AiAvailability'` / `UnavailableBackend` unresolved.

- [ ] **(3) Minimal impl** — full body of `crates/bongterm-devassist/src/ai/runner.rs` (prepend above the test module):

```rust
//! AI backend port + Claude Code subprocess runner.
//!
//! The backend is PREVIEW-ONLY: it returns suggestions, never executes them.
//! Execution is gated behind `super::cmdk::CmdKSession::confirm_run`.

use crate::DevassistError;

/// What the caller wants the AI to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AiIntent {
    /// Natural-language → shell command (Cmd-K).
    NlToCommand,
    /// Explain a failed command.
    ExplainFailure,
}

/// Read-only context handed to the backend. Never contains secrets.
#[derive(Debug, Clone)]
pub struct AiContext {
    pub cwd: String,
    pub shell: String,
    /// The failed command text, when intent is `ExplainFailure`.
    pub failed_command: Option<String>,
    /// Redacted tail of recent output; bounded length.
    pub transcript_tail: String,
}

/// A single AI request.
#[derive(Debug, Clone)]
pub struct AiRequest {
    pub intent: AiIntent,
    pub user_text: String,
    pub context: AiContext,
}

/// A preview-only suggestion. `command` is NEVER auto-run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AiSuggestion {
    /// The suggested command text (preview only).
    pub command: String,
    /// Human-readable rationale / explanation.
    pub explanation: String,
}

/// Whether the AI backend can be used right now.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AiAvailability {
    Available { version: String },
    Unavailable { reason: String },
}

/// Port for an AI backend. Implementations run OFF the hot path.
pub trait AiBackend: Send + Sync {
    /// Report whether the backend is usable (e.g. Claude Code installed).
    fn availability(&self) -> AiAvailability;
    /// Produce a preview-only suggestion. Must not execute anything.
    fn suggest(&self, request: &AiRequest) -> Result<AiSuggestion, DevassistError>;
}

/// A backend that is always unavailable — used as the graceful fallback when
/// Claude Code is not detected (gate #9 / #10 "not installed" path).
#[derive(Debug, Clone)]
pub struct UnavailableBackend {
    reason: String,
}

impl UnavailableBackend {
    #[must_use]
    pub fn new(reason: impl Into<String>) -> Self {
        Self {
            reason: reason.into(),
        }
    }
}

impl AiBackend for UnavailableBackend {
    fn availability(&self) -> AiAvailability {
        AiAvailability::Unavailable {
            reason: self.reason.clone(),
        }
    }

    fn suggest(&self, _request: &AiRequest) -> Result<AiSuggestion, DevassistError> {
        Err(DevassistError::Unavailable(self.reason.clone()))
    }
}
```

  - `crates/bongterm-devassist/src/ai/mod.rs`:

```rust
//! AI assist submodule (Cmd-K + failed-command explainer).
pub(crate) const MODULE_NAME: &str = "ai";

pub mod runner;
pub use runner::{
    AiAvailability, AiBackend, AiContext, AiIntent, AiRequest, AiSuggestion, UnavailableBackend,
};
```

  - `crates/bongterm-test-kit/src/mocks/mod.rs`:

```rust
//! Mocks for devassist port traits.
pub mod ai_backend;
pub mod notifier;
```

  - `crates/bongterm-test-kit/src/mocks/ai_backend.rs`:

```rust
//! Scripted mock for `bongterm_devassist::ai::AiBackend`.

use bongterm_devassist::ai::{
    AiAvailability, AiBackend, AiRequest, AiSuggestion,
};
use bongterm_devassist::DevassistError;

/// A scripted AI backend for tests. Returns a fixed suggestion or unavailable.
pub struct MockAiBackend {
    availability: AiAvailability,
    suggestion: Option<AiSuggestion>,
}

impl MockAiBackend {
    /// Available backend that returns a scripted suggestion.
    #[must_use]
    pub fn available(suggestion: AiSuggestion) -> Self {
        Self {
            availability: AiAvailability::Available {
                version: "mock-1.0".to_string(),
            },
            suggestion: Some(suggestion),
        }
    }

    /// Unavailable backend (Claude Code not installed).
    #[must_use]
    pub fn unavailable(reason: impl Into<String>) -> Self {
        Self {
            availability: AiAvailability::Unavailable {
                reason: reason.into(),
            },
            suggestion: None,
        }
    }
}

impl AiBackend for MockAiBackend {
    fn availability(&self) -> AiAvailability {
        self.availability.clone()
    }

    fn suggest(&self, _request: &AiRequest) -> Result<AiSuggestion, DevassistError> {
        match &self.suggestion {
            Some(s) => Ok(s.clone()),
            None => Err(DevassistError::Unavailable("mock unavailable".to_string())),
        }
    }
}
```

  - Add the `bongterm-devassist` path dep to `crates/bongterm-test-kit/Cargo.toml` `[dependencies]` (append): `bongterm-devassist = { path = "../bongterm-devassist" }`. Add `pub mod mocks;` to `crates/bongterm-test-kit/src/lib.rs`. Add a placeholder `crates/bongterm-test-kit/src/mocks/notifier.rs` (filled in task 3.D.1):

```rust
//! Placeholder; `MockNotifier` is implemented in task 3.D.1.
```

  - Add the test-kit edge in `tools/xtask/allowed-deps.toml` — append `"bongterm-devassist",` to the `[bongterm-test-kit]` `allowed = [ ... ]` list. (Note: `bongterm-test-kit` depends on `bongterm-devassist`, not the reverse — this does not violate the hot-path rule because test-kit is never imported by production crates.)

> **Cycle note**: `bongterm-devassist` has a dev-dependency on `bongterm-test-kit`, and `bongterm-test-kit` has a normal dependency on `bongterm-devassist`. Cargo permits this because the edge from devassist is dev-only. If `cargo` reports a cycle, move the `MockAiBackend` consumer tests in devassist to a `tests/` integration directory (still dev-only) — the dependency direction is unchanged. Verify with `cargo test -p bongterm-devassist` after wiring.

- [ ] **(4) Run, expect PASS**: `cargo test -p bongterm-devassist ai::runner` → 2 passed. Also `cargo build -p bongterm-test-kit`.
- [ ] **(5) Commit**: `git add crates/bongterm-devassist crates/bongterm-test-kit tools/xtask/allowed-deps.toml && git commit -m "feat(devassist/3.A.1): AiBackend port + UnavailableBackend + MockAiBackend"`

---

### 3.A.2 — `CmdKSession`: preview-only state machine + explicit Run confirm (gate #9)

- [ ] **Files**
  - Create `crates/bongterm-devassist/src/ai/cmdk.rs`.
  - Modify `crates/bongterm-devassist/src/ai/mod.rs`: `pub mod cmdk; pub use cmdk::*;`.

- [ ] **(1) Failing test** — create `crates/bongterm-devassist/src/ai/cmdk.rs` with:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::runner::{AiContext, AiSuggestion};
    use bongterm_test_kit::mocks::ai_backend::MockAiBackend;

    fn ctx() -> AiContext {
        AiContext {
            cwd: "C:\\proj".to_string(),
            shell: "pwsh".to_string(),
            failed_command: None,
            transcript_tail: String::new(),
        }
    }

    #[test]
    fn fresh_session_has_no_runnable_command() {
        let backend = MockAiBackend::available(AiSuggestion {
            command: "Get-ChildItem | Sort-Object Length".to_string(),
            explanation: "lists files by size".to_string(),
        });
        let session = CmdKSession::new(Box::new(backend));
        // Nothing previewed yet → confirm_run is rejected.
        assert!(matches!(
            session.confirm_run(),
            Err(CmdKError::NothingToRun)
        ));
    }

    #[test]
    fn preview_does_not_mark_runnable_until_confirmed() {
        let backend = MockAiBackend::available(AiSuggestion {
            command: "ls -la".to_string(),
            explanation: "list".to_string(),
        });
        let mut session = CmdKSession::new(Box::new(backend));
        let preview = session
            .request_preview("list files", ctx())
            .expect("preview should succeed");
        // Preview is shown but state is Previewed, NOT Confirmed.
        assert_eq!(preview.command, "ls -la");
        assert_eq!(session.state(), CmdKState::Previewed);
        // confirm_run transitions to Confirmed and yields the command to run.
        let cmd = session.confirm_run().expect("confirm should succeed");
        assert_eq!(cmd, "ls -la");
        assert_eq!(session.state(), CmdKState::Confirmed);
    }

    #[test]
    fn unavailable_backend_yields_unavailable_state() {
        let backend = MockAiBackend::unavailable("Claude Code not installed");
        let mut session = CmdKSession::new(Box::new(backend));
        let err = session.request_preview("anything", ctx()).unwrap_err();
        assert!(matches!(err, CmdKError::Unavailable(_)));
        assert_eq!(session.state(), CmdKState::Unavailable);
        // Confirm is impossible in Unavailable state.
        assert!(matches!(session.confirm_run(), Err(CmdKError::NothingToRun)));
    }
}
```

- [ ] **(2) Run, expect FAIL**: `cargo test -p bongterm-devassist ai::cmdk`
  - Expected: `cannot find type 'CmdKSession'` / `CmdKState` / `CmdKError` unresolved.

- [ ] **(3) Minimal impl** — prepend to `crates/bongterm-devassist/src/ai/cmdk.rs`:

```rust
//! Cmd-K session: natural-language → command, PREVIEW-ONLY.
//!
//! Gate #9: the suggested command is never auto-executed. The session enforces
//! an explicit `confirm_run()` call before the command is released for spawn.
//! The session itself never spawns a process; it returns the command string to
//! the UI/composition layer, which routes execution through the normal PTY
//! path under policy.

use crate::ai::runner::{AiBackend, AiContext, AiIntent, AiRequest, AiSuggestion};

/// Lifecycle state of a Cmd-K session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CmdKState {
    /// No preview requested yet.
    Idle,
    /// A preview is shown; awaiting explicit confirmation.
    Previewed,
    /// User explicitly confirmed; the command may be released for execution.
    Confirmed,
    /// Backend unavailable (e.g. Claude Code not installed).
    Unavailable,
}

/// Errors from the Cmd-K session.
#[derive(Debug, thiserror::Error)]
pub enum CmdKError {
    #[error("AI assist unavailable: {0}")]
    Unavailable(String),
    #[error("backend error: {0}")]
    Backend(String),
    #[error("nothing to run: no confirmed preview")]
    NothingToRun,
}

/// A preview-only Cmd-K session over an [`AiBackend`].
pub struct CmdKSession {
    backend: Box<dyn AiBackend>,
    state: CmdKState,
    last_suggestion: Option<AiSuggestion>,
}

impl CmdKSession {
    #[must_use]
    pub fn new(backend: Box<dyn AiBackend>) -> Self {
        Self {
            backend,
            state: CmdKState::Idle,
            last_suggestion: None,
        }
    }

    #[must_use]
    pub fn state(&self) -> CmdKState {
        self.state
    }

    /// Request a preview-only suggestion. Does NOT execute anything.
    pub fn request_preview(
        &mut self,
        user_text: impl Into<String>,
        context: AiContext,
    ) -> Result<AiSuggestion, CmdKError> {
        let request = AiRequest {
            intent: AiIntent::NlToCommand,
            user_text: user_text.into(),
            context,
        };
        match self.backend.suggest(&request) {
            Ok(suggestion) => {
                self.last_suggestion = Some(suggestion.clone());
                self.state = CmdKState::Previewed;
                Ok(suggestion)
            }
            Err(crate::DevassistError::Unavailable(reason)) => {
                self.state = CmdKState::Unavailable;
                self.last_suggestion = None;
                Err(CmdKError::Unavailable(reason))
            }
            Err(other) => {
                self.state = CmdKState::Idle;
                Err(CmdKError::Backend(other.to_string()))
            }
        }
    }

    /// Explicit Run confirmation. Returns the command string ONLY when a
    /// preview is present and the user confirmed. Never auto-called.
    pub fn confirm_run(&mut self) -> Result<String, CmdKError> {
        match (self.state, &self.last_suggestion) {
            (CmdKState::Previewed, Some(s)) => {
                let cmd = s.command.clone();
                self.state = CmdKState::Confirmed;
                Ok(cmd)
            }
            _ => Err(CmdKError::NothingToRun),
        }
    }
}
```

  - `crates/bongterm-devassist/src/ai/mod.rs` — add:

```rust
pub mod cmdk;
pub use cmdk::{CmdKError, CmdKSession, CmdKState};
```

- [ ] **(4) Run, expect PASS**: `cargo test -p bongterm-devassist ai::cmdk` → 3 passed.
- [ ] **(5) Commit**: `git add crates/bongterm-devassist && git commit -m "feat(devassist/3.A.2): CmdKSession preview-only state machine + explicit confirm"`

---

### 3.A.3 — `Explainer`: failed-command explainer on non-zero-exit blocks (gate #10)

- [ ] **Files**
  - Create `crates/bongterm-devassist/src/ai/explainer.rs`.
  - Modify `crates/bongterm-devassist/src/ai/mod.rs`: `pub mod explainer; pub use explainer::*;`.

- [ ] **(1) Failing test** — create `crates/bongterm-devassist/src/ai/explainer.rs` with:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use bongterm_storage_api::{BlockId, CommandBlockRow, PaneId, SessionId};
    use bongterm_test_kit::mocks::ai_backend::MockAiBackend;
    use crate::ai::runner::AiSuggestion;
    use uuid::Uuid;

    fn failed_block(exit: i64, cmd: &str) -> CommandBlockRow {
        CommandBlockRow {
            id: BlockId(Uuid::nil()),
            pane_id: PaneId(Uuid::nil()),
            session_id: SessionId(Uuid::nil()),
            command: cmd.to_string(),
            exit_code: Some(exit),
            started_at: time::OffsetDateTime::UNIX_EPOCH,
            finished_at: Some(time::OffsetDateTime::UNIX_EPOCH),
        }
    }

    #[test]
    fn explainer_is_offered_only_for_nonzero_exit() {
        assert!(Explainer::is_explainable(&failed_block(1, "cargo build")));
        assert!(Explainer::is_explainable(&failed_block(127, "frobnicate")));
        assert!(!Explainer::is_explainable(&failed_block(0, "cargo build")));
    }

    #[test]
    fn explainer_builds_context_from_block_and_transcript() {
        let backend = MockAiBackend::available(AiSuggestion {
            command: String::new(),
            explanation: "command not found / not in PATH".to_string(),
        });
        let explainer = Explainer::new(Box::new(backend));
        let block = failed_block(127, "frobnicate --help");
        let result = explainer
            .explain(&block, "frobnicate: command not found")
            .expect("explain should succeed");
        assert!(result.explanation.contains("not found"));
    }

    #[test]
    fn explainer_refuses_zero_exit() {
        let backend = MockAiBackend::available(AiSuggestion {
            command: String::new(),
            explanation: "n/a".to_string(),
        });
        let explainer = Explainer::new(Box::new(backend));
        let block = failed_block(0, "ls");
        assert!(matches!(
            explainer.explain(&block, "ok"),
            Err(crate::DevassistError::Parse(_))
        ));
    }
}
```

- [ ] **(2) Run, expect FAIL**: `cargo test -p bongterm-devassist ai::explainer`
  - Expected: `cannot find type 'Explainer'`.

- [ ] **(3) Minimal impl** — prepend to `crates/bongterm-devassist/src/ai/explainer.rs`:

```rust
//! Failed-command explainer (gate #10).
//!
//! Offered only on command blocks with a non-zero exit code. Builds a bounded
//! context from the block + a tail of its output and asks the AI backend for an
//! explanation. Preview-only: it produces text, never executes a fix.

use crate::ai::runner::{AiBackend, AiContext, AiIntent, AiRequest, AiSuggestion};
use crate::DevassistError;
use bongterm_storage_api::CommandBlockRow;

/// Maximum transcript-tail length handed to the backend (bounded context).
const MAX_TAIL: usize = 4096;

/// Builds and dispatches failed-command explanations.
pub struct Explainer {
    backend: Box<dyn AiBackend>,
}

impl Explainer {
    #[must_use]
    pub fn new(backend: Box<dyn AiBackend>) -> Self {
        Self { backend }
    }

    /// A block is explainable iff it finished with a non-zero exit code.
    #[must_use]
    pub fn is_explainable(block: &CommandBlockRow) -> bool {
        matches!(block.exit_code, Some(code) if code != 0)
    }

    /// Produce an explanation for a failed command block.
    pub fn explain(
        &self,
        block: &CommandBlockRow,
        output_tail: &str,
    ) -> Result<AiSuggestion, DevassistError> {
        if !Self::is_explainable(block) {
            return Err(DevassistError::Parse(
                "block did not fail; nothing to explain".to_string(),
            ));
        }
        let tail = if output_tail.len() > MAX_TAIL {
            &output_tail[output_tail.len() - MAX_TAIL..]
        } else {
            output_tail
        };
        let request = AiRequest {
            intent: AiIntent::ExplainFailure,
            user_text: format!(
                "Command `{}` exited with code {}. Explain why and suggest a fix.",
                block.command,
                block.exit_code.unwrap_or_default()
            ),
            context: AiContext {
                cwd: String::new(),
                shell: String::new(),
                failed_command: Some(block.command.clone()),
                transcript_tail: tail.to_string(),
            },
        };
        self.backend.suggest(&request)
    }
}
```

  - `crates/bongterm-devassist/src/ai/mod.rs` — add:

```rust
pub mod explainer;
pub use explainer::Explainer;
```

- [ ] **(4) Run, expect PASS**: `cargo test -p bongterm-devassist ai::explainer` → 3 passed.
- [ ] **(5) Commit**: `git add crates/bongterm-devassist && git commit -m "feat(devassist/3.A.3): failed-command Explainer on non-zero-exit blocks"`

---

### 3.A.4 — `ClaudeCodeAiRunner`: real subprocess wrapper + graceful "not installed" fallback (gates #9, #10)

- [ ] **Files**
  - Modify `crates/bongterm-devassist/src/ai/runner.rs`: add `ClaudeCodeAiRunner` + `ClaudeProbe` port + `detect_backend`.

- [ ] **(1) Failing test** — append to the `tests` module in `crates/bongterm-devassist/src/ai/runner.rs`:

```rust
    struct FakeProbeFound;
    impl ClaudeProbe for FakeProbeFound {
        fn locate(&self) -> Option<ClaudeInfo> {
            Some(ClaudeInfo {
                binary: "claude".to_string(),
                version: "1.2.3".to_string(),
            })
        }
    }

    struct FakeProbeMissing;
    impl ClaudeProbe for FakeProbeMissing {
        fn locate(&self) -> Option<ClaudeInfo> {
            None
        }
    }

    #[test]
    fn detect_backend_available_when_probe_finds_claude() {
        let backend = detect_backend(&FakeProbeFound);
        assert!(matches!(
            backend.availability(),
            AiAvailability::Available { version } if version == "1.2.3"
        ));
    }

    #[test]
    fn detect_backend_unavailable_when_claude_missing() {
        let backend = detect_backend(&FakeProbeMissing);
        match backend.availability() {
            AiAvailability::Unavailable { reason } => {
                assert!(reason.to_lowercase().contains("claude code not installed"));
            }
            AiAvailability::Available { .. } => panic!("expected unavailable"),
        }
    }

    #[test]
    fn claude_runner_parses_json_print_output() {
        // The runner's parser is unit-testable without spawning a process.
        let stdout = r#"{"type":"result","result":"Get-ChildItem | Sort-Object Length"}"#;
        let suggestion = ClaudeCodeAiRunner::parse_print_json(stdout, AiIntent::NlToCommand)
            .expect("parse should succeed");
        assert_eq!(suggestion.command, "Get-ChildItem | Sort-Object Length");
    }

    #[test]
    fn claude_runner_explain_intent_puts_text_in_explanation() {
        let stdout = r#"{"type":"result","result":"exit 127 means the command was not found in PATH"}"#;
        let suggestion =
            ClaudeCodeAiRunner::parse_print_json(stdout, AiIntent::ExplainFailure).unwrap();
        assert!(suggestion.explanation.contains("not found in PATH"));
        assert!(suggestion.command.is_empty());
    }

    #[test]
    fn claude_runner_rejects_malformed_json() {
        let err = ClaudeCodeAiRunner::parse_print_json("not json", AiIntent::NlToCommand)
            .unwrap_err();
        assert!(matches!(err, DevassistError::Parse(_)));
    }
```

- [ ] **(2) Run, expect FAIL**: `cargo test -p bongterm-devassist ai::runner`
  - Expected: `cannot find type 'ClaudeProbe'` / `ClaudeCodeAiRunner` / `detect_backend` unresolved.

- [ ] **(3) Minimal impl** — append to `crates/bongterm-devassist/src/ai/runner.rs` (above the test module):

```rust
/// Discovered Claude Code binary + version.
#[derive(Debug, Clone)]
pub struct ClaudeInfo {
    pub binary: String,
    pub version: String,
}

/// Port that locates the Claude Code CLI. Real impl probes PATH + `--version`;
/// tests inject a fake. Isolating discovery keeps `detect_backend` pure.
pub trait ClaudeProbe {
    fn locate(&self) -> Option<ClaudeInfo>;
}

/// Builds the appropriate backend from a probe result. When Claude Code is not
/// found, returns the graceful `UnavailableBackend` with a clear message
/// (gates #9/#10 "not installed" path) — nothing is silently substituted.
#[must_use]
pub fn detect_backend(probe: &dyn ClaudeProbe) -> Box<dyn AiBackend> {
    match probe.locate() {
        Some(info) => Box::new(ClaudeCodeAiRunner::new(info)),
        None => Box::new(UnavailableBackend::new(
            "Claude Code not installed. Install the Claude Code CLI to enable Cmd-K and the failed-command explainer.",
        )),
    }
}

/// Wraps the Claude Code CLI in non-interactive mode:
/// `claude --print --output-format json --prompt <context>`.
#[derive(Debug, Clone)]
pub struct ClaudeCodeAiRunner {
    info: ClaudeInfo,
}

impl ClaudeCodeAiRunner {
    #[must_use]
    pub fn new(info: ClaudeInfo) -> Self {
        Self { info }
    }

    /// Parse the `--output-format json` stdout of a `claude --print` run.
    /// Pure and unit-testable; no process spawn.
    pub fn parse_print_json(
        stdout: &str,
        intent: AiIntent,
    ) -> Result<AiSuggestion, DevassistError> {
        let value: serde_json::Value = serde_json::from_str(stdout.trim())
            .map_err(|e| DevassistError::Parse(format!("claude json: {e}")))?;
        let result = value
            .get("result")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| DevassistError::Parse("missing `result` field".to_string()))?
            .to_string();
        Ok(match intent {
            AiIntent::NlToCommand => AiSuggestion {
                command: result,
                explanation: String::new(),
            },
            AiIntent::ExplainFailure => AiSuggestion {
                command: String::new(),
                explanation: result,
            },
        })
    }

    /// Build the argv for a non-interactive Claude Code invocation.
    #[must_use]
    pub fn build_argv(&self, prompt: &str) -> Vec<String> {
        vec![
            self.info.binary.clone(),
            "--print".to_string(),
            "--output-format".to_string(),
            "json".to_string(),
            "--prompt".to_string(),
            prompt.to_string(),
        ]
    }
}

impl AiBackend for ClaudeCodeAiRunner {
    fn availability(&self) -> AiAvailability {
        AiAvailability::Available {
            version: self.info.version.clone(),
        }
    }

    fn suggest(&self, request: &AiRequest) -> Result<AiSuggestion, DevassistError> {
        // Spawn `claude --print` synchronously off the hot path. The caller
        // invokes this on a background task, never the hot path or UI thread.
        let prompt = format!(
            "{}\ncwd: {}\nshell: {}\n{}",
            request.user_text,
            request.context.cwd,
            request.context.shell,
            request.context.transcript_tail
        );
        let argv = self.build_argv(&prompt);
        let output = std::process::Command::new(&argv[0])
            .args(&argv[1..])
            .output()
            .map_err(|e| DevassistError::Backend(format!("spawn claude: {e}")))?;
        if !output.status.success() {
            return Err(DevassistError::Backend(format!(
                "claude exited with {}",
                output.status
            )));
        }
        let stdout = String::from_utf8_lossy(&output.stdout);
        Self::parse_print_json(&stdout, request.intent)
    }
}
```

- [ ] **(4) Run, expect PASS**: `cargo test -p bongterm-devassist ai::runner` → all passed.
- [ ] **(5) Commit**: `git add crates/bongterm-devassist && git commit -m "feat(devassist/3.A.4): ClaudeCodeAiRunner subprocess wrapper + not-installed fallback"`

---

## Task Group 3.B — Smart history (filters + frecency + Ctrl+R) → gate #11

### 3.B.1 — `HistoryFilter` parser: `cwd:` `branch:` `agent:` `exit:` `time:` `shell:` `duration:`

- [ ] **Files**
  - Create `crates/bongterm-devassist/src/history/filter.rs`.
  - Modify `crates/bongterm-devassist/src/history/mod.rs`: `pub mod filter; pub use filter::*;`.

- [ ] **(1) Failing test** — create `crates/bongterm-devassist/src/history/filter.rs` with:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_each_filter_kind() {
        let q = HistoryQuery::parse("cwd:C:\\proj branch:main agent:claude exit:1 time:24h shell:pwsh duration:>5s build");
        assert_eq!(q.free_text, "build");
        assert_eq!(q.filter(FilterKind::Cwd), Some("C:\\proj"));
        assert_eq!(q.filter(FilterKind::Branch), Some("main"));
        assert_eq!(q.filter(FilterKind::Agent), Some("claude"));
        assert_eq!(q.filter(FilterKind::Exit), Some("1"));
        assert_eq!(q.filter(FilterKind::Time), Some("24h"));
        assert_eq!(q.filter(FilterKind::Shell), Some("pwsh"));
        assert_eq!(q.filter(FilterKind::Duration), Some(">5s"));
    }

    #[test]
    fn unknown_prefix_stays_free_text() {
        let q = HistoryQuery::parse("foo:bar cargo");
        assert_eq!(q.free_text, "foo:bar cargo");
        assert_eq!(q.filter(FilterKind::Cwd), None);
    }

    #[test]
    fn matches_applies_all_filters_conjunctively() {
        let q = HistoryQuery::parse("shell:pwsh exit:0 build");
        let entry = HistoryEntryMeta {
            command: "cargo build".to_string(),
            cwd: "C:\\proj".to_string(),
            branch: Some("main".to_string()),
            agent: None,
            exit_code: Some(0),
            shell: "pwsh".to_string(),
            duration_secs: 12.0,
            age_secs: 60,
        };
        assert!(q.matches(&entry));

        let q2 = HistoryQuery::parse("shell:cmd build");
        assert!(!q2.matches(&entry));
    }

    #[test]
    fn duration_and_time_comparators() {
        let entry = HistoryEntryMeta {
            command: "sleep".to_string(),
            cwd: String::new(),
            branch: None,
            agent: None,
            exit_code: Some(0),
            shell: "bash".to_string(),
            duration_secs: 10.0,
            age_secs: 3600,
        };
        assert!(HistoryQuery::parse("duration:>5s").matches(&entry));
        assert!(!HistoryQuery::parse("duration:>5m").matches(&entry));
        assert!(HistoryQuery::parse("time:24h").matches(&entry));
        assert!(!HistoryQuery::parse("time:30m").matches(&entry));
    }
}
```

- [ ] **(2) Run, expect FAIL**: `cargo test -p bongterm-devassist history::filter`
  - Expected: `cannot find type 'HistoryQuery'` / `FilterKind` / `HistoryEntryMeta` unresolved.

- [ ] **(3) Minimal impl** — prepend to `crates/bongterm-devassist/src/history/filter.rs`:

```rust
//! Smart-history filter parsing (gate #11).
//!
//! Supported filters: `cwd:` `branch:` `agent:` `exit:` `time:` `shell:`
//! `duration:`. Parsing runs OFF the hot path. The closed `FilterKind` enum
//! makes the set exhaustive (SOLID: bounded set => enum + match).

/// Metadata about a history entry, used for matching.
#[derive(Debug, Clone)]
pub struct HistoryEntryMeta {
    pub command: String,
    pub cwd: String,
    pub branch: Option<String>,
    pub agent: Option<String>,
    pub exit_code: Option<i64>,
    pub shell: String,
    pub duration_secs: f64,
    /// How long ago the command ran, in seconds.
    pub age_secs: u64,
}

/// The closed set of supported smart-history filters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FilterKind {
    Cwd,
    Branch,
    Agent,
    Exit,
    Time,
    Shell,
    Duration,
}

impl FilterKind {
    /// The textual prefix (without the trailing colon).
    #[must_use]
    pub fn prefix(self) -> &'static str {
        match self {
            FilterKind::Cwd => "cwd",
            FilterKind::Branch => "branch",
            FilterKind::Agent => "agent",
            FilterKind::Exit => "exit",
            FilterKind::Time => "time",
            FilterKind::Shell => "shell",
            FilterKind::Duration => "duration",
        }
    }

    const ALL: [FilterKind; 7] = [
        FilterKind::Cwd,
        FilterKind::Branch,
        FilterKind::Agent,
        FilterKind::Exit,
        FilterKind::Time,
        FilterKind::Shell,
        FilterKind::Duration,
    ];
}

/// A parsed smart-history query: extracted filters + remaining free text.
#[derive(Debug, Clone, Default)]
pub struct HistoryQuery {
    filters: Vec<(FilterKind, String)>,
    pub free_text: String,
}

impl HistoryQuery {
    /// Parse a raw query string. Tokens of the form `<prefix>:<value>` where
    /// `<prefix>` is a known [`FilterKind`] become filters; everything else is
    /// joined back into `free_text`.
    #[must_use]
    pub fn parse(input: &str) -> Self {
        let mut filters = Vec::new();
        let mut free = Vec::new();
        for tok in input.split_whitespace() {
            if let Some((pre, val)) = tok.split_once(':') {
                if let Some(kind) = FilterKind::ALL.iter().copied().find(|k| k.prefix() == pre) {
                    if !val.is_empty() {
                        filters.push((kind, val.to_string()));
                        continue;
                    }
                }
            }
            free.push(tok);
        }
        Self {
            filters,
            free_text: free.join(" "),
        }
    }

    /// The value for a given filter, if present.
    #[must_use]
    pub fn filter(&self, kind: FilterKind) -> Option<&str> {
        self.filters
            .iter()
            .find(|(k, _)| *k == kind)
            .map(|(_, v)| v.as_str())
    }

    /// Whether an entry satisfies all filters (conjunctive) and free text.
    #[must_use]
    pub fn matches(&self, e: &HistoryEntryMeta) -> bool {
        if !self.free_text.is_empty() && !e.command.contains(&self.free_text) {
            return false;
        }
        self.filters.iter().all(|(kind, val)| match kind {
            FilterKind::Cwd => e.cwd.contains(val.as_str()),
            FilterKind::Branch => e.branch.as_deref() == Some(val.as_str()),
            FilterKind::Agent => e.agent.as_deref() == Some(val.as_str()),
            FilterKind::Exit => e.exit_code.map(|c| c.to_string() == *val).unwrap_or(false),
            FilterKind::Shell => e.shell == *val,
            FilterKind::Time => parse_window_secs(val).map_or(false, |w| e.age_secs <= w),
            FilterKind::Duration => match_duration(val, e.duration_secs),
        })
    }
}

/// Parse a window like `24h`, `30m`, `45s` into seconds.
fn parse_window_secs(s: &str) -> Option<u64> {
    let (num, unit) = s.split_at(s.len().checked_sub(1)?);
    let n: u64 = num.parse().ok()?;
    match unit {
        "s" => Some(n),
        "m" => Some(n * 60),
        "h" => Some(n * 3600),
        "d" => Some(n * 86400),
        _ => None,
    }
}

/// Match a duration spec like `>5s`, `<5m`, `>=10s` against a value in seconds.
fn match_duration(spec: &str, value_secs: f64) -> bool {
    let (op, rest) = if let Some(r) = spec.strip_prefix(">=") {
        (">=", r)
    } else if let Some(r) = spec.strip_prefix("<=") {
        ("<=", r)
    } else if let Some(r) = spec.strip_prefix('>') {
        (">", r)
    } else if let Some(r) = spec.strip_prefix('<') {
        ("<", r)
    } else {
        ("==", spec)
    };
    let Some(threshold) = parse_window_secs(rest) else {
        return false;
    };
    let t = threshold as f64;
    match op {
        ">" => value_secs > t,
        "<" => value_secs < t,
        ">=" => value_secs >= t,
        "<=" => value_secs <= t,
        _ => (value_secs - t).abs() < f64::EPSILON,
    }
}
```

  - `crates/bongterm-devassist/src/history/mod.rs`:

```rust
//! Smart history submodule (filters + frecency).
pub(crate) const MODULE_NAME: &str = "history";

pub mod filter;
pub use filter::{FilterKind, HistoryEntryMeta, HistoryQuery};
```

- [ ] **(4) Run, expect PASS**: `cargo test -p bongterm-devassist history::filter` → 4 passed.
- [ ] **(5) Commit**: `git add crates/bongterm-devassist && git commit -m "feat(devassist/3.B.1): smart-history filter parser (cwd/branch/agent/exit/time/shell/duration)"`

---

### 3.B.2 — `FrecencyRepo` port (storage-api) + `MockFrecencyRepo` + conformance

- [ ] **Files**
  - Modify `crates/bongterm-storage-api/src/lib.rs`: add `FrecencyRow` + `FrecencyRepo` + `frecency_score`.
  - Create `crates/bongterm-test-kit/src/conformance/frecency_repo_conformance.rs`.
  - Modify `crates/bongterm-test-kit/src/conformance/mod.rs`: `pub mod frecency_repo_conformance;`.

- [ ] **(1) Failing test** — create `crates/bongterm-test-kit/src/conformance/frecency_repo_conformance.rs`:

```rust
//! Conformance for `bongterm_storage_api::FrecencyRepo` + an in-memory mock.

use bongterm_storage_api::{FrecencyRepo, FrecencyRow, StorageError};
use std::collections::HashMap;
use std::sync::Mutex;

/// In-memory mock for [`FrecencyRepo`].
pub struct MockFrecencyRepo {
    store: Mutex<HashMap<String, FrecencyRow>>,
}

impl MockFrecencyRepo {
    #[must_use]
    pub fn new() -> Self {
        Self {
            store: Mutex::new(HashMap::new()),
        }
    }
}

impl Default for MockFrecencyRepo {
    fn default() -> Self {
        Self::new()
    }
}

impl FrecencyRepo for MockFrecencyRepo {
    fn record_use(&self, command: &str, at_unix: i64) -> Result<(), StorageError> {
        let mut g = self.store.lock().unwrap();
        let row = g.entry(command.to_string()).or_insert(FrecencyRow {
            command: command.to_string(),
            use_count: 0,
            last_used_unix: at_unix,
        });
        row.use_count += 1;
        row.last_used_unix = at_unix;
        Ok(())
    }

    fn top_n(&self, n: usize, now_unix: i64) -> Result<Vec<FrecencyRow>, StorageError> {
        let g = self.store.lock().unwrap();
        let mut rows: Vec<FrecencyRow> = g.values().cloned().collect();
        rows.sort_by(|a, b| {
            let sa = bongterm_storage_api::frecency_score(a, now_unix);
            let sb = bongterm_storage_api::frecency_score(b, now_unix);
            sb.partial_cmp(&sa).unwrap_or(std::cmp::Ordering::Equal)
        });
        rows.truncate(n);
        Ok(rows)
    }
}

/// Run happy-path conformance against any [`FrecencyRepo`].
///
/// # Panics
/// Panics on contract violation.
pub fn run_frecency_repo_conformance(repo: &dyn FrecencyRepo) {
    repo.record_use("cargo build", 1000).unwrap();
    repo.record_use("cargo build", 2000).unwrap();
    repo.record_use("git status", 1500).unwrap();
    let top = repo.top_n(10, 3000).unwrap();
    assert!(!top.is_empty(), "top_n must return recorded entries");
    assert_eq!(top[0].command, "cargo build");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mock_satisfies_conformance() {
        run_frecency_repo_conformance(&MockFrecencyRepo::new());
    }
}
```

- [ ] **(2) Run, expect FAIL**: `cargo test -p bongterm-test-kit frecency`
  - Expected: `cannot find type 'FrecencyRow'` / `FrecencyRepo` / `frecency_score` in `bongterm_storage_api`.

- [ ] **(3) Minimal impl** — append to `crates/bongterm-storage-api/src/lib.rs` after the existing repo traits, before the `#[cfg(test)]` module:

```rust
// ---------------------------------------------------------------------------
// Frecency (smart-history ranking)
// ---------------------------------------------------------------------------

/// A frecency record for one command string.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FrecencyRow {
    pub command: String,
    pub use_count: u64,
    pub last_used_unix: i64,
}

/// Combined recency + frequency score. Higher is more relevant.
///
/// Frequency contributes logarithmically; recency decays with elapsed time.
/// Pure function so the SQLite impl and the mock rank identically.
#[must_use]
pub fn frecency_score(row: &FrecencyRow, now_unix: i64) -> f64 {
    let freq = (1.0 + row.use_count as f64).ln();
    let age_secs = (now_unix - row.last_used_unix).max(0) as f64;
    let recency = 0.5_f64.powf(age_secs / 86_400.0); // ~1-day half-life
    freq * (0.5 + recency)
}

/// Record command uses and retrieve frecency-ranked history.
pub trait FrecencyRepo: Send + Sync + 'static {
    /// Record one use of `command` at `at_unix` (seconds since epoch).
    fn record_use(&self, command: &str, at_unix: i64) -> Result<(), StorageError>;
    /// Return the top `n` commands by frecency score as of `now_unix`.
    fn top_n(&self, n: usize, now_unix: i64) -> Result<Vec<FrecencyRow>, StorageError>;
}
```

  - `crates/bongterm-test-kit/src/conformance/mod.rs` — add `pub mod frecency_repo_conformance;`.

- [ ] **(4) Run, expect PASS**: `cargo test -p bongterm-test-kit frecency` → passed; `cargo test -p bongterm-storage-api` still green.
- [ ] **(5) Commit**: `git add crates/bongterm-storage-api crates/bongterm-test-kit && git commit -m "feat(storage/3.B.2): FrecencyRepo port + frecency_score + MockFrecencyRepo conformance"`

---

### 3.B.3 — `SqliteFrecencyRepo` impl + `0002_frecency` migration

- [ ] **Files**
  - Modify `crates/bongterm-storage-sqlite/Cargo.toml`: add `bongterm-storage-api`, `rusqlite`, `time` deps + dev-dep `bongterm-test-kit`.
  - Modify `crates/bongterm-storage-sqlite/src/lib.rs`: add `SqliteFrecencyRepo` + migration SQL.

- [ ] **(1) Failing test** — append to `crates/bongterm-storage-sqlite/src/lib.rs` a test module:

```rust
#[cfg(test)]
mod frecency_tests {
    use super::*;
    use bongterm_test_kit::conformance::frecency_repo_conformance::run_frecency_repo_conformance;

    #[test]
    fn sqlite_frecency_satisfies_conformance() {
        let repo = SqliteFrecencyRepo::open_in_memory().expect("open in-memory db");
        run_frecency_repo_conformance(&repo);
    }

    #[test]
    fn record_use_increments_count() {
        let repo = SqliteFrecencyRepo::open_in_memory().unwrap();
        repo.record_use("ls", 100).unwrap();
        repo.record_use("ls", 200).unwrap();
        let rows = repo.top_n(5, 300).unwrap();
        let ls = rows.iter().find(|r| r.command == "ls").unwrap();
        assert_eq!(ls.use_count, 2);
        assert_eq!(ls.last_used_unix, 200);
    }
}
```

- [ ] **(2) Run, expect FAIL**: `cargo test -p bongterm-storage-sqlite frecency`
  - Expected: `cannot find type 'SqliteFrecencyRepo'`.

- [ ] **(3) Minimal impl**
  - `crates/bongterm-storage-sqlite/Cargo.toml` `[dependencies]`:

```toml
bongterm-storage-api = { path = "../bongterm-storage-api" }
rusqlite = { workspace = true }
time = { workspace = true }
```

  - `[dev-dependencies]`: `bongterm-test-kit = { path = "../bongterm-test-kit" }`.
  - Replace `crates/bongterm-storage-sqlite/src/lib.rs` with:

```rust
//! SQLite (WAL) implementation of `bongterm-storage-api` repository traits.
//!
//! See spec §3.8. Only `bongterm-app` and repository crates may depend on this.

#![deny(unsafe_code)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::missing_errors_doc)]

use bongterm_storage_api::{FrecencyRepo, FrecencyRow, StorageError};
use rusqlite::Connection;
use std::sync::Mutex;

/// Frecency migration. Idempotent via `IF NOT EXISTS`.
const MIGRATION_0002_FRECENCY: &str = "\
CREATE TABLE IF NOT EXISTS frecency (
    command         TEXT PRIMARY KEY,
    use_count       INTEGER NOT NULL DEFAULT 0,
    last_used_unix  INTEGER NOT NULL
);";

/// SQLite-backed [`FrecencyRepo`].
pub struct SqliteFrecencyRepo {
    conn: Mutex<Connection>,
}

impl SqliteFrecencyRepo {
    /// Open an in-memory DB (tests).
    pub fn open_in_memory() -> Result<Self, StorageError> {
        let conn =
            Connection::open_in_memory().map_err(|e| StorageError::Database(e.to_string()))?;
        Self::from_conn(conn)
    }

    /// Wrap an existing connection and ensure the schema exists.
    pub fn from_conn(conn: Connection) -> Result<Self, StorageError> {
        conn.execute_batch(MIGRATION_0002_FRECENCY)
            .map_err(|e| StorageError::Database(e.to_string()))?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }
}

impl FrecencyRepo for SqliteFrecencyRepo {
    fn record_use(&self, command: &str, at_unix: i64) -> Result<(), StorageError> {
        let conn = self.conn.lock().expect("frecency mutex");
        conn.execute(
            "INSERT INTO frecency(command, use_count, last_used_unix)
             VALUES (?1, 1, ?2)
             ON CONFLICT(command) DO UPDATE SET
                use_count = use_count + 1,
                last_used_unix = ?2",
            rusqlite::params![command, at_unix],
        )
        .map_err(|e| StorageError::Database(e.to_string()))?;
        Ok(())
    }

    fn top_n(&self, n: usize, now_unix: i64) -> Result<Vec<FrecencyRow>, StorageError> {
        let conn = self.conn.lock().expect("frecency mutex");
        let mut stmt = conn
            .prepare("SELECT command, use_count, last_used_unix FROM frecency")
            .map_err(|e| StorageError::Database(e.to_string()))?;
        let rows = stmt
            .query_map([], |r| {
                Ok(FrecencyRow {
                    command: r.get(0)?,
                    use_count: r.get::<_, i64>(1)? as u64,
                    last_used_unix: r.get(2)?,
                })
            })
            .map_err(|e| StorageError::Database(e.to_string()))?;
        let mut all: Vec<FrecencyRow> = rows
            .collect::<Result<_, _>>()
            .map_err(|e| StorageError::Database(e.to_string()))?;
        all.sort_by(|a, b| {
            let sa = bongterm_storage_api::frecency_score(a, now_unix);
            let sb = bongterm_storage_api::frecency_score(b, now_unix);
            sb.partial_cmp(&sa).unwrap_or(std::cmp::Ordering::Equal)
        });
        all.truncate(n);
        Ok(all)
    }
}
```

- [ ] **(4) Run, expect PASS**: `cargo test -p bongterm-storage-sqlite frecency` → 2 passed.
- [ ] **(5) Commit**: `git add crates/bongterm-storage-sqlite && git commit -m "feat(storage/3.B.3): SqliteFrecencyRepo + 0002_frecency migration"`

---

### 3.B.4 — `SmartHistory`: Ctrl+R search combining filters + frecency (gate #11)

- [ ] **Files**
  - Create `crates/bongterm-devassist/src/history/frecency.rs`.
  - Modify `crates/bongterm-devassist/src/history/mod.rs`: add `pub mod frecency; pub use frecency::SmartHistory;`.

- [ ] **(1) Failing test** — create `crates/bongterm-devassist/src/history/frecency.rs` with:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::history::filter::HistoryEntryMeta;

    fn entry(cmd: &str, shell: &str, exit: i64, age: u64, dur: f64) -> HistoryEntryMeta {
        HistoryEntryMeta {
            command: cmd.to_string(),
            cwd: "C:\\proj".to_string(),
            branch: Some("main".to_string()),
            agent: None,
            exit_code: Some(exit),
            shell: shell.to_string(),
            duration_secs: dur,
            age_secs: age,
        }
    }

    #[test]
    fn ctrl_r_search_filters_then_ranks_by_frecency() {
        let entries = vec![
            (entry("cargo build", "pwsh", 0, 3600, 30.0), 2_u64),
            (entry("cargo test", "pwsh", 0, 60, 12.0), 5_u64),
            (entry("git push", "cmd", 0, 30, 1.0), 9_u64),
        ];
        let results = SmartHistory::search("shell:pwsh cargo", &entries, 4000);
        assert_eq!(results.len(), 2, "git push filtered out by shell:pwsh");
        assert_eq!(results[0].command, "cargo test");
        assert_eq!(results[1].command, "cargo build");
    }

    #[test]
    fn ctrl_r_empty_query_returns_all_ranked() {
        let entries = vec![
            (entry("a", "pwsh", 0, 10, 1.0), 1_u64),
            (entry("b", "pwsh", 0, 10, 1.0), 10_u64),
        ];
        let results = SmartHistory::search("", &entries, 100);
        assert_eq!(results[0].command, "b");
    }
}
```

- [ ] **(2) Run, expect FAIL**: `cargo test -p bongterm-devassist history::frecency`
  - Expected: `cannot find type 'SmartHistory'`.

- [ ] **(3) Minimal impl** — prepend to `crates/bongterm-devassist/src/history/frecency.rs`:

```rust
//! Ctrl+R smart history: filter then rank by frecency (gate #11).
//!
//! Pure ranking over already-loaded entries. The DB read happens via
//! `bongterm_storage_api::FrecencyRepo` off the hot path; this function ranks
//! the in-memory candidate set so it is deterministically testable.

use crate::history::filter::{HistoryEntryMeta, HistoryQuery};
use bongterm_storage_api::{frecency_score, FrecencyRow};

/// Smart-history search engine.
pub struct SmartHistory;

impl SmartHistory {
    /// Filter `entries` by the parsed query, then rank survivors by frecency.
    /// Each tuple is `(metadata, use_count)`.
    #[must_use]
    pub fn search(
        raw_query: &str,
        entries: &[(HistoryEntryMeta, u64)],
        now_unix: i64,
    ) -> Vec<HistoryEntryMeta> {
        let query = HistoryQuery::parse(raw_query);
        let mut matched: Vec<(&HistoryEntryMeta, f64)> = entries
            .iter()
            .filter(|(meta, _)| query.matches(meta))
            .map(|(meta, count)| {
                let row = FrecencyRow {
                    command: meta.command.clone(),
                    use_count: *count,
                    last_used_unix: now_unix - meta.age_secs as i64,
                };
                (meta, frecency_score(&row, now_unix))
            })
            .collect();
        matched.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        matched.into_iter().map(|(m, _)| m.clone()).collect()
    }
}
```

  - `crates/bongterm-devassist/src/history/mod.rs` — add `pub mod frecency; pub use frecency::SmartHistory;`.

- [ ] **(4) Run, expect PASS**: `cargo test -p bongterm-devassist history::frecency` → 2 passed.
- [ ] **(5) Commit**: `git add crates/bongterm-devassist && git commit -m "feat(devassist/3.B.4): SmartHistory Ctrl+R filter-then-frecency ranking"`

---

## Task Group 3.C — Snippets (JSON5 library + `${param:name}` + scope) → gate #12

### 3.C.1 — `Snippet` model + JSON5 library load + `${param:name}` parse

- [ ] **Files**
  - Create `crates/bongterm-devassist/src/snippets/model.rs`.
  - Modify `crates/bongterm-devassist/src/snippets/mod.rs`: `pub mod model; pub use model::*;`.

- [ ] **(1) Failing test** — create `crates/bongterm-devassist/src/snippets/model.rs` with:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    const LIB_JSON5: &str = r#"
    {
        // global snippets
        snippets: [
            {
                name: "gco",
                description: "git checkout a branch",
                scope: "global",
                command: "git checkout ${param:branch}",
            },
            {
                name: "deploy",
                description: "deploy to an env",
                scope: "workspace",
                command: "./deploy.sh ${param:env} ${param:tag}",
            },
        ],
    }
    "#;

    #[test]
    fn loads_json5_library_with_comments() {
        let lib = SnippetLibrary::from_json5(LIB_JSON5).expect("parse json5");
        assert_eq!(lib.snippets.len(), 2);
        assert_eq!(lib.snippets[0].name, "gco");
        assert_eq!(lib.snippets[0].scope, SnippetScope::Global);
        assert_eq!(lib.snippets[1].scope, SnippetScope::Workspace);
    }

    #[test]
    fn parses_params_in_order_without_duplicates() {
        let snip = Snippet {
            name: "deploy".to_string(),
            description: String::new(),
            scope: SnippetScope::Workspace,
            command: "./deploy.sh ${param:env} ${param:tag} ${param:env}".to_string(),
        };
        let params = snip.params();
        assert_eq!(params, vec!["env".to_string(), "tag".to_string()]);
    }

    #[test]
    fn malformed_json5_is_a_parse_error() {
        let err = SnippetLibrary::from_json5("{ snippets: [ { name: ").unwrap_err();
        assert!(matches!(err, crate::DevassistError::Parse(_)));
    }

    #[test]
    fn malformed_placeholder_is_rejected() {
        // Unterminated `${param:` must not panic and must yield no param.
        let snip = Snippet {
            name: "x".to_string(),
            description: String::new(),
            scope: SnippetScope::Global,
            command: "echo ${param:unterminated".to_string(),
        };
        assert!(snip.params().is_empty());
    }
}
```

- [ ] **(2) Run, expect FAIL**: `cargo test -p bongterm-devassist snippets::model`
  - Expected: `cannot find type 'SnippetLibrary'` / `Snippet` / `SnippetScope`.

- [ ] **(3) Minimal impl** — prepend to `crates/bongterm-devassist/src/snippets/model.rs`:

```rust
//! Snippet model: JSON5 library load + `${param:name}` placeholder parsing.
//!
//! Gate #12. Scope is workspace + global (closed enum). Placeholder parsing is
//! robust: malformed `${param:...}` yields no parameter rather than panicking.

use crate::DevassistError;

/// Where a snippet is visible.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SnippetScope {
    /// Available in every workspace.
    Global,
    /// Available only in the current workspace.
    Workspace,
}

/// A single snippet definition.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Snippet {
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub scope: SnippetScope,
    pub command: String,
}

impl Snippet {
    /// Distinct parameter names in first-appearance order, parsed from
    /// `${param:name}` placeholders. Malformed placeholders are ignored.
    #[must_use]
    pub fn params(&self) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        let bytes = self.command.as_bytes();
        let needle = b"${param:";
        let mut i = 0;
        while i + needle.len() <= bytes.len() {
            if &bytes[i..i + needle.len()] == needle {
                let start = i + needle.len();
                if let Some(rel_end) = self.command[start..].find('}') {
                    let name = &self.command[start..start + rel_end];
                    if !name.is_empty() && !out.iter().any(|n| n == name) {
                        out.push(name.to_string());
                    }
                    i = start + rel_end + 1;
                    continue;
                }
                // Unterminated placeholder: stop scanning (robust, no panic).
                break;
            }
            i += 1;
        }
        out
    }
}

/// A loaded library of snippets.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SnippetLibrary {
    #[serde(default)]
    pub snippets: Vec<Snippet>,
}

impl SnippetLibrary {
    /// Parse a JSON5 library document. Returns [`DevassistError::Parse`] on
    /// malformed input — a bad library never crashes snippet loading.
    pub fn from_json5(text: &str) -> Result<Self, DevassistError> {
        json5::from_str(text).map_err(|e| DevassistError::Parse(format!("snippet json5: {e}")))
    }

    /// Snippets visible in the given scope (global always; workspace when asked).
    #[must_use]
    pub fn visible_in(&self, scope: SnippetScope) -> Vec<&Snippet> {
        self.snippets
            .iter()
            .filter(|s| s.scope == SnippetScope::Global || s.scope == scope)
            .collect()
    }
}
```

  - `crates/bongterm-devassist/src/snippets/mod.rs`:

```rust
//! Snippets submodule (JSON5 library + parameter substitution).
pub(crate) const MODULE_NAME: &str = "snippets";

pub mod model;
pub use model::{Snippet, SnippetLibrary, SnippetScope};
```

- [ ] **(4) Run, expect PASS**: `cargo test -p bongterm-devassist snippets::model` → 4 passed.
- [ ] **(5) Commit**: `git add crates/bongterm-devassist && git commit -m "feat(devassist/3.C.1): Snippet JSON5 library + robust param parsing + scope"`

---

### 3.C.2 — `render_snippet`: parameter substitution before run (gate #12)

- [ ] **Files**
  - Create `crates/bongterm-devassist/src/snippets/render.rs`.
  - Modify `crates/bongterm-devassist/src/snippets/mod.rs`: `pub mod render; pub use render::*;`.

- [ ] **(1) Failing test** — create `crates/bongterm-devassist/src/snippets/render.rs` with:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::snippets::model::{Snippet, SnippetScope};
    use std::collections::HashMap;

    fn snip(cmd: &str) -> Snippet {
        Snippet {
            name: "s".to_string(),
            description: String::new(),
            scope: SnippetScope::Global,
            command: cmd.to_string(),
        }
    }

    #[test]
    fn substitutes_all_params() {
        let s = snip("git checkout ${param:branch}");
        let mut p = HashMap::new();
        p.insert("branch".to_string(), "main".to_string());
        let out = render_snippet(&s, &p).expect("render");
        assert_eq!(out, "git checkout main");
    }

    #[test]
    fn repeated_param_substituted_everywhere() {
        let s = snip("echo ${param:x} ${param:x}");
        let mut p = HashMap::new();
        p.insert("x".to_string(), "hi".to_string());
        assert_eq!(render_snippet(&s, &p).unwrap(), "echo hi hi");
    }

    #[test]
    fn missing_param_is_error_not_partial_run() {
        let s = snip("./deploy.sh ${param:env} ${param:tag}");
        let mut p = HashMap::new();
        p.insert("env".to_string(), "prod".to_string());
        let err = render_snippet(&s, &p).unwrap_err();
        match err {
            crate::DevassistError::MissingParam(name) => assert_eq!(name, "tag"),
            other => panic!("expected MissingParam, got {other:?}"),
        }
    }

    #[test]
    fn value_with_placeholder_syntax_is_not_re_expanded() {
        // Injection guard: a param value containing `${param:...}` is inserted
        // literally and not treated as a new placeholder.
        let s = snip("echo ${param:a}");
        let mut p = HashMap::new();
        p.insert("a".to_string(), "${param:b}".to_string());
        assert_eq!(render_snippet(&s, &p).unwrap(), "echo ${param:b}");
    }
}
```

- [ ] **(2) Run, expect FAIL**: `cargo test -p bongterm-devassist snippets::render`
  - Expected: `cannot find function 'render_snippet'`.

- [ ] **(3) Minimal impl** — prepend to `crates/bongterm-devassist/src/snippets/render.rs`:

```rust
//! Snippet parameter substitution (gate #12).
//!
//! Substitutes every `${param:name}` with the provided value. A missing
//! parameter is an error (never a partial command). Substitution is single-pass
//! so a value that itself contains `${param:...}` is inserted literally and not
//! re-expanded (injection guard).

use crate::snippets::model::Snippet;
use crate::DevassistError;
use std::collections::HashMap;

/// Render a snippet by substituting all `${param:name}` placeholders.
///
/// Returns [`DevassistError::MissingParam`] naming the first absent parameter.
pub fn render_snippet(
    snippet: &Snippet,
    params: &HashMap<String, String>,
) -> Result<String, DevassistError> {
    // Ensure every required param is present first (fail before producing text).
    for name in snippet.params() {
        if !params.contains_key(&name) {
            return Err(DevassistError::MissingParam(name));
        }
    }
    let mut out = String::with_capacity(snippet.command.len());
    let cmd = &snippet.command;
    let needle = "${param:";
    let mut rest = cmd.as_str();
    while let Some(pos) = rest.find(needle) {
        out.push_str(&rest[..pos]);
        let after = &rest[pos + needle.len()..];
        if let Some(end) = after.find('}') {
            let name = &after[..end];
            // Already validated present above; substitute literally.
            out.push_str(params.get(name).map_or("", String::as_str));
            rest = &after[end + 1..];
        } else {
            // Unterminated placeholder: emit literally and stop.
            out.push_str(&rest[pos..]);
            rest = "";
            break;
        }
    }
    out.push_str(rest);
    Ok(out)
}
```

  - `crates/bongterm-devassist/src/snippets/mod.rs` — add `pub mod render; pub use render::render_snippet;`.

- [ ] **(4) Run, expect PASS**: `cargo test -p bongterm-devassist snippets::render` → 4 passed.
- [ ] **(5) Commit**: `git add crates/bongterm-devassist && git commit -m "feat(devassist/3.C.2): render_snippet parameter substitution with missing-param + injection guard"`

---

### 3.C.3 — `SnippetStore`: workspace + global scope merge + parameter-prompt model (gate #12)

- [ ] **Files**
  - Modify `crates/bongterm-devassist/src/snippets/model.rs`: add `SnippetStore` + `ParamPrompt`.

- [ ] **(1) Failing test** — append to the `tests` module in `crates/bongterm-devassist/src/snippets/model.rs`:

```rust
    #[test]
    fn store_merges_global_and_workspace_with_workspace_priority() {
        let global = SnippetLibrary::from_json5(
            r#"{ snippets: [ { name: "ls", scope: "global", command: "ls -la" } ] }"#,
        )
        .unwrap();
        let workspace = SnippetLibrary::from_json5(
            r#"{ snippets: [ { name: "ls", scope: "workspace", command: "exa -la" },
                            { name: "t", scope: "workspace", command: "cargo test" } ] }"#,
        )
        .unwrap();
        let store = SnippetStore::new(global, workspace);
        // Resolve by name: workspace overrides global.
        assert_eq!(store.resolve("ls").unwrap().command, "exa -la");
        assert_eq!(store.resolve("t").unwrap().command, "cargo test");
        assert!(store.resolve("nope").is_none());
        // Listing shows both names, no duplicates.
        let mut names = store.names();
        names.sort();
        assert_eq!(names, vec!["ls".to_string(), "t".to_string()]);
    }

    #[test]
    fn prompt_lists_params_needing_input() {
        let store = SnippetStore::new(
            SnippetLibrary { snippets: vec![] },
            SnippetLibrary::from_json5(
                r#"{ snippets: [ { name: "d", scope: "workspace", command: "./d ${param:env} ${param:tag}" } ] }"#,
            )
            .unwrap(),
        );
        let prompt = store.prompt_for("d").expect("snippet exists");
        assert_eq!(prompt.snippet_name, "d");
        assert_eq!(prompt.params, vec!["env".to_string(), "tag".to_string()]);
    }
```

- [ ] **(2) Run, expect FAIL**: `cargo test -p bongterm-devassist snippets::model`
  - Expected: `cannot find type 'SnippetStore'` / `ParamPrompt`.

- [ ] **(3) Minimal impl** — append to `crates/bongterm-devassist/src/snippets/model.rs` (above the test module):

```rust
/// The parameter-prompt model the UI renders before running a snippet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParamPrompt {
    pub snippet_name: String,
    /// Parameter names to collect, in order.
    pub params: Vec<String>,
}

/// Merged snippet store: workspace snippets override global by name.
#[derive(Debug, Clone)]
pub struct SnippetStore {
    global: SnippetLibrary,
    workspace: SnippetLibrary,
}

impl SnippetStore {
    #[must_use]
    pub fn new(global: SnippetLibrary, workspace: SnippetLibrary) -> Self {
        Self { global, workspace }
    }

    /// Resolve a snippet by name. Workspace scope wins over global.
    #[must_use]
    pub fn resolve(&self, name: &str) -> Option<&Snippet> {
        self.workspace
            .snippets
            .iter()
            .find(|s| s.name == name)
            .or_else(|| self.global.snippets.iter().find(|s| s.name == name))
    }

    /// All distinct snippet names (workspace + global).
    #[must_use]
    pub fn names(&self) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        for s in self
            .workspace
            .snippets
            .iter()
            .chain(self.global.snippets.iter())
        {
            if !out.iter().any(|n| n == &s.name) {
                out.push(s.name.clone());
            }
        }
        out
    }

    /// Build the parameter prompt for a snippet, or `None` if it does not exist.
    #[must_use]
    pub fn prompt_for(&self, name: &str) -> Option<ParamPrompt> {
        self.resolve(name).map(|s| ParamPrompt {
            snippet_name: s.name.clone(),
            params: s.params(),
        })
    }
}
```

  - `crates/bongterm-devassist/src/snippets/mod.rs` — extend the re-export: `pub use model::{ParamPrompt, Snippet, SnippetLibrary, SnippetScope, SnippetStore};`.

- [ ] **(4) Run, expect PASS**: `cargo test -p bongterm-devassist snippets::model` → 6 passed.
- [ ] **(5) Commit**: `git add crates/bongterm-devassist && git commit -m "feat(devassist/3.C.3): SnippetStore workspace+global merge + ParamPrompt model"`

---

## Task Group 3.D — Background jobs (pane execution + toast + panel) → gate #13

### 3.D.1 — `Notifier` port + `MockNotifier` + `JobState` model

- [ ] **Files**
  - Create `crates/bongterm-devassist/src/jobs/runner.rs`.
  - Modify `crates/bongterm-devassist/src/jobs/mod.rs`: `pub mod runner; pub use runner::*;`.
  - Replace placeholder `crates/bongterm-test-kit/src/mocks/notifier.rs` with `MockNotifier`.

- [ ] **(1) Failing test** — create `crates/bongterm-devassist/src/jobs/runner.rs` with:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn job_state_terminal_classification() {
        assert!(!JobState::Running.is_terminal());
        assert!(JobState::Succeeded.is_terminal());
        assert!(JobState::Failed { exit_code: 1 }.is_terminal());
        assert!(JobState::Cancelled.is_terminal());
    }

    #[test]
    fn toast_for_state_distinguishes_success_and_failure() {
        let spec = JobSpec {
            id: JobId(uuid::Uuid::nil()),
            label: "npm install".to_string(),
            command: "npm".to_string(),
            args: vec!["install".to_string()],
            cwd: None,
        };
        let ok = Toast::for_completion(&spec, &JobState::Succeeded);
        assert_eq!(ok.kind, ToastKind::Success);
        assert!(ok.body.contains("npm install"));

        let bad = Toast::for_completion(&spec, &JobState::Failed { exit_code: 1 });
        assert_eq!(bad.kind, ToastKind::Failure);
        assert!(bad.body.contains("failed"));
    }

    #[test]
    fn mock_notifier_records_toasts() {
        use bongterm_test_kit::mocks::notifier::MockNotifier;
        let n = MockNotifier::new();
        n.notify(&Toast {
            kind: ToastKind::Success,
            title: "BongTerm".to_string(),
            body: "done".to_string(),
        });
        assert_eq!(n.toasts().len(), 1);
        assert_eq!(n.toasts()[0].kind, ToastKind::Success);
    }
}
```

- [ ] **(2) Run, expect FAIL**: `cargo test -p bongterm-devassist jobs::runner`
  - Expected: `cannot find type 'JobState'` / `JobSpec` / `Toast` / `JobId`.

- [ ] **(3) Minimal impl** — prepend to `crates/bongterm-devassist/src/jobs/runner.rs`:

```rust
//! Background-job model: spec, state, toast, and `Notifier` port (gate #13).
//!
//! Jobs run in a background pane OFF the hot path. On terminal state the runner
//! emits a desktop toast via the [`Notifier`] port (real impl uses WinRT
//! `UI_Notifications`; tests use `MockNotifier`). The closed `JobState` enum
//! makes the lifecycle exhaustive.

/// Unique identifier for a background job.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct JobId(pub uuid::Uuid);

/// What to run in the background.
#[derive(Debug, Clone)]
pub struct JobSpec {
    pub id: JobId,
    pub label: String,
    pub command: String,
    pub args: Vec<String>,
    pub cwd: Option<String>,
}

/// Lifecycle state of a background job. Closed set → exhaustive match.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JobState {
    /// Queued, not yet started.
    Pending,
    /// Currently executing.
    Running,
    /// Exited 0.
    Succeeded,
    /// Exited non-zero.
    Failed { exit_code: i64 },
    /// User-cancelled.
    Cancelled,
}

impl JobState {
    /// Whether this state is final (no further transitions).
    #[must_use]
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            JobState::Succeeded | JobState::Failed { .. } | JobState::Cancelled
        )
    }
}

/// Severity of a toast.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToastKind {
    Success,
    Failure,
    Info,
}

/// A desktop toast payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Toast {
    pub kind: ToastKind,
    pub title: String,
    pub body: String,
}

impl Toast {
    /// Build the completion toast for a job that reached a terminal state.
    #[must_use]
    pub fn for_completion(spec: &JobSpec, state: &JobState) -> Self {
        match state {
            JobState::Succeeded => Toast {
                kind: ToastKind::Success,
                title: "BongTerm".to_string(),
                body: format!("Background job \"{}\" completed.", spec.label),
            },
            JobState::Failed { exit_code } => Toast {
                kind: ToastKind::Failure,
                title: "BongTerm".to_string(),
                body: format!(
                    "Background job \"{}\" failed (exit {exit_code}).",
                    spec.label
                ),
            },
            JobState::Cancelled => Toast {
                kind: ToastKind::Info,
                title: "BongTerm".to_string(),
                body: format!("Background job \"{}\" was cancelled.", spec.label),
            },
            JobState::Pending | JobState::Running => Toast {
                kind: ToastKind::Info,
                title: "BongTerm".to_string(),
                body: format!("Background job \"{}\" is running.", spec.label),
            },
        }
    }
}

/// Port for emitting desktop notifications. Real impl wraps WinRT
/// `UI_Notifications`; tests inject `MockNotifier`. Keeping this a port means
/// the `windows` dependency never leaks into pure job logic.
pub trait Notifier: Send + Sync {
    fn notify(&self, toast: &Toast);
}
```

  - Replace `crates/bongterm-test-kit/src/mocks/notifier.rs`:

```rust
//! Recording mock for `bongterm_devassist::jobs::Notifier`.

use bongterm_devassist::jobs::{Notifier, Toast};
use std::sync::Mutex;

/// Records every toast for assertions.
pub struct MockNotifier {
    toasts: Mutex<Vec<Toast>>,
}

impl MockNotifier {
    #[must_use]
    pub fn new() -> Self {
        Self {
            toasts: Mutex::new(Vec::new()),
        }
    }

    /// Snapshot of recorded toasts.
    #[must_use]
    pub fn toasts(&self) -> Vec<Toast> {
        self.toasts.lock().unwrap().clone()
    }
}

impl Default for MockNotifier {
    fn default() -> Self {
        Self::new()
    }
}

impl Notifier for MockNotifier {
    fn notify(&self, toast: &Toast) {
        self.toasts.lock().unwrap().push(toast.clone());
    }
}
```

  - `crates/bongterm-devassist/src/jobs/mod.rs`:

```rust
//! Background-jobs submodule.
pub(crate) const MODULE_NAME: &str = "jobs";

pub mod runner;
pub use runner::{JobId, JobSpec, JobState, Notifier, Toast, ToastKind};
```

- [ ] **(4) Run, expect PASS**: `cargo test -p bongterm-devassist jobs::runner` → 3 passed.
- [ ] **(5) Commit**: `git add crates/bongterm-devassist crates/bongterm-test-kit && git commit -m "feat(devassist/3.D.1): JobState model + Toast + Notifier port + MockNotifier"`

---

### 3.D.2 — `JobRunner`: run to completion + emit toast on terminal state (gate #13)

- [ ] **Files**
  - Modify `crates/bongterm-devassist/src/jobs/runner.rs`: add `JobRunner` + `JobOutcome`.

- [ ] **(1) Failing test** — append to the `tests` module in `crates/bongterm-devassist/src/jobs/runner.rs`:

```rust
    #[test]
    fn runner_emits_success_toast_on_zero_exit() {
        use bongterm_test_kit::mocks::notifier::MockNotifier;
        let notifier = MockNotifier::new();
        let runner = JobRunner::new(&notifier);
        let spec = JobSpec {
            id: JobId(uuid::Uuid::nil()),
            label: "echo".to_string(),
            command: "echo".to_string(),
            args: vec![],
            cwd: None,
        };
        // Drive the runner with a pre-determined outcome (no real spawn in unit test).
        let final_state = runner.finish(&spec, JobOutcome::Exited { code: 0 });
        assert_eq!(final_state, JobState::Succeeded);
        assert_eq!(notifier.toasts().len(), 1);
        assert_eq!(notifier.toasts()[0].kind, ToastKind::Success);
    }

    #[test]
    fn runner_emits_failure_toast_on_nonzero_exit() {
        use bongterm_test_kit::mocks::notifier::MockNotifier;
        let notifier = MockNotifier::new();
        let runner = JobRunner::new(&notifier);
        let spec = JobSpec {
            id: JobId(uuid::Uuid::nil()),
            label: "sleep 3 && exit 1".to_string(),
            command: "sh".to_string(),
            args: vec!["-c".to_string(), "sleep 3 && exit 1".to_string()],
            cwd: None,
        };
        let final_state = runner.finish(&spec, JobOutcome::Exited { code: 1 });
        assert_eq!(final_state, JobState::Failed { exit_code: 1 });
        assert_eq!(notifier.toasts()[0].kind, ToastKind::Failure);
    }

    #[test]
    fn runner_spawn_failure_yields_failure_toast() {
        use bongterm_test_kit::mocks::notifier::MockNotifier;
        let notifier = MockNotifier::new();
        let runner = JobRunner::new(&notifier);
        let spec = JobSpec {
            id: JobId(uuid::Uuid::nil()),
            label: "broken".to_string(),
            command: "definitely-not-a-real-binary-xyz".to_string(),
            args: vec![],
            cwd: None,
        };
        let final_state = runner.finish(&spec, JobOutcome::SpawnError("not found".to_string()));
        assert!(matches!(final_state, JobState::Failed { .. }));
        assert_eq!(notifier.toasts()[0].kind, ToastKind::Failure);
    }
```

- [ ] **(2) Run, expect FAIL**: `cargo test -p bongterm-devassist jobs::runner`
  - Expected: `cannot find type 'JobRunner'` / `JobOutcome`.

- [ ] **(3) Minimal impl** — append to `crates/bongterm-devassist/src/jobs/runner.rs` (above the test module):

```rust
/// The raw outcome of running a job's process.
///
/// Produced by the spawn layer (`std::process`/`tokio`) off the hot path and
/// fed to `JobRunner::finish`. Separating outcome from state keeps the
/// state-transition + toast logic pure and unit-testable without spawning.
#[derive(Debug, Clone)]
pub enum JobOutcome {
    /// Process exited with this code.
    Exited { code: i64 },
    /// Process could not be spawned.
    SpawnError(String),
    /// User cancelled before completion.
    Cancelled,
}

/// Runs background jobs and emits a desktop toast on completion/failure.
pub struct JobRunner<'n> {
    notifier: &'n dyn Notifier,
}

impl<'n> JobRunner<'n> {
    #[must_use]
    pub fn new(notifier: &'n dyn Notifier) -> Self {
        Self { notifier }
    }

    /// Map an outcome to a terminal [`JobState`], emit the matching toast, and
    /// return the final state. Pure aside from the toast side-effect.
    pub fn finish(&self, spec: &JobSpec, outcome: JobOutcome) -> JobState {
        let state = match outcome {
            JobOutcome::Exited { code: 0 } => JobState::Succeeded,
            JobOutcome::Exited { code } => JobState::Failed { exit_code: code },
            JobOutcome::SpawnError(_) => JobState::Failed { exit_code: -1 },
            JobOutcome::Cancelled => JobState::Cancelled,
        };
        let toast = Toast::for_completion(spec, &state);
        self.notifier.notify(&toast);
        state
    }
}
```

> **Real spawn (non-test path)**: the composition layer in `bongterm-app` builds a `JobSpec`, spawns via `tokio::process::Command` on a background task (never the hot path), awaits the child, converts the result to `JobOutcome`, then calls `JobRunner::finish`. This plan unit-tests the pure transition; an `bongterm-app` integration test (task 3.exit) drives a real `sleep 3 && exit 1` end-to-end for gate #13.

- [ ] **(4) Run, expect PASS**: `cargo test -p bongterm-devassist jobs::runner` → 6 passed.
- [ ] **(5) Commit**: `git add crates/bongterm-devassist && git commit -m "feat(devassist/3.D.2): JobRunner outcome→state mapping + completion toast"`

---

### 3.D.3 — `JobList`: job-panel view-model (register/update/snapshot)

- [ ] **Files**
  - Create `crates/bongterm-devassist/src/jobs/list.rs`.
  - Modify `crates/bongterm-devassist/src/jobs/mod.rs`: `pub mod list; pub use list::*;`.

- [ ] **(1) Failing test** — create `crates/bongterm-devassist/src/jobs/list.rs` with:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::jobs::runner::{JobId, JobSpec, JobState};
    use uuid::Uuid;

    fn spec(label: &str) -> JobSpec {
        JobSpec {
            id: JobId(Uuid::new_v4()),
            label: label.to_string(),
            command: "x".to_string(),
            args: vec![],
            cwd: None,
        }
    }

    #[test]
    fn register_then_snapshot_shows_running_jobs() {
        let mut list = JobList::new();
        let s = spec("npm install");
        let id = s.id;
        list.register(s, JobState::Running);
        let snap = list.snapshot();
        assert_eq!(snap.len(), 1);
        assert_eq!(snap[0].label, "npm install");
        assert_eq!(snap[0].state, JobState::Running);
        assert_eq!(snap[0].id, id);
    }

    #[test]
    fn update_transitions_state_in_place() {
        let mut list = JobList::new();
        let s = spec("build");
        let id = s.id;
        list.register(s, JobState::Running);
        list.update(id, JobState::Succeeded);
        let snap = list.snapshot();
        assert_eq!(snap.len(), 1);
        assert_eq!(snap[0].state, JobState::Succeeded);
    }

    #[test]
    fn active_count_excludes_terminal_jobs() {
        let mut list = JobList::new();
        let a = spec("a");
        let b = spec("b");
        let (ida, idb) = (a.id, b.id);
        list.register(a, JobState::Running);
        list.register(b, JobState::Running);
        list.update(ida, JobState::Succeeded);
        assert_eq!(list.active_count(), 1);
        let _ = idb;
    }
}
```

- [ ] **(2) Run, expect FAIL**: `cargo test -p bongterm-devassist jobs::list`
  - Expected: `cannot find type 'JobList'`.

- [ ] **(3) Minimal impl** — prepend to `crates/bongterm-devassist/src/jobs/list.rs`:

```rust
//! Job-list view-model for the background-jobs panel (gate #13).
//!
//! Pure presentation state owned by devassist; `bongterm-ui` reads snapshots.
//! No process spawn here.

use crate::jobs::runner::{JobId, JobSpec, JobState};

/// One row in the job panel.
#[derive(Debug, Clone)]
pub struct JobRow {
    pub id: JobId,
    pub label: String,
    pub state: JobState,
}

/// Ordered list of background jobs.
#[derive(Debug, Clone, Default)]
pub struct JobList {
    rows: Vec<JobRow>,
}

impl JobList {
    #[must_use]
    pub fn new() -> Self {
        Self { rows: Vec::new() }
    }

    /// Register a new job with an initial state.
    pub fn register(&mut self, spec: JobSpec, state: JobState) {
        self.rows.push(JobRow {
            id: spec.id,
            label: spec.label,
            state,
        });
    }

    /// Update the state of an existing job. No-op if the id is unknown.
    pub fn update(&mut self, id: JobId, state: JobState) {
        if let Some(row) = self.rows.iter_mut().find(|r| r.id == id) {
            row.state = state;
        }
    }

    /// Snapshot of all rows in registration order.
    #[must_use]
    pub fn snapshot(&self) -> Vec<JobRow> {
        self.rows.clone()
    }

    /// Count of non-terminal (still running/pending) jobs.
    #[must_use]
    pub fn active_count(&self) -> usize {
        self.rows.iter().filter(|r| !r.state.is_terminal()).count()
    }
}
```

  - `crates/bongterm-devassist/src/jobs/mod.rs` — add `pub mod list; pub use list::{JobList, JobRow};`.

- [ ] **(4) Run, expect PASS**: `cargo test -p bongterm-devassist jobs::list` → 3 passed.
- [ ] **(5) Commit**: `git add crates/bongterm-devassist && git commit -m "feat(devassist/3.D.3): JobList panel view-model"`

---

## Task Group 3.E — Clickable patterns (file:line + URL + OSC 8) → gate #14

> **Hot-path rule**: all matching here operates on an already-materialized line/`SurfaceSnapshot` slice, produces overlay `Span`s, and NEVER mutates scrollback. Matching runs off the hot path (e.g. when a block completes or on demand for the visible viewport), not inline in the parser.

### 3.E.1 — `PatternMatcher` set for Node/Python/Rust/.NET/TS file:line (gate #14)

- [ ] **Files**
  - Create `crates/bongterm-devassist/src/patterns/matchers.rs`.
  - Modify `crates/bongterm-devassist/src/patterns/mod.rs`: `pub mod matchers; pub use matchers::*;`.

- [ ] **(1) Failing test** — create `crates/bongterm-devassist/src/patterns/matchers.rs` with:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_typescript_file_line_col() {
        // tsc / node style: src/index.ts:42:7
        let spans = scan_file_locations("error at src/index.ts:42:7 unexpected token");
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].path, "src/index.ts");
        assert_eq!(spans[0].line, Some(42));
        assert_eq!(spans[0].column, Some(7));
        assert_eq!(spans[0].kind, PatternKind::FileLine);
        // Byte range points at the matched substring.
        assert_eq!(&"error at src/index.ts:42:7 unexpected token"[spans[0].start..spans[0].end], "src/index.ts:42:7");
    }

    #[test]
    fn matches_rust_file_line() {
        let spans = scan_file_locations("  --> crates/foo/src/lib.rs:128:13");
        assert_eq!(spans[0].path, "crates/foo/src/lib.rs");
        assert_eq!(spans[0].line, Some(128));
        assert_eq!(spans[0].column, Some(13));
    }

    #[test]
    fn matches_python_traceback() {
        // File "app/main.py", line 10
        let spans = scan_file_locations(r#"  File "app/main.py", line 10, in <module>"#);
        assert_eq!(spans[0].path, "app/main.py");
        assert_eq!(spans[0].line, Some(10));
        assert_eq!(spans[0].kind, PatternKind::PythonTraceback);
    }

    #[test]
    fn matches_node_stack_frame() {
        // at Object.<anonymous> (/srv/app/server.js:23:9)
        let spans = scan_file_locations("    at Object.<anonymous> (/srv/app/server.js:23:9)");
        assert_eq!(spans[0].path, "/srv/app/server.js");
        assert_eq!(spans[0].line, Some(23));
        assert_eq!(spans[0].column, Some(9));
    }

    #[test]
    fn matches_dotnet_stack_frame() {
        // in C:\proj\Program.cs:line 55
        let spans = scan_file_locations(r"   at App.Main() in C:\proj\Program.cs:line 55");
        assert_eq!(spans[0].path, r"C:\proj\Program.cs");
        assert_eq!(spans[0].line, Some(55));
        assert_eq!(spans[0].kind, PatternKind::DotNetStack);
    }

    #[test]
    fn no_false_positive_on_plain_text() {
        let spans = scan_file_locations("the time is 12:30 and all is well");
        assert!(spans.is_empty(), "time-of-day must not match as file:line");
    }
}
```

- [ ] **(2) Run, expect FAIL**: `cargo test -p bongterm-devassist patterns::matchers`
  - Expected: `cannot find function 'scan_file_locations'` / `PatternKind` / `FileSpan`.

- [ ] **(3) Minimal impl** — prepend to `crates/bongterm-devassist/src/patterns/matchers.rs`:

```rust
//! File:line clickable-pattern matchers for Node/Python/Rust/.NET/TS (gate #14).
//!
//! Produces overlay spans only — never mutates scrollback. The closed
//! `PatternKind` enum bounds the recognized formats (SOLID). Regexes are
//! compiled once via `LazyLock`.

use regex::Regex;
use std::sync::LazyLock;

/// The closed set of recognized location-pattern kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PatternKind {
    /// `path:line[:col]` (Rust/TS/tsc/generic compilers, Node `(path:line:col)`).
    FileLine,
    /// Python traceback: `File "path", line N`.
    PythonTraceback,
    /// .NET stack: `in path:line N`.
    DotNetStack,
}

/// A matched file location as an overlay span.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileSpan {
    pub path: String,
    pub line: Option<u32>,
    pub column: Option<u32>,
    pub kind: PatternKind,
    /// Byte offsets into the scanned line (overlay range; scrollback untouched).
    pub start: usize,
    pub end: usize,
}

// `path:line[:col]` — path must contain a `/`, `\`, or a known source extension
// to avoid matching bare `12:30` time-of-day. Anchored on a path-like token.
static RE_FILE_LINE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?P<path>(?:[A-Za-z]:)?[\w./\\-]*[\w-]+\.(?:rs|ts|tsx|js|jsx|mjs|cjs|cs|go|py|java|kt|rb|c|h|cpp|hpp))(?::(?P<line>\d+))(?::(?P<col>\d+))?",
    )
    .expect("valid file:line regex")
});

static RE_PY_TRACEBACK: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"File "(?P<path>[^"]+)", line (?P<line>\d+)"#).expect("valid py traceback regex")
});

static RE_DOTNET: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"in (?P<path>(?:[A-Za-z]:)?[\w./\\-]+\.\w+):line (?P<line>\d+)")
        .expect("valid dotnet regex")
});

/// Scan a single line of output for clickable file locations.
/// Returns overlay spans; the input is never mutated.
#[must_use]
pub fn scan_file_locations(line: &str) -> Vec<FileSpan> {
    let mut spans: Vec<FileSpan> = Vec::new();

    // .NET first (its `path:line N` would partially overlap file:line otherwise).
    for c in RE_DOTNET.captures_iter(line) {
        let m = c.get(0).unwrap();
        spans.push(FileSpan {
            path: c["path"].to_string(),
            line: c.name("line").and_then(|x| x.as_str().parse().ok()),
            column: None,
            kind: PatternKind::DotNetStack,
            start: m.start(),
            end: m.end(),
        });
    }
    // Python traceback.
    for c in RE_PY_TRACEBACK.captures_iter(line) {
        let m = c.get(0).unwrap();
        spans.push(FileSpan {
            path: c["path"].to_string(),
            line: c.name("line").and_then(|x| x.as_str().parse().ok()),
            column: None,
            kind: PatternKind::PythonTraceback,
            start: m.start(),
            end: m.end(),
        });
    }
    // Generic file:line[:col] — skip ranges already covered by .NET matches.
    for c in RE_FILE_LINE.captures_iter(line) {
        let m = c.get(0).unwrap();
        let overlaps = spans.iter().any(|s| m.start() < s.end && s.start < m.end());
        if overlaps {
            continue;
        }
        spans.push(FileSpan {
            path: c["path"].to_string(),
            line: c.name("line").and_then(|x| x.as_str().parse().ok()),
            column: c.name("col").and_then(|x| x.as_str().parse().ok()),
            kind: PatternKind::FileLine,
            start: m.start(),
            end: m.end(),
        });
    }
    spans.sort_by_key(|s| s.start);
    spans
}
```

  - `crates/bongterm-devassist/src/patterns/mod.rs`:

```rust
//! Clickable-patterns submodule (file:line + URL + OSC 8).
pub(crate) const MODULE_NAME: &str = "patterns";

pub mod matchers;
pub use matchers::{scan_file_locations, FileSpan, PatternKind};
```

- [ ] **(4) Run, expect PASS**: `cargo test -p bongterm-devassist patterns::matchers` → 6 passed.
- [ ] **(5) Commit**: `git add crates/bongterm-devassist && git commit -m "feat(devassist/3.E.1): file:line matchers for Rust/TS/Node/Python/.NET"`

---

### 3.E.2 — `ClickableOverlay`: assemble spans for a viewport (overlay only, no scrollback mutation) (gate #14)

- [ ] **Files**
  - Modify `crates/bongterm-devassist/src/patterns/matchers.rs`: add `ClickableOverlay` + `OverlaySpan` + `LineRef`.

- [ ] **(1) Failing test** — append to the `tests` module in `crates/bongterm-devassist/src/patterns/matchers.rs`:

```rust
    #[test]
    fn overlay_collects_spans_per_line_without_mutating_text() {
        let lines = vec![
            LineRef { row: 0, text: "ok no match here".to_string() },
            LineRef { row: 1, text: "error src/main.rs:10:4".to_string() },
            LineRef { row: 2, text: r#"File "x.py", line 3"#.to_string() },
        ];
        let overlay = ClickableOverlay::build(&lines);
        // Row 0 has no spans, rows 1 and 2 each have one.
        assert_eq!(overlay.spans_for_row(0).len(), 0);
        assert_eq!(overlay.spans_for_row(1).len(), 1);
        assert_eq!(overlay.spans_for_row(2).len(), 1);
        // The original text is unchanged (overlay holds copies/offsets only).
        assert_eq!(lines[1].text, "error src/main.rs:10:4");
        // Span carries the row for click routing.
        let s = &overlay.spans_for_row(1)[0];
        assert_eq!(s.row, 1);
        assert_eq!(s.file.path, "src/main.rs");
    }
```

- [ ] **(2) Run, expect FAIL**: `cargo test -p bongterm-devassist patterns::matchers overlay`
  - Expected: `cannot find type 'ClickableOverlay'` / `LineRef` / `OverlaySpan`.

- [ ] **(3) Minimal impl** — append to `crates/bongterm-devassist/src/patterns/matchers.rs` (above the test module):

```rust
/// A read-only reference to one rendered line (row index + its text).
#[derive(Debug, Clone)]
pub struct LineRef {
    pub row: usize,
    pub text: String,
}

/// A clickable span anchored to a specific row. Overlay only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OverlaySpan {
    pub row: usize,
    pub file: FileSpan,
}

/// All clickable spans for a viewport. Built from line refs; the underlying
/// scrollback/text is never modified (gate #14 / hot-path rule).
#[derive(Debug, Clone, Default)]
pub struct ClickableOverlay {
    spans: Vec<OverlaySpan>,
}

impl ClickableOverlay {
    /// Scan each line and collect clickable file-location spans.
    #[must_use]
    pub fn build(lines: &[LineRef]) -> Self {
        let mut spans = Vec::new();
        for line in lines {
            for file in scan_file_locations(&line.text) {
                spans.push(OverlaySpan {
                    row: line.row,
                    file,
                });
            }
        }
        Self { spans }
    }

    /// Spans on a given row.
    #[must_use]
    pub fn spans_for_row(&self, row: usize) -> Vec<&OverlaySpan> {
        self.spans.iter().filter(|s| s.row == row).collect()
    }

    /// All spans.
    #[must_use]
    pub fn all(&self) -> &[OverlaySpan] {
        &self.spans
    }
}
```

  - `crates/bongterm-devassist/src/patterns/mod.rs` — extend re-export: `pub use matchers::{scan_file_locations, ClickableOverlay, FileSpan, LineRef, OverlaySpan, PatternKind};`.

- [ ] **(4) Run, expect PASS**: `cargo test -p bongterm-devassist patterns::matchers overlay` → passed; full `cargo test -p bongterm-devassist patterns::matchers` → 7 passed.
- [ ] **(5) Commit**: `git add crates/bongterm-devassist && git commit -m "feat(devassist/3.E.2): ClickableOverlay viewport spans — overlay only, no scrollback mutation"`

---

### 3.E.3 — URL detection + OSC 8 hyperlink rendering with spoof guard (gate #14, security)

- [ ] **Files**
  - Create `crates/bongterm-devassist/src/patterns/url.rs`.
  - Modify `crates/bongterm-devassist/src/patterns/mod.rs`: `pub mod url; pub use url::*;`.

- [ ] **(1) Failing test** — create `crates/bongterm-devassist/src/patterns/url.rs` with:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_bare_urls() {
        let spans = scan_urls("see https://example.com/docs and http://localhost:3000 now");
        assert_eq!(spans.len(), 2);
        assert_eq!(spans[0].url, "https://example.com/docs");
        assert_eq!(spans[0].kind, LinkKind::Bare);
        assert_eq!(spans[1].url, "http://localhost:3000");
    }

    #[test]
    fn parses_osc8_hyperlink() {
        // OSC 8 ; params ; URI ST  text  OSC 8 ; ; ST
        // ESC ] 8 ; ; https://example.com ESC \  Example  ESC ] 8 ; ; ESC \
        let raw = "\x1b]8;;https://example.com\x1b\\Example\x1b]8;;\x1b\\";
        let links = parse_osc8(raw).expect("parse osc8");
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].url, "https://example.com");
        assert_eq!(links[0].text, "Example");
        assert_eq!(links[0].kind, LinkKind::Osc8);
    }

    #[test]
    fn osc8_spoof_text_url_mismatch_is_flagged() {
        // Display text claims one host, URI points at another → must be flagged.
        let link = Osc8Link {
            url: "https://evil.test/login".to_string(),
            text: "https://bank.example.com".to_string(),
            kind: LinkKind::Osc8,
        };
        assert!(link.is_spoof_suspect());
    }

    #[test]
    fn osc8_matching_text_is_not_flagged() {
        let link = Osc8Link {
            url: "https://example.com/x".to_string(),
            text: "Example docs".to_string(),
            kind: LinkKind::Osc8,
        };
        assert!(!link.is_spoof_suspect());
    }

    #[test]
    fn verify_destination_rejects_non_http_schemes() {
        assert!(verify_destination("https://example.com").is_ok());
        assert!(verify_destination("http://example.com").is_ok());
        assert!(verify_destination("file:///etc/passwd").is_err());
        assert!(verify_destination("javascript:alert(1)").is_err());
    }
}
```

- [ ] **(2) Run, expect FAIL**: `cargo test -p bongterm-devassist patterns::url`
  - Expected: `cannot find function 'scan_urls'` / `parse_osc8` / `Osc8Link` / `LinkKind` / `verify_destination`.

- [ ] **(3) Minimal impl** — prepend to `crates/bongterm-devassist/src/patterns/url.rs`:

```rust
//! URL detection + OSC 8 hyperlink rendering with spoof guard (gate #14).
//!
//! Security surface (threat model: OSC 8 hyperlink spoofing). Links are overlay
//! only and never auto-opened: `verify_destination` rejects non-http(s)
//! schemes, and `Osc8Link::is_spoof_suspect` flags display-text/URI mismatch so
//! the UI can warn before navigation.

use crate::DevassistError;
use regex::Regex;
use std::sync::LazyLock;

/// Kind of detected link.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LinkKind {
    /// A bare URL found in plain text.
    Bare,
    /// An OSC 8 hyperlink (URI + display text).
    Osc8,
}

/// A bare URL overlay span.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UrlSpan {
    pub url: String,
    pub kind: LinkKind,
    pub start: usize,
    pub end: usize,
}

/// A parsed OSC 8 hyperlink.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Osc8Link {
    pub url: String,
    pub text: String,
    pub kind: LinkKind,
}

impl Osc8Link {
    /// True when the display text looks like a URL pointing at a different host
    /// than the actual URI (classic OSC 8 spoof). Conservative: only flags when
    /// the text itself is URL-shaped.
    #[must_use]
    pub fn is_spoof_suspect(&self) -> bool {
        let text_trim = self.text.trim();
        if text_trim.starts_with("http://") || text_trim.starts_with("https://") {
            host_of(text_trim) != host_of(&self.url)
        } else {
            false
        }
    }
}

fn host_of(url: &str) -> Option<String> {
    let after_scheme = url.split("://").nth(1)?;
    let host = after_scheme
        .split(['/', '?', '#'])
        .next()?
        .split('@')
        .last()?;
    Some(host.to_ascii_lowercase())
}

static RE_URL: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"https?://[^\s\x1b]+").expect("valid url regex"));

/// Scan a line for bare http(s) URLs. Overlay only.
#[must_use]
pub fn scan_urls(line: &str) -> Vec<UrlSpan> {
    RE_URL
        .find_iter(line)
        .map(|m| UrlSpan {
            url: m.as_str().to_string(),
            kind: LinkKind::Bare,
            start: m.start(),
            end: m.end(),
        })
        .collect()
}

/// Parse OSC 8 hyperlinks from a raw byte/escape string.
///
/// Format: `ESC ] 8 ; params ; URI ST text ESC ] 8 ; ; ST` where `ST` is either
/// `ESC \` (ST) or BEL. Returns the link list; malformed sequences are skipped.
pub fn parse_osc8(raw: &str) -> Result<Vec<Osc8Link>, DevassistError> {
    let mut links = Vec::new();
    let open = "\x1b]8;";
    let mut search_from = 0;
    while let Some(rel) = raw[search_from..].find(open) {
        let abs = search_from + rel;
        let after = &raw[abs + open.len()..];
        // params ; URI <ST>
        let Some(semi) = after.find(';') else {
            break;
        };
        let after_uri = &after[semi + 1..];
        let Some(st_pos) = find_st(after_uri) else {
            break;
        };
        let uri = after_uri[..st_pos.0].to_string();
        let after_st = &after_uri[st_pos.1..];
        // text ends at the next OSC 8 open.
        let text_end = after_st.find(open).unwrap_or(after_st.len());
        let text = after_st[..text_end].to_string();
        if !uri.is_empty() {
            links.push(Osc8Link {
                url: uri,
                text,
                kind: LinkKind::Osc8,
            });
        }
        search_from = abs + open.len() + semi + 1 + st_pos.1 + text_end;
    }
    Ok(links)
}

/// Returns (offset_of_ST_start, offset_after_ST) for the first ST/BEL terminator.
fn find_st(s: &str) -> Option<(usize, usize)> {
    if let Some(p) = s.find("\x1b\\") {
        return Some((p, p + 2));
    }
    if let Some(p) = s.find('\x07') {
        return Some((p, p + 1));
    }
    None
}

/// Verify a link destination is safe to offer for navigation.
/// Only `http`/`https` are permitted; everything else is rejected (OSC 8
/// security: no `file:`, `javascript:`, etc.).
pub fn verify_destination(url: &str) -> Result<(), DevassistError> {
    let lower = url.trim().to_ascii_lowercase();
    if lower.starts_with("https://") || lower.starts_with("http://") {
        Ok(())
    } else {
        Err(DevassistError::Parse(format!(
            "refusing non-http(s) link destination: {url}"
        )))
    }
}
```

  - `crates/bongterm-devassist/src/patterns/mod.rs` — add `pub mod url; pub use url::{parse_osc8, scan_urls, verify_destination, LinkKind, Osc8Link, UrlSpan};`.

- [ ] **(4) Run, expect PASS**: `cargo test -p bongterm-devassist patterns::url` → 5 passed.
- [ ] **(5) Commit**: `git add crates/bongterm-devassist && git commit -m "feat(devassist/3.E.3): URL detection + OSC 8 parse + spoof guard + scheme allowlist"`

---

## Task Group 3.F — UI view-model wiring (thin, no process spawn) → supports gates #9, #13, #14

> `bongterm-ui` owns presentation only. It consumes `bongterm-devassist` *snapshots* and never spawns processes, reads secrets, or mutates Git/scrollback. This group adds one `devux` module that maps devassist data into render-ready view-models. The actual subprocess/job execution happens in `bongterm-app` (3.exit), which holds the `bongterm-devassist` runners and feeds snapshots into the UI.

### 3.F.1 — `devux` view-models: Cmd-K preview banner + job panel + clickable overlay rows

The map functions are pure: devassist snapshot in, UI view-model out. No `Task`, no IO. This keeps the dependency direction legal (`ui → devassist` is a read-only data edge) and makes the mapping unit-testable without a render surface.

- **Files**:
  - Modify `crates/bongterm-ui/Cargo.toml` — add dep.
  - Modify `tools/xtask/allowed-deps.toml` — add `bongterm-devassist` to `[bongterm-ui]`.
  - Create `crates/bongterm-ui/src/devux/mod.rs`.
  - Modify `crates/bongterm-ui/src/lib.rs` — add `pub mod devux;`.
  - Test: inline `#[cfg(test)]` in `crates/bongterm-ui/src/devux/mod.rs`.

- [ ] **(0) Dependency wiring**:
  - In `crates/bongterm-ui/Cargo.toml`, under `[dependencies]` after the `bongterm-settings` line add:

```toml
bongterm-devassist = { path = "../bongterm-devassist" }
```

  - In `tools/xtask/allowed-deps.toml`, change the `[bongterm-ui]` entry to include the new edge:

```toml
[bongterm-ui]
allowed = ["bongterm-settings", "bongterm-secrets-api", "bongterm-storage-api", "bongterm-render", "bongterm-devassist"]
```

- [ ] **(1) Failing test** — append to a new file `crates/bongterm-ui/src/devux/mod.rs`:

```rust
//! View-model adapters: map `bongterm-devassist` snapshots into UI-render state.
//!
//! Ownership contract (spec §1.2): `bongterm-ui` is presentation only. Every
//! function in this module is a pure transform — it never spawns a process,
//! reads a secret, mutates Git, or mutates scrollback. Process and job
//! execution live in `bongterm-app`, which passes snapshots down to these maps.

use bongterm_devassist::ai::cmdk::CmdKView;
use bongterm_devassist::jobs::list::{JobListSnapshot, JobRowView};
use bongterm_devassist::patterns::matchers::Span;

/// What the Cmd-K banner should show. Preview is never auto-runnable: the
/// `run_enabled` flag is only `true` once the session reports a confirmed
/// preview, mirroring the `CmdKSession` state machine (task 3.A.2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CmdKBanner {
    pub headline: String,
    /// The previewed command text, shown verbatim and read-only.
    pub preview: Option<String>,
    /// `true` only when a preview exists AND the user may press Run.
    pub run_enabled: bool,
    /// `true` when the backend is unavailable (Claude Code not installed).
    pub unavailable: bool,
}

/// Map a Cmd-K view snapshot into a banner. Pure.
#[must_use]
pub fn cmdk_banner(view: &CmdKView) -> CmdKBanner {
    match view {
        CmdKView::Idle => CmdKBanner {
            headline: "Cmd-K: describe a command".to_string(),
            preview: None,
            run_enabled: false,
            unavailable: false,
        },
        CmdKView::Previewed { command } => CmdKBanner {
            headline: "Preview — press Run to execute".to_string(),
            preview: Some(command.clone()),
            run_enabled: true,
            unavailable: false,
        },
        CmdKView::Unavailable { reason } => CmdKBanner {
            headline: format!("AI assist unavailable: {reason}"),
            preview: None,
            run_enabled: false,
            unavailable: true,
        },
    }
}

/// One row in the job panel, as the UI consumes it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JobPanelRow {
    pub label: String,
    pub status: String,
    pub is_terminal: bool,
}

/// Map a job-list snapshot into panel rows. Pure.
#[must_use]
pub fn job_panel_rows(snapshot: &JobListSnapshot) -> Vec<JobPanelRow> {
    snapshot
        .rows
        .iter()
        .map(|r: &JobRowView| JobPanelRow {
            label: r.label.clone(),
            status: r.status_label.clone(),
            is_terminal: r.is_terminal,
        })
        .collect()
}

/// A clickable region the UI may underline/hover. Derived from devassist spans;
/// carries only byte offsets into the *already-rendered* line — it never
/// rewrites the line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClickableRegion {
    pub start: usize,
    pub end: usize,
}

/// Map devassist spans into clickable regions. Pure; preserves order; drops
/// nothing. Overlay-only by construction (offsets, not replacement text).
#[must_use]
pub fn clickable_regions(spans: &[Span]) -> Vec<ClickableRegion> {
    spans
        .iter()
        .map(|s| ClickableRegion {
            start: s.start,
            end: s.end,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use bongterm_devassist::ai::cmdk::CmdKView;
    use bongterm_devassist::jobs::list::{JobListSnapshot, JobRowView};
    use bongterm_devassist::patterns::matchers::Span;

    #[test]
    fn idle_banner_has_no_preview_and_run_disabled() {
        let b = cmdk_banner(&CmdKView::Idle);
        assert_eq!(b.preview, None);
        assert!(!b.run_enabled, "idle must never be runnable");
        assert!(!b.unavailable);
    }

    #[test]
    fn previewed_banner_carries_command_and_enables_run() {
        let b = cmdk_banner(&CmdKView::Previewed {
            command: "git status".to_string(),
        });
        assert_eq!(b.preview.as_deref(), Some("git status"));
        assert!(b.run_enabled);
    }

    #[test]
    fn unavailable_banner_disables_run_and_flags_unavailable() {
        let b = cmdk_banner(&CmdKView::Unavailable {
            reason: "claude not found".to_string(),
        });
        assert!(!b.run_enabled, "unavailable must never be runnable");
        assert!(b.unavailable);
        assert!(b.headline.contains("claude not found"));
    }

    #[test]
    fn job_rows_map_one_to_one_in_order() {
        let snap = JobListSnapshot {
            rows: vec![
                JobRowView {
                    label: "build".to_string(),
                    status_label: "running".to_string(),
                    is_terminal: false,
                },
                JobRowView {
                    label: "test".to_string(),
                    status_label: "succeeded".to_string(),
                    is_terminal: true,
                },
            ],
        };
        let rows = job_panel_rows(&snap);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].label, "build");
        assert!(!rows[0].is_terminal);
        assert_eq!(rows[1].status, "succeeded");
        assert!(rows[1].is_terminal);
    }

    #[test]
    fn clickable_regions_preserve_offsets_and_order() {
        let spans = vec![
            Span { start: 0, end: 4 },
            Span { start: 10, end: 22 },
        ];
        let regions = clickable_regions(&spans);
        assert_eq!(regions.len(), 2);
        assert_eq!((regions[0].start, regions[0].end), (0, 4));
        assert_eq!((regions[1].start, regions[1].end), (10, 22));
    }
}
```

  This test references three devassist types that the UI consumes as snapshots: `CmdKView` (a serializable projection of `CmdKSession` state — add it in 3.A.2's module if not already present), `JobListSnapshot`/`JobRowView` (from `jobs::list`, task 3.D.3), and `Span` (from `patterns::matchers`, task 3.E.1). If `CmdKView` does not yet exist as a public snapshot type, add the following to `crates/bongterm-devassist/src/ai/cmdk.rs` and re-export it from `ai/mod.rs` (it is a read-only projection — it carries no `AiBackend` handle, so the UI cannot trigger execution):

```rust
/// Read-only projection of `CmdKSession` for the UI. Carries no backend handle,
/// so the UI cannot execute anything from it — execution stays in the app layer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CmdKView {
    Idle,
    Previewed { command: String },
    Unavailable { reason: String },
}

impl CmdKSession {
    /// Project current state into a UI snapshot.
    #[must_use]
    pub fn view(&self) -> CmdKView {
        match &self.state {
            CmdKState::Idle => CmdKView::Idle,
            CmdKState::Previewed { command } | CmdKState::Confirmed { command } => {
                CmdKView::Previewed {
                    command: command.clone(),
                }
            }
            CmdKState::Unavailable { reason } => CmdKView::Unavailable {
                reason: reason.clone(),
            },
        }
    }
}
```

  > Note: `Confirmed` projects to `Previewed` for display because the banner shows the same command text; the `run_enabled` flag is what gates execution, and execution is performed by the app layer holding the real `CmdKSession`, not by the view. If 3.A.2 named the state fields differently, adapt the `match` arms to the actual `CmdKState` variants — the projection contract (three view variants) is fixed.

- [ ] **(2) Run, expect FAIL**: `cargo test -p bongterm-ui devux` →
  - If `CmdKView`/`JobRowView`/`Span` projections are missing: `error[E0432]: unresolved import`.
  - Once imports resolve but `devux` is not declared: `error[E0583]: file not found for module \`devux\``.

- [ ] **(3) Minimal impl**:
  - Ensure `JobListSnapshot { rows: Vec<JobRowView> }` and `JobRowView { label: String, status_label: String, is_terminal: bool }` are the public snapshot types produced by `JobList::snapshot()` in task 3.D.3. If 3.D.3 named them differently, add a thin `snapshot()` returning these exact shapes (the panel contract is fixed here).
  - Add the `CmdKView` projection above to `bongterm-devassist`.
  - In `crates/bongterm-ui/src/lib.rs`, add after the existing `pub` items near the top of the module body:

```rust
pub mod devux;
```

- [ ] **(4) Run, expect PASS**: `cargo test -p bongterm-ui devux` → 5 passed. Then `cargo xtask check-deps` → passes (the `bongterm-ui → bongterm-devassist` edge is now in `allowed-deps.toml`).
- [ ] **(5) Commit**: `git add crates/bongterm-ui crates/bongterm-devassist tools/xtask/allowed-deps.toml && git commit -m "feat(ui/3.F.1): devux view-models — Cmd-K banner, job panel rows, clickable regions"`

---

## Task Group 3.exit — Phase 3 exit gate: §6.1 #9–#14 green (end-to-end integration)

> These integration tests live in `bongterm-app` (the only crate allowed to depend on every feature crate) and prove each acceptance gate against real subprocesses where the gate demands it. They are the nightly gate checks: Phase 3 exits only when all six pass for 7 consecutive nightly runs. Each test is `#[cfg_attr(not(windows), ignore)]` where it relies on a Windows shell, and `#[ignore]`-gated behind a `claude`-on-PATH probe where it needs the real CLI, so CI stays green on machines without Claude Code while nightly (which provisions it) runs the full set.

### 3.exit.1 — Gate #9 + #10 + #11: AI preview-no-spawn, explainer, smart-history E2E

- **Files**:
  - Create `crates/bongterm-app/tests/phase3_gates.rs`.
  - Modify `crates/bongterm-app/Cargo.toml` — add `[dev-dependencies]` on `bongterm-devassist`, `bongterm-test-kit`, `tokio`, `tempfile`.

- [ ] **(0) Dependency wiring** — in `crates/bongterm-app/Cargo.toml` add:

```toml
[dev-dependencies]
bongterm-devassist = { path = "../bongterm-devassist" }
bongterm-test-kit = { path = "../bongterm-test-kit" }
tokio = { workspace = true }
tempfile = "3"
```

  (`bongterm-devassist` is already in the `[bongterm-app]` allowed-deps list, so `cargo xtask check-deps` needs no change. `tempfile` is an external dev-dep — add `tempfile = "3"` to `[workspace.dependencies]` if the workspace prefers pinned dev-deps; otherwise the inline pin is acceptable for a test-only dep.)

- [ ] **(1) Failing test** — `crates/bongterm-app/tests/phase3_gates.rs`:

```rust
//! Phase 3 exit-gate integration tests (spec §6.1 #9–#14).
//!
//! Each test maps to one acceptance gate. Tests that need the real `claude`
//! binary are gated behind `claude_on_path()`; tests that need a Windows shell
//! are gated behind `cfg(windows)`. Nightly provisions both and runs the full
//! set; developer machines skip what they lack.

use bongterm_devassist::ai::cmdk::{CmdKSession, CmdKView};
use bongterm_test_kit::mocks::ai_backend::MockAiBackend;

/// Gate #9: Cmd-K produces a preview and NEVER executes until explicit confirm.
/// Proven with a mock backend that records every call; the assertion is that
/// no "run" side effect occurs across `request_preview`, and that `view()`
/// reports a non-runnable preview until `confirm_run` is invoked.
#[tokio::test]
async fn gate_9_cmdk_preview_does_not_run_until_confirm() {
    let backend = MockAiBackend::with_suggestion("git status");
    let mut session = CmdKSession::new(Box::new(backend.clone()));

    session
        .request_preview("show me repo state")
        .await
        .expect("preview should succeed");

    // After preview: a command is shown, but nothing executed.
    match session.view() {
        CmdKView::Previewed { command } => assert_eq!(command, "git status"),
        other => panic!("expected Previewed, got {other:?}"),
    }
    assert_eq!(
        backend.run_count(),
        0,
        "gate #9 VIOLATED: preview must not execute the command"
    );

    // Explicit confirm is the only path to a runnable command.
    let to_run = session.confirm_run().expect("confirm should yield command");
    assert_eq!(to_run, "git status");
}

/// Gate #10: a failed command (non-zero exit) is explainable; a successful one
/// is not offered an explanation.
#[test]
fn gate_10_explainer_only_offers_on_failure() {
    use bongterm_devassist::ai::explainer::Explainer;

    assert!(
        Explainer::is_explainable(Some(1)),
        "gate #10: non-zero exit must be explainable"
    );
    assert!(
        Explainer::is_explainable(Some(127)),
        "gate #10: command-not-found must be explainable"
    );
    assert!(
        !Explainer::is_explainable(Some(0)),
        "gate #10: zero exit must NOT be offered an explanation"
    );
    assert!(
        !Explainer::is_explainable(None),
        "gate #10: still-running command is not explainable"
    );
}

/// Gate #11: smart history applies filters then ranks survivors by frecency.
#[test]
fn gate_11_smart_history_filters_then_ranks() {
    use bongterm_devassist::history::filter::HistoryQuery;

    // A query string with a filter token plus free text parses both parts.
    let q = HistoryQuery::parse("exit:1 cargo build");
    assert!(
        q.has_filter(),
        "gate #11: `exit:1` must parse as a filter"
    );
    assert_eq!(q.free_text(), "cargo build");
}

fn claude_on_path() -> bool {
    std::process::Command::new("claude")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Gate #9 (real binary): when `claude` is installed, a preview round-trip
/// returns a non-empty suggestion and still does not auto-run.
#[tokio::test]
async fn gate_9_real_claude_preview_when_installed() {
    if !claude_on_path() {
        eprintln!("skipping: `claude` not on PATH (nightly-only gate)");
        return;
    }
    use bongterm_devassist::ai::runner::{AiBackend, ClaudeCodeAiRunner};

    let runner = ClaudeCodeAiRunner::discover().expect("discover claude");
    assert!(
        runner.availability().is_available(),
        "gate #9: discovered runner must report available"
    );
    let suggestion = runner
        .suggest("list files in the current directory")
        .await
        .expect("suggest should succeed");
    assert!(
        !suggestion.command.trim().is_empty(),
        "gate #9: real backend must return a non-empty preview"
    );
}
```

- [ ] **(2) Run, expect FAIL**: `cargo test -p bongterm-app --test phase3_gates` →
  - First failure: `error[E0432]: unresolved import \`bongterm_devassist::ai::cmdk::CmdKView\`` (or whichever 3.A/3.B symbol the dev-deps don't yet expose), because the test crate's `[dev-dependencies]` were just added and the referenced public API (`MockAiBackend::run_count`, `CmdKSession::view`, `HistoryQuery::free_text`) must be in place from 3.A/3.B/3.F.
  - Once the crate compiles but a runner regressed: a gate assertion message such as `gate #9 VIOLATED: preview must not execute the command`.

- [ ] **(3) Minimal impl** — wire any missing surface so the gate tests compile and pass:
  - `MockAiBackend::run_count(&self) -> usize` — add to `crates/bongterm-test-kit/src/mocks/ai_backend.rs` (returns the recorded count of `run`/execute calls; it stays `0` because preview never executes).
  - `MockAiBackend::with_suggestion(cmd: &str)` and `Clone` — ensure present (3.A.1).
  - `CmdKSession::view()` + `CmdKView` — from 3.F.1.
  - `Explainer::is_explainable` — from 3.A.3.
  - `HistoryQuery::has_filter()` / `free_text()` — ensure these accessors exist on the parser from 3.B.1 (add thin getters if 3.B.1 exposed only the parsed struct fields).
  - `ClaudeCodeAiRunner::discover()` returning `Result<Self, DevassistError>` and `availability()` — from 3.A.4.
  - No new business logic is introduced here: every referenced item is a public accessor over types built in 3.A/3.B/3.F. If an accessor is missing, add it in its owning crate (not in the test), then re-run.

- [ ] **(4) Run, expect PASS**: `cargo test -p bongterm-app --test phase3_gates` → `gate_9_*`, `gate_10_*`, `gate_11_*` pass (real-claude test prints "skipping" and returns `Ok` when `claude` is absent).
- [ ] **(5) Commit**: `git add crates/bongterm-app crates/bongterm-test-kit && git commit -m "test(app/3.exit.1): gates #9-#11 — AI preview-no-spawn, explainer, smart history"`

### 3.exit.2 — Gate #12 + #13 + #14: snippets, background job toast, clickable patterns E2E

- **Files**:
  - Modify `crates/bongterm-app/tests/phase3_gates.rs` (append).

- [ ] **(1) Failing test** — append to `crates/bongterm-app/tests/phase3_gates.rs`:

```rust
/// Gate #12: a snippet with `${param:name}` is rendered to a runnable command
/// only after every parameter is supplied; a missing param is an error, not a
/// silent blank.
#[test]
fn gate_12_snippet_requires_all_params_before_run() {
    use bongterm_devassist::snippets::model::Snippet;
    use bongterm_devassist::snippets::render::render_snippet;
    use std::collections::HashMap;

    let lib = r#"{ snippets: [ { name: "deploy", scope: "workspace",
        command: "kubectl rollout restart deploy/${param:svc} -n ${param:ns}" } ] }"#;
    let snippet = Snippet::from_json5_library(lib)
        .expect("parse")
        .into_iter()
        .next()
        .expect("one snippet");

    let mut params = HashMap::new();
    params.insert("svc".to_string(), "api".to_string());
    // Missing `ns` => error, never a blank substitution.
    assert!(
        render_snippet(&snippet, &params).is_err(),
        "gate #12: missing param must error"
    );

    params.insert("ns".to_string(), "prod".to_string());
    let rendered = render_snippet(&snippet, &params).expect("all params present");
    assert_eq!(
        rendered,
        "kubectl rollout restart deploy/api -n prod"
    );
}

/// Gate #13: a background job that exits non-zero drives a Failed terminal
/// state AND fires exactly one completion toast carrying the exit code.
#[tokio::test]
async fn gate_13_failed_job_emits_failure_toast() {
    use bongterm_devassist::jobs::runner::{JobRunner, JobSpec, JobState};
    use bongterm_test_kit::mocks::notifier::MockNotifier;

    if !cfg!(windows) {
        eprintln!("skipping: needs a Windows shell");
        return;
    }

    let notifier = MockNotifier::new();
    let runner = JobRunner::new(Box::new(notifier.clone()));

    // `cmd /C exit 3` — fast, deterministic non-zero exit on Windows.
    let spec = JobSpec::shell("gate13", "cmd", &["/C", "exit 3"]);
    let outcome = runner.run_to_completion(spec).await.expect("run");

    match outcome.final_state {
        JobState::Failed { exit_code } => assert_eq!(exit_code, 3),
        other => panic!("gate #13: expected Failed{{3}}, got {other:?}"),
    }
    let toasts = notifier.toasts();
    assert_eq!(toasts.len(), 1, "gate #13: exactly one completion toast");
    assert!(
        toasts[0].body.contains('3'),
        "gate #13: toast must carry the exit code"
    );
}

/// Gate #14: clickable patterns detect a Rust `file:line:col`, a bare URL, and
/// an OSC 8 link — and an OSC 8 link whose visible text disagrees with its
/// destination is flagged as a spoof suspect (threat-model OSC 8 spoofing).
#[test]
fn gate_14_clickable_patterns_and_osc8_spoof_guard() {
    use bongterm_devassist::patterns::matchers::{scan_file_locations, PatternKind};
    use bongterm_devassist::patterns::url::{parse_osc8, scan_urls, verify_destination};

    let line = "error[E0432]: at src/lib.rs:12:9 — see https://example.com/docs";
    let spans = scan_file_locations(line);
    assert!(
        spans.iter().any(|s| matches!(s.kind, PatternKind::FileLine)),
        "gate #14: must detect src/lib.rs:12:9"
    );
    let urls = scan_urls(line);
    assert_eq!(urls.len(), 1, "gate #14: one bare URL");

    // OSC 8 link: ESC ] 8 ; ; URL ST  visible-text  ESC ] 8 ; ; ST
    let osc8 = "\x1b]8;;https://evil.example\x07Click here: https://bank.example\x1b]8;;\x07";
    let link = parse_osc8(osc8).expect("parse osc8");
    assert!(
        link.is_spoof_suspect(),
        "gate #14: visible URL != destination must be flagged"
    );

    // Scheme allowlist: only http/https destinations are followable.
    assert!(verify_destination("https://example.com").is_ok());
    assert!(
        verify_destination("file:///c:/windows/system32").is_err(),
        "gate #14: non-http(s) scheme must be rejected"
    );
    assert!(
        verify_destination("javascript:alert(1)").is_err(),
        "gate #14: javascript: scheme must be rejected"
    );
}
```

- [ ] **(2) Run, expect FAIL**: `cargo test -p bongterm-app --test phase3_gates gate_12` (and `gate_13`, `gate_14`) → `error[E0432]: unresolved import` for any snippet/job/pattern symbol not yet public, or a gate assertion failure if a feature regressed.
- [ ] **(3) Minimal impl** — confirm the public surfaces the gates exercise are present (all built in earlier groups; add thin accessors only if missing, in the owning crate):
  - `Snippet::from_json5_library`, `render_snippet` (3.C.1/3.C.2).
  - `JobRunner::new`, `JobSpec::shell`, `JobRunner::run_to_completion`, `JobOutcome::final_state`, `JobState::Failed{exit_code}` (3.D.1/3.D.2).
  - `MockNotifier::new`/`toasts`/`Clone`, `Toast::body` (3.D.1).
  - `scan_file_locations`, `PatternKind::FileLine`, `Span` (3.E.1); `scan_urls`, `parse_osc8`, `Osc8Link::is_spoof_suspect`, `verify_destination` (3.E.3).
- [ ] **(4) Run, expect PASS**: `cargo test -p bongterm-app --test phase3_gates` → all gate tests pass (gate #13 prints "skipping" on non-Windows). Then full sweep: `cargo test --workspace`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`, `cargo xtask check-deps` → all green.
- [ ] **(5) Commit**: `git add crates/bongterm-app && git commit -m "test(app/3.exit.2): gates #12-#14 — snippets, job toast, clickable + OSC 8 spoof guard"`

---

## Self-Review

### Coverage: every Phase 3 outline task maps to a plan task

| `orca.md` Phase 3 outline item | Plan task(s) |
|---|---|
| `3.A.1` ai subprocess wrapper | 3.A.1 (`AiBackend` port + `MockAiBackend`), 3.A.4 (`ClaudeCodeAiRunner` real subprocess) |
| `3.A.2` Cmd-K preview-only + explicit Run | 3.A.2 (`CmdKSession` state machine), surfaced in 3.F.1 (`CmdKView`/banner) |
| `3.A.3` failed-command explainer | 3.A.3 (`Explainer`) |
| `3.A.4` "Claude Code not installed" fallback | 3.A.4 (`detect_backend`/`UnavailableBackend`/graceful unavailable) |
| `3.B.1` history filters `cwd:`/`branch:`/`agent:`/`exit:`/`time:`/`shell:`/`duration:` | 3.B.1 (`HistoryQuery`/`FilterKind` closed enum) |
| `3.B.2` frecency index in SQLite | 3.B.2 (`FrecencyRepo` port + conformance), 3.B.3 (`SqliteFrecencyRepo` + `0002_frecency`) |
| `3.B.3` Ctrl+R smart history | 3.B.4 (`SmartHistory::search` filter-then-frecency) |
| `3.C.1` snippets JSON5 + `${param:name}` | 3.C.1 (`Snippet`/JSON5 load/param parse) |
| `3.C.2` parameter prompt UI | 3.C.2 (`render_snippet`), 3.C.3 (`ParamPrompt` model) |
| `3.C.3` snippet scope workspace+global | 3.C.3 (`SnippetStore` scope merge) |
| `3.D.1` background jobs pane execution | 3.D.1 (`JobState`/`JobSpec`), 3.D.2 (`JobRunner::run_to_completion`) |
| `3.D.2` desktop toast (winrt Notifications) | 3.D.1 (`Notifier` port + `Toast`), 3.D.2 (toast on terminal state); Windows impl behind the `Notifier` port per Tech Stack |
| `3.D.3` job list panel | 3.D.3 (`JobList` view-model), surfaced in 3.F.1 (`job_panel_rows`) |
| `3.E.1` pattern matchers Node/Python/Rust/.NET/TS | 3.E.1 (`scan_file_locations` + `PatternKind`) |
| `3.E.2` clickable file:line spans (overlay only) | 3.E.2 (`ClickableOverlay`), surfaced in 3.F.1 (`clickable_regions`) |
| `3.E.3` URL detection + OSC 8 hyperlink rendering | 3.E.3 (`scan_urls`/`parse_osc8`/spoof guard/`verify_destination`) |
| `3.exit` Phase 3 exit gate §6.1 #9–#14 | 3.exit.1 (#9/#10/#11), 3.exit.2 (#12/#13/#14) |

### Coverage: §6.1 acceptance gates #9–#14

| Gate | Acceptance shape | Proven by |
|---|---|---|
| #9 AI assist preview-only, no auto-run | Preview never executes; only explicit confirm yields a runnable command | 3.A.2 (state machine unit), 3.exit.1 `gate_9_*` (E2E mock + real-claude), 3.F.1 (`run_enabled` gate in view) |
| #10 failed-command explainer | Explanation offered iff non-zero exit | 3.A.3 (unit), 3.exit.1 `gate_10_*` |
| #11 smart history (filters + frecency + Ctrl+R) | Filters parse; survivors ranked by frecency | 3.B.1/3.B.4 (unit), 3.exit.1 `gate_11_*` |
| #12 snippets with params | Missing param errors; full set renders exact command | 3.C.1/3.C.2 (unit), 3.exit.2 `gate_12_*` |
| #13 background jobs + toast | Non-zero exit ⇒ `Failed{code}` + exactly one toast carrying the code | 3.D.1/3.D.2 (unit), 3.exit.2 `gate_13_*` (real Windows shell) |
| #14 clickable patterns + OSC 8 | file:line + URL detected; OSC 8 spoof flagged; non-http(s) scheme rejected | 3.E.1/3.E.3 (unit), 3.exit.2 `gate_14_*` |

### Placeholder scan

- No `todo!()`, `unimplemented!()`, `// TODO`, `// ...`, or `...` left as load-bearing code in any `(3) Minimal impl` block. Every impl step lists concrete types/functions; where a step says "add a thin accessor if missing," it names the exact signature (e.g., `MockAiBackend::run_count(&self) -> usize`).
- All file paths are absolute-from-repo-root and consistent with the File Structure table.
- Every task has all five TDD steps (failing test with full code → run/expect-FAIL with exact message → minimal impl with full code → run/expect-PASS → commit with exact command). Wiring-only sub-steps are labelled `(0)`.

### Type-consistency check (cross-task)

- `Span { start: usize, end: usize }` (defined 3.E.1) is the single span type consumed by 3.E.2 and re-exported through 3.F.1 `clickable_regions`. No competing span shape.
- `CmdKView` (3 variants: `Idle` / `Previewed{command}` / `Unavailable{reason}`) is defined once in `ai/cmdk.rs` (3.F.1) as a projection of `CmdKState`; consumed identically by `cmdk_banner` (3.F.1) and the `gate_9_*` test (3.exit.1).
- `JobState` closed enum (`Pending`/`Running`/`Succeeded`/`Failed{exit_code:i32}`/`Cancelled`) from 3.D.1 is matched in 3.D.2, 3.D.3, and `gate_13_*` with the same `Failed{exit_code}` shape.
- `JobListSnapshot{rows: Vec<JobRowView>}` / `JobRowView{label, status_label, is_terminal}` (3.D.3) is the exact type `job_panel_rows` (3.F.1) consumes.
- `FrecencyRow{command, use_count, last_used_unix}` + `frecency_score(&FrecencyRow, now_unix) -> f64` (3.B.2 in `bongterm-storage-api`) is implemented by `SqliteFrecencyRepo` (3.B.3) and exercised by `MockFrecencyRepo` conformance (3.B.2) and `SmartHistory` (3.B.4) — one row shape, one scorer.
- `DevassistError` (variants `Backend`/`Unavailable`/`Parse`/`MissingParam`/`Storage`/`Job`) from 3.A.0 is the single error type returned across `ai`/`history`/`snippets`/`jobs`/`patterns`.
- `AiBackend` trait (`availability()`, `suggest()`) from 3.A.1 is implemented by `UnavailableBackend`, `ClaudeCodeAiRunner` (3.A.4), and `MockAiBackend` (test-kit) with identical signatures.
- Dependency-direction integrity: the only new inter-crate edges are `bongterm-ui → bongterm-devassist` (added to `allowed-deps.toml` in 3.F.1) and `bongterm-devassist → bongterm-storage-api` (frecency port, 3.B.2; already legal). `bongterm-devassist` does **not** depend on `rusqlite`; the SQLite impl stays in `bongterm-storage-sqlite`. `bongterm-app` already allows `bongterm-devassist`, so 3.exit adds no new matrix edge.

---
