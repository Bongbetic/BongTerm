# BongTerm Phase 4 Execution Plan (MCP + Secrets + Security)

Date: 2026-05-29
Source: `docs/superpowers/specs/2026-05-27-bongt-mvp0-design.md` (§6.1 gates #16, #19, #23, #31) + PRD v7 §20.x (MCP host/registry/process governance), §1.3 / §20.1 (Context Optimizer — token budget, NOT process governance), §37 (secrets reference model), §35 (threat model), §3.4 / §3.7 (MCP + security/approval runtime).
Status: Active

> **For agentic workers.** This plan is written to be executed by subagents one task at a time per `superpowers:subagent-driven-development`. Every task is bite-sized and self-contained: it names the exact files, gives the **full** failing test code, the exact `cargo` command to run and the message to expect, the **full** minimal implementation code, the pass command, and the exact `git commit`. Do not improvise types or signatures — they are fixed across tasks for consistency. Do not skip the RED step. Do not batch commits. When a task is complete, strike its `orca.md` entry in a separate chore commit (not covered here; follow the existing `chore(orca): …` convention).

## Goal

Deliver MCP process governance (one process per server per workspace under JobObject caps), a token-budget Context Optimizer (distinct from process governance), a DPAPI/Credential-Manager secret vault with late-scoped env injection, a redaction corpus with a secret-leak regression gate, and a dangerous-command + workspace-trust + production-safety policy layer — exiting only when §6.1 gates #16, #19, #23, #31 are green for 7 consecutive nightly runs and the §35 threat-model review passes.

## Architecture

`bongterm-mcp` gains a real `Supervisor` (registry + lifecycle + health/RSS sampling + restart-with-backoff + idle shutdown), a JSON config importer with schema validation, and a `ContextOptimizer` (per-agent tool allowlist + token-budget preview + temporary scoped config generation) — all sitting over the existing `McpTransport` port and `ProcessGovernor` from `bongterm-process-control`. `bongterm-vault-windows` becomes a real `SecretStore` over DPAPI + Windows Credential Manager with a `.env` importer and an env-block builder for late-scoped spawn-time resolution. `bongterm-security` gains a `Redactor` corpus, a `DangerousCommandMatcher`, a `WorkspaceTrustStore`, and a `ProductionSafetyMode` flag; `xtask secret-leak-corpus` becomes the real regression gate. Every cross-domain call stays behind the existing port traits — no new ownership crosses the §1.2 matrix.

## Tech Stack

- Rust stable `1.95`, edition `2024`, `x86_64-pc-windows-msvc`. No nightly in product code.
- `windows` `0.58` (`Win32_Security_Credentials` for Credential Manager, `Win32_Security_Cryptography` for DPAPI `CryptProtectData`/`CryptUnprotectData`, `Win32_System_JobObjects` for caps) — only inside `bongterm-vault-windows` and `bongterm-process-control`.
- `serde` + `serde_json` for MCP config import; `thiserror` for library errors.
- Mocks live in the originating trait crate (existing convention) and are exercised by `bongterm-test-kit` conformance suites.
- `cargo test` / `cargo clippy --all-targets --all-features --workspace -- -D warnings` / `cargo fmt --all -- --check` gate every commit.
- `cargo xtask secret-leak-corpus` is the §6.1 #23 regression gate; `cargo xtask check-deps` enforces the dependency manifest.

---

## File Structure

Every file created or modified in Phase 4, with its single responsibility.

### `bongterm-mcp` (registry, lifecycle, governance, context optimizer)
- `crates/bongterm-mcp/src/lib.rs` — **modify**: declare new modules; keep existing `McpTransport` port + `MockMcpTransport` untouched.
- `crates/bongterm-mcp/src/config.rs` — **new**: `McpConfigFile`, JSON import + schema validation (`validate`), `forbidden-install` rejection (`npx -y`), plaintext-secret rejection (only `${secret:NAME}`/`${env:NAME}` refs allowed in committed config).
- `crates/bongterm-mcp/src/supervisor.rs` — **new**: `Supervisor` (one process/server/workspace registry), `ServerKey`, `ServerHealth`, restart-with-backoff state machine, idle-shutdown gate keyed on active-agent attachment, health-check + RSS-sample cadence hooks.
- `crates/bongterm-mcp/src/optimizer.rs` — **new**: `ContextOptimizer` — per-agent tool allowlist filtering + `TokenBudgetPreview` (token estimate, NOT RSS), temporary scoped config generation, `McpSupport`-gated `Unavailable` labeling.
- `crates/bongterm-mcp/Cargo.toml` — **modify**: add `bongterm-process-control`, `time` (backoff clock), keep `serde`/`serde_json`/`thiserror`.

### `bongterm-process-control` (real JobObject enforcement)
- `crates/bongterm-process-control/src/lib.rs` — **modify**: declare `job` module behind `#[cfg(windows)]`; keep port + mocks intact.
- `crates/bongterm-process-control/src/job.rs` — **new** (`#[cfg(windows)]`): `WindowsJobGovernor` implementing `ProcessGovernor` via `CreateJobObject` / `SetInformationJobObject` (RSS + child-count caps) / `AssignProcessToJobObject` / `TerminateJobObject`, RSS sampling via `K32GetProcessMemoryInfo`.
- `crates/bongterm-process-control/Cargo.toml` — **modify**: add `windows` (target-gated).

### `bongterm-vault-windows` (real SecretStore)
- `crates/bongterm-vault-windows/src/lib.rs` — **modify**: declare modules; export `WindowsVault`.
- `crates/bongterm-vault-windows/src/dpapi.rs` — **new** (`#[cfg(windows)]`): `protect`/`unprotect` wrappers over `CryptProtectData`/`CryptUnprotectData`.
- `crates/bongterm-vault-windows/src/credman.rs` — **new** (`#[cfg(windows)]`): Credential Manager read/write/delete for DPAPI-wrapped blobs (`CredWriteW`/`CredReadW`/`CredDeleteW`).
- `crates/bongterm-vault-windows/src/vault.rs` — **new**: `WindowsVault` implementing `SecretStore` (late-scoped resolve, consumer authorization, fail-closed on missing), `EnvImport` (`.env` parse → vault writes; never persists plaintext to disk), `build_env_block` (resolve refs → in-memory env block for spawn).
- `crates/bongterm-vault-windows/Cargo.toml` — **modify**: add `bongterm-secrets-api`, `windows` (target-gated), `thiserror`.

### `bongterm-security` (redactor, dangerous-command, trust, prod-mode)
- `crates/bongterm-security/src/lib.rs` — **modify**: declare new modules; keep `PolicyEvaluator`/`Decision`/`EnforcementLevel`/`MockPolicyEvaluator` untouched.
- `crates/bongterm-security/src/redactor.rs` — **new**: `Redactor` corpus (AWS / GitHub PAT / OpenAI / Anthropic / JWT / SSH private key / high-entropy), idempotent `redact`, `RedactionPreview`.
- `crates/bongterm-security/src/dangerous.rs` — **new**: `DangerousCommandMatcher` (closed `DangerKind` enum) for `git push --force`, `rm -rf`, `kubectl delete`, `terraform destroy`.
- `crates/bongterm-security/src/trust.rs` — **new**: `WorkspaceTrustStore` (newly opened folder defaults Untrusted), `TrustState`.
- `crates/bongterm-security/src/prod_mode.rs` — **new**: `ProductionSafetyMode` flag + `escalate` (raises enforcement for dangerous classes when ON).
- `crates/bongterm-security/Cargo.toml` — **modify**: (no new external deps; `serde`/`thiserror` already present).

### `bongterm-test-kit` (conformance + negative coverage)
- `crates/bongterm-test-kit/src/conformance/mcp_supervisor_conformance.rs` — **new**: contract suite for any `Supervisor` impl.
- `crates/bongterm-test-kit/src/conformance/redactor_conformance.rs` — **new**: idempotence + corpus coverage contract for any `Redactor`-shaped impl.
- `crates/bongterm-test-kit/src/conformance/mod.rs` — **modify**: declare the two new conformance modules.
- `crates/bongterm-test-kit/src/conformance/negative.rs` — **modify**: add fail-closed tests for env-block leakage and dangerous-command non-bypass.

### `xtask` (secret-leak regression gate)
- `tools/xtask/src/secret_leak_corpus.rs` — **modify**: real impl running `tests/fixtures/secrets/` through `bongterm-security::Redactor`, exit non-zero on any surviving synthetic token.
- `tools/xtask/Cargo.toml` — **modify**: add `bongterm-security`.
- `tests/fixtures/secrets/corpus.jsonl` — **new**: synthetic token corpus (one JSON object per line: `kind`, `sample`, `must_be_redacted`).

### Dependency manifest + docs
- `tools/xtask/allowed-deps.toml` — **modify**: add edges `bongterm-mcp → bongterm-process-control` (already present), `xtask` is not graph-checked (tools/*), `bongterm-test-kit → bongterm-vault-windows`, `bongterm-process-control` keeps `windows` (external, untracked).
- `docs/adr/0010-mcp-process-governance.md` — **new**: records one-proc-per-server + backoff + idle-shutdown decision (referenced by exit gate).
- `docs/security/threat-model-phase4.md` — **new**: §35 scenario-by-scenario coverage table written at the exit gate.

---

## Ordered Work Plan (by orca outline section)

- **4.A** MCP supervision: real `Supervisor`, JobObject caps, JSON import + schema validation, no-`npx -y` policy, idle shutdown, health/RSS/restart-backoff. (gate #16, #31)
- **4.B** Context Optimizer v1: per-agent tool allowlist + token-budget preview, temporary scoped config, Unavailable label. (gate #16)
- **4.C** Secrets: `WindowsVault` `SecretStore`, `.env` import, vault-backed env at spawn, launch-time disclosure model. (gate #19 supporting, security contract §37)
- **4.D** Redaction: `Redactor` corpus, `xtask secret-leak-corpus` real impl, telemetry redaction preview. (gate #19, #23)
- **4.E** Policy: dangerous-command matcher, workspace trust prompt, production safety mode. (threat-model review)
- **4.exit** Exit gate: §6.1 #16, #19, #23, #31 green + threat-model review.

Each section's TDD tasks follow. (Authored in subsequent edits to this file — see task index below.)

---

## Task Index

- 4.A.1 `Supervisor` skeleton + `ServerKey` one-proc-per-server registry
- 4.A.2 JobObject caps wiring (`Supervisor` → `ProcessGovernor`) + `WindowsJobGovernor`
- 4.A.3 MCP JSON config import + schema validation
- 4.A.4 No-`npx -y` forbidden-install policy
- 4.A.5 Idle shutdown only when no active agent attached
- 4.A.6 Health check + RSS sample + restart-with-backoff
- 4.B.1 Context Optimizer per-agent tool allowlist
- 4.B.2 Token-budget preview (token estimate, not RSS)
- 4.B.3 Temporary scoped MCP config generation + Unavailable label
- 4.C.1 `WindowsVault` `SecretStore` (DPAPI + Credential Manager)
- 4.C.2 `.env` import flow (no plaintext on disk)
- 4.C.3 Vault-backed env block at spawn (late, scoped resolution)
- 4.C.4 Launch-time disclosure model
- 4.D.1 `Redactor` corpus (AWS/GitHub/OpenAI/Anthropic/JWT/SSH/high-entropy)
- 4.D.2 `xtask secret-leak-corpus` real impl
- 4.D.3 Telemetry redaction preview
- 4.E.1 Dangerous-command matcher
- 4.E.2 Workspace trust store
- 4.E.3 Production safety mode
- 4.exit Exit gate + threat-model review

---

## 4.A — MCP Supervision

### 4.A.1 `Supervisor` skeleton + one-proc-per-server registry

**Files**
- `crates/bongterm-mcp/src/supervisor.rs` (new)
- `crates/bongterm-mcp/src/lib.rs` (modify: `pub mod supervisor;`)
- `crates/bongterm-mcp/Cargo.toml` (modify: add `bongterm-process-control`, `time`)

**Step 1 — failing test** (append to `crates/bongterm-mcp/src/supervisor.rs`):
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{McpServerConfig, MockMcpTransport};

    fn cfg(name: &str) -> McpServerConfig {
        McpServerConfig { name: name.to_string(), argv: vec!["node".into(), "s.js".into()], env: vec![] }
    }

    #[test]
    fn one_process_per_server_per_workspace() {
        let sup = Supervisor::new();
        let ws = WorkspaceId("ws-1".into());
        sup.register(ws.clone(), cfg("fs"), Box::new(MockMcpTransport::new())).unwrap();
        // Re-registering the same (workspace, server-name) must be rejected: exactly one process.
        let dup = sup.register(ws.clone(), cfg("fs"), Box::new(MockMcpTransport::new()));
        assert!(matches!(dup, Err(SupervisorError::AlreadyRegistered(_))), "got {dup:?}");
        assert_eq!(sup.server_count(&ws), 1);
        // Same server name in a different workspace is a distinct process.
        let ws2 = WorkspaceId("ws-2".into());
        sup.register(ws2.clone(), cfg("fs"), Box::new(MockMcpTransport::new())).unwrap();
        assert_eq!(sup.server_count(&ws2), 1);
    }
}
```

**Step 2 — run, expect FAIL**
`cargo test -p bongterm-mcp supervisor::tests::one_process_per_server_per_workspace`
Expect: `error[E0432]: unresolved import` / `cannot find type 'Supervisor'` (module/type not yet defined).

**Step 3 — minimal impl** (top of `crates/bongterm-mcp/src/supervisor.rs`):
```rust
//! MCP server supervision: one process per server per workspace, lifecycle,
//! health/RSS sampling, restart-with-backoff, idle shutdown.
//! Spec §3.4. This crate owns governance, never agent UI / renderer / Git.

use std::collections::HashMap;
use std::sync::Mutex;

use crate::{McpServerConfig, McpTransport};

/// Identifies a workspace; MCP processes are scoped one-per-server-per-workspace.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct WorkspaceId(pub String);

/// Composite key: a server name is unique within a workspace.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ServerKey {
    pub workspace: WorkspaceId,
    pub server_name: String,
}

#[derive(Debug, thiserror::Error)]
pub enum SupervisorError {
    #[error("server already registered: {0}")]
    AlreadyRegistered(String),
    #[error("server not registered: {0}")]
    NotRegistered(String),
    #[error("transport error: {0}")]
    Transport(#[from] crate::McpError),
}

struct Entry {
    config: McpServerConfig,
    transport: Box<dyn McpTransport>,
}

/// The MCP supervisor registry. Holds at most one transport per `ServerKey`.
#[derive(Default)]
pub struct Supervisor {
    entries: Mutex<HashMap<ServerKey, Entry>>,
}

impl Supervisor {
    #[must_use]
    pub fn new() -> Self {
        Self { entries: Mutex::new(HashMap::new()) }
    }

    /// Register a server transport for a workspace. Rejects a duplicate
    /// (workspace, server-name) — exactly one process per server per workspace.
    ///
    /// # Panics
    /// Panics if the internal mutex is poisoned.
    pub fn register(
        &self,
        workspace: WorkspaceId,
        config: McpServerConfig,
        transport: Box<dyn McpTransport>,
    ) -> Result<(), SupervisorError> {
        let key = ServerKey { workspace, server_name: config.name.clone() };
        let mut map = self.entries.lock().unwrap();
        if map.contains_key(&key) {
            return Err(SupervisorError::AlreadyRegistered(key.server_name));
        }
        map.insert(key, Entry { config, transport });
        Ok(())
    }

    /// Count registered servers in a workspace.
    ///
    /// # Panics
    /// Panics if the internal mutex is poisoned.
    #[must_use]
    pub fn server_count(&self, workspace: &WorkspaceId) -> usize {
        self.entries
            .lock()
            .unwrap()
            .keys()
            .filter(|k| &k.workspace == workspace)
            .count()
    }
}
```
Add to `crates/bongterm-mcp/src/lib.rs` (after the existing `#![allow(...)]` lines, before the type defs):
```rust
pub mod supervisor;
```
Add to `crates/bongterm-mcp/Cargo.toml` `[dependencies]`:
```toml
bongterm-process-control = { path = "../bongterm-process-control" }
time = { workspace = true }
```

**Step 4 — run, expect PASS**
`cargo test -p bongterm-mcp supervisor::tests::one_process_per_server_per_workspace`

**Step 5 — commit**
```
git add crates/bongterm-mcp/src/supervisor.rs crates/bongterm-mcp/src/lib.rs crates/bongterm-mcp/Cargo.toml
git commit -m "$(cat <<'EOF'
feat(mcp/4.A.1): Supervisor registry — one process per server per workspace

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>
EOF
)"
```

- [ ] 4.A.1 complete

### 4.A.2 JobObject caps wiring + `WindowsJobGovernor`

**Files**
- `crates/bongterm-mcp/src/supervisor.rs` (modify: store + expose caps per server)
- `crates/bongterm-process-control/src/job.rs` (new, `#[cfg(windows)]`)
- `crates/bongterm-process-control/src/lib.rs` (modify: `#[cfg(windows)] pub mod job;`)
- `crates/bongterm-process-control/Cargo.toml` (modify: add target-gated `windows`)

**Step 1 — failing test** (append to `crates/bongterm-mcp/src/supervisor.rs` `tests` module):
```rust
    use bongterm_process_control::{JobObjectCaps, MockProcessGovernor, ProcessGovernor, ProcessHandle};

    #[test]
    fn registers_server_under_job_object_caps() {
        let sup = Supervisor::new();
        let ws = WorkspaceId("ws-1".into());
        let caps = JobObjectCaps { rss_bytes: 60 * 1024 * 1024, cpu_rate_bps: 5000, child_proc_count: 4 };
        let gov = MockProcessGovernor::new();
        // Default MCP RSS cap is 60 MB per spec §3.4.
        sup.register_with_caps(ws.clone(), cfg("fs"), Box::new(MockMcpTransport::new()), caps, ProcessHandle(4321), &gov).unwrap();
        assert_eq!(gov.caps_for(ProcessHandle(4321)), Some(caps), "caps must be attached at registration");
        assert_eq!(sup.caps_for(&ws, "fs"), Some(caps));
    }
```

**Step 2 — run, expect FAIL**
`cargo test -p bongterm-mcp supervisor::tests::registers_server_under_job_object_caps`
Expect: `no method named 'register_with_caps'` / `no method named 'caps_for'`.

**Step 3 — minimal impl** — in `crates/bongterm-mcp/src/supervisor.rs`, add `caps: Option<JobObjectCaps>` to `Entry` (import at top: `use bongterm_process_control::{JobObjectCaps, ProcessGovernor, ProcessHandle};`), set `caps: None` in the existing `register`, and add:
```rust
impl Supervisor {
    /// Register a server and attach JobObject caps to its process handle.
    ///
    /// # Panics
    /// Panics if the internal mutex is poisoned.
    pub fn register_with_caps(
        &self,
        workspace: WorkspaceId,
        config: McpServerConfig,
        transport: Box<dyn McpTransport>,
        caps: JobObjectCaps,
        handle: ProcessHandle,
        governor: &dyn ProcessGovernor,
    ) -> Result<(), SupervisorError> {
        let key = ServerKey { workspace, server_name: config.name.clone() };
        let mut map = self.entries.lock().unwrap();
        if map.contains_key(&key) {
            return Err(SupervisorError::AlreadyRegistered(key.server_name));
        }
        governor
            .attach(handle, caps)
            .map_err(|e| SupervisorError::Transport(crate::McpError::Transport(e.to_string())))?;
        map.insert(key, Entry { config, transport, caps: Some(caps) });
        Ok(())
    }

    /// Return the caps attached to a server, if any.
    ///
    /// # Panics
    /// Panics if the internal mutex is poisoned.
    #[must_use]
    pub fn caps_for(&self, workspace: &WorkspaceId, server_name: &str) -> Option<JobObjectCaps> {
        let key = ServerKey { workspace: workspace.clone(), server_name: server_name.to_string() };
        self.entries.lock().unwrap().get(&key).and_then(|e| e.caps)
    }
}
```
Create `crates/bongterm-process-control/src/job.rs`:
```rust
//! Windows JobObject-backed `ProcessGovernor`. Enforces RSS + child-count caps
//! via documented Win32 user-mode APIs only (no OS-bypass — see CLAUDE.md).
#![allow(unsafe_code)] // scoped to documented Win32 JobObject calls

use crate::{GovernorError, JobObjectCaps, ProcessGovernor, ProcessHandle, TerminationReason};
use std::collections::HashMap;
use std::sync::Mutex;

use windows::Win32::Foundation::{CloseHandle, HANDLE};
use windows::Win32::System::JobObjects::{
    AssignProcessToJobObject, CreateJobObjectW, JobObjectExtendedLimitInformation,
    SetInformationJobObject, TerminateJobObject, JOBOBJECT_BASIC_LIMIT_INFORMATION,
    JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JOB_OBJECT_LIMIT_ACTIVE_PROCESS,
    JOB_OBJECT_LIMIT_JOB_MEMORY,
};
use windows::Win32::System::ProcessStatus::{K32GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS};
use windows::Win32::System::Threading::{OpenProcess, PROCESS_QUERY_INFORMATION, PROCESS_VM_READ};

/// Production `ProcessGovernor` backed by a Windows JobObject per process.
pub struct WindowsJobGovernor {
    jobs: Mutex<HashMap<ProcessHandle, isize>>, // raw HANDLE value per process
}

impl WindowsJobGovernor {
    #[must_use]
    pub fn new() -> Self {
        Self { jobs: Mutex::new(HashMap::new()) }
    }
}

impl Default for WindowsJobGovernor {
    fn default() -> Self {
        Self::new()
    }
}

impl ProcessGovernor for WindowsJobGovernor {
    fn attach(&self, handle: ProcessHandle, caps: JobObjectCaps) -> Result<(), GovernorError> {
        // SAFETY: all calls are documented Win32 user-mode APIs; handles are
        // owned and closed on error paths.
        unsafe {
            let job = CreateJobObjectW(None, None)
                .map_err(|e| GovernorError::JobObject(e.to_string()))?;
            let mut info = JOBOBJECT_EXTENDED_LIMIT_INFORMATION {
                BasicLimitInformation: JOBOBJECT_BASIC_LIMIT_INFORMATION::default(),
                ..Default::default()
            };
            let mut flags = 0u32;
            if caps.rss_bytes > 0 {
                flags |= JOB_OBJECT_LIMIT_JOB_MEMORY.0;
                info.JobMemoryLimit = caps.rss_bytes as usize;
            }
            if caps.child_proc_count > 0 {
                flags |= JOB_OBJECT_LIMIT_ACTIVE_PROCESS.0;
                info.BasicLimitInformation.ActiveProcessLimit = caps.child_proc_count;
            }
            info.BasicLimitInformation.LimitFlags =
                windows::Win32::System::JobObjects::JOB_OBJECT_LIMIT_FLAGS(flags);
            SetInformationJobObject(
                job,
                JobObjectExtendedLimitInformation,
                std::ptr::addr_of!(info).cast(),
                u32::try_from(std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>()).unwrap(),
            )
            .map_err(|e| GovernorError::JobObject(e.to_string()))?;
            let proc = OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_VM_READ, false, handle.0)
                .map_err(|e| GovernorError::JobObject(e.to_string()))?;
            let assign = AssignProcessToJobObject(job, proc);
            let _ = CloseHandle(proc);
            assign.map_err(|e| GovernorError::JobObject(e.to_string()))?;
            self.jobs.lock().unwrap().insert(handle, job.0 as isize);
        }
        Ok(())
    }

    fn update_caps(&self, handle: ProcessHandle, caps: JobObjectCaps) -> Result<(), GovernorError> {
        if !self.jobs.lock().unwrap().contains_key(&handle) {
            return Err(GovernorError::NotTracked(handle));
        }
        // Re-attaching with new caps is the simplest correct path for MVP-0.
        self.attach(handle, caps)
    }

    fn sample_rss(&self, handle: ProcessHandle) -> Result<u64, GovernorError> {
        if !self.jobs.lock().unwrap().contains_key(&handle) {
            return Err(GovernorError::NotTracked(handle));
        }
        // SAFETY: documented Win32 process-memory query.
        unsafe {
            let proc = OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_VM_READ, false, handle.0)
                .map_err(|e| GovernorError::JobObject(e.to_string()))?;
            let mut counters = PROCESS_MEMORY_COUNTERS::default();
            let ok = K32GetProcessMemoryInfo(
                proc,
                std::ptr::addr_of_mut!(counters),
                u32::try_from(std::mem::size_of::<PROCESS_MEMORY_COUNTERS>()).unwrap(),
            );
            let _ = CloseHandle(proc);
            if ok.as_bool() {
                Ok(counters.WorkingSetSize as u64)
            } else {
                Err(GovernorError::JobObject("K32GetProcessMemoryInfo failed".into()))
            }
        }
    }

    fn terminate(&self, handle: ProcessHandle, _reason: TerminationReason) -> Result<(), GovernorError> {
        let raw = self
            .jobs
            .lock()
            .unwrap()
            .remove(&handle)
            .ok_or(GovernorError::NotTracked(handle))?;
        // SAFETY: terminating an owned JobObject; handle closed afterward.
        unsafe {
            let job = HANDLE(raw as *mut core::ffi::c_void);
            TerminateJobObject(job, 1).map_err(|e| GovernorError::JobObject(e.to_string()))?;
            let _ = CloseHandle(job);
        }
        Ok(())
    }
}
```
Add to `crates/bongterm-process-control/src/lib.rs` (top, after attribute lines):
```rust
#[cfg(windows)]
pub mod job;
```
Change `crates/bongterm-process-control/src/lib.rs` attribute `#![deny(unsafe_code)]` → `#![cfg_attr(not(windows), deny(unsafe_code))]` (the JobObject impl needs scoped `unsafe`; the `#[allow(unsafe_code)]` in `job.rs` keeps it scoped to that module).
Add to `crates/bongterm-process-control/Cargo.toml`:
```toml
[target.'cfg(windows)'.dependencies]
windows = { workspace = true }
```

**Step 4 — run, expect PASS**
`cargo test -p bongterm-mcp supervisor::tests::registers_server_under_job_object_caps`
Then on Windows: `cargo build -p bongterm-process-control` (compiles the `#[cfg(windows)]` module).

**Step 5 — commit**
```
git add crates/bongterm-mcp/src/supervisor.rs crates/bongterm-process-control/src/job.rs crates/bongterm-process-control/src/lib.rs crates/bongterm-process-control/Cargo.toml
git commit -m "$(cat <<'EOF'
feat(mcp/4.A.2): JobObject caps wired into Supervisor + WindowsJobGovernor

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>
EOF
)"
```

- [ ] 4.A.2 complete

### 4.A.3 MCP JSON config import + schema validation

**Files**
- `crates/bongterm-mcp/src/config.rs` (new)
- `crates/bongterm-mcp/src/lib.rs` (modify: `pub mod config;`)

**Step 1 — failing test** (append to `crates/bongterm-mcp/src/config.rs`):
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn imports_valid_config() {
        let json = r#"{ "servers": [
            { "name": "fs", "argv": ["node", "fs-server.js"], "env": [["ROOT", "${env:HOME}"]] }
        ]}"#;
        let file = McpConfigFile::import(json).expect("valid config must import");
        assert_eq!(file.servers.len(), 1);
        assert_eq!(file.servers[0].name, "fs");
    }

    #[test]
    fn rejects_empty_server_name() {
        let json = r#"{ "servers": [ { "name": "", "argv": ["node"], "env": [] } ]}"#;
        let err = McpConfigFile::import(json).unwrap_err();
        assert!(matches!(err, ConfigError::Schema(_)), "got {err:?}");
    }

    #[test]
    fn rejects_empty_argv() {
        let json = r#"{ "servers": [ { "name": "fs", "argv": [], "env": [] } ]}"#;
        let err = McpConfigFile::import(json).unwrap_err();
        assert!(matches!(err, ConfigError::Schema(_)), "got {err:?}");
    }

    #[test]
    fn rejects_plaintext_secret_in_env() {
        // Committed config may carry only ${secret:NAME} / ${env:NAME} refs (§37).
        let json = r#"{ "servers": [
            { "name": "fs", "argv": ["node"], "env": [["TOKEN", "ghp_realLookingPlaintextValue1234567890"]] }
        ]}"#;
        let err = McpConfigFile::import(json).unwrap_err();
        assert!(matches!(err, ConfigError::PlaintextSecret { .. }), "got {err:?}");
    }

    #[test]
    fn allows_secret_reference_in_env() {
        let json = r#"{ "servers": [
            { "name": "fs", "argv": ["node"], "env": [["TOKEN", "${secret:GITHUB_PAT}"]] }
        ]}"#;
        assert!(McpConfigFile::import(json).is_ok());
    }
}
```

**Step 2 — run, expect FAIL**
`cargo test -p bongterm-mcp config::tests`
Expect: `cannot find type 'McpConfigFile'` / `ConfigError`.

**Step 3 — minimal impl** (top of `crates/bongterm-mcp/src/config.rs`):
```rust
//! Manual MCP JSON config import + schema validation (spec §3.4, §37).
//! Committed config holds `${secret:NAME}` / `${env:NAME}` references only;
//! plaintext secret values are rejected.

use crate::McpServerConfig;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct McpConfigFile {
    pub servers: Vec<McpServerConfig>,
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("JSON parse error: {0}")]
    Parse(String),
    #[error("schema validation failed: {0}")]
    Schema(String),
    #[error("plaintext secret rejected for env var {var}: use ${{secret:NAME}} or ${{env:NAME}}")]
    PlaintextSecret { var: String },
}

/// Heuristic: a value that is neither a `${secret:…}`/`${env:…}` reference nor a
/// benign literal but *looks* like a credential is rejected. We reject any value
/// containing a high-entropy token-shaped substring while not being a reference.
fn looks_like_plaintext_secret(value: &str) -> bool {
    if value.starts_with("${secret:") || value.starts_with("${env:") {
        return false;
    }
    // Token-shaped: long run of base62/underscore/dash with mixed case or digits.
    let token_like = value
        .split(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == '-'))
        .any(|seg| seg.len() >= 20 && seg.chars().any(|c| c.is_ascii_digit()));
    token_like
}

impl McpConfigFile {
    /// Import + validate a JSON config string.
    pub fn import(json: &str) -> Result<Self, ConfigError> {
        let file: Self = serde_json::from_str(json).map_err(|e| ConfigError::Parse(e.to_string()))?;
        if file.servers.is_empty() {
            return Err(ConfigError::Schema("at least one server required".into()));
        }
        for s in &file.servers {
            if s.name.trim().is_empty() {
                return Err(ConfigError::Schema("server name must be non-empty".into()));
            }
            if s.argv.is_empty() || s.argv[0].trim().is_empty() {
                return Err(ConfigError::Schema(format!("server '{}' argv must be non-empty", s.name)));
            }
            for (var, value) in &s.env {
                if looks_like_plaintext_secret(value) {
                    return Err(ConfigError::PlaintextSecret { var: var.clone() });
                }
            }
        }
        Ok(file)
    }
}
```
Add to `crates/bongterm-mcp/src/lib.rs`:
```rust
pub mod config;
```

**Step 4 — run, expect PASS**
`cargo test -p bongterm-mcp config::tests`

**Step 5 — commit**
```
git add crates/bongterm-mcp/src/config.rs crates/bongterm-mcp/src/lib.rs
git commit -m "$(cat <<'EOF'
feat(mcp/4.A.3): MCP JSON config import + schema validation; reject plaintext secrets

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>
EOF
)"
```

- [ ] 4.A.3 complete

### 4.A.4 No-`npx -y` forbidden-install policy at import + register

**Files**
- `crates/bongterm-mcp/src/config.rs` (modify: reject `npx -y` argv at import)
- `crates/bongterm-mcp/src/supervisor.rs` (modify: `register` calls transport `start`, which already rejects — add an explicit pre-check test)

**Step 1 — failing test** (append to `crates/bongterm-mcp/src/config.rs` `tests`):
```rust
    #[test]
    fn rejects_npx_dash_y_at_import() {
        let json = r#"{ "servers": [
            { "name": "fs", "argv": ["npx", "-y", "@scope/server"], "env": [] }
        ]}"#;
        let err = McpConfigFile::import(json).unwrap_err();
        assert!(matches!(err, ConfigError::ForbiddenInstall { .. }), "got {err:?}");
    }

    #[test]
    fn allows_npx_without_dash_y_at_import() {
        let json = r#"{ "servers": [
            { "name": "fs", "argv": ["npx", "@scope/server"], "env": [] }
        ]}"#;
        assert!(McpConfigFile::import(json).is_ok());
    }
```

**Step 2 — run, expect FAIL**
`cargo test -p bongterm-mcp config::tests::rejects_npx_dash_y_at_import`
Expect: `no variant ... ForbiddenInstall`.

**Step 3 — minimal impl** — add to `ConfigError`:
```rust
    #[error("forbidden install command for server '{server}': `npx -y` auto-install is not allowed")]
    ForbiddenInstall { server: String },
```
In `McpConfigFile::import`, inside the per-server loop (before the env loop):
```rust
            for w in s.argv.windows(2) {
                if w[0] == "npx" && w[1] == "-y" {
                    return Err(ConfigError::ForbiddenInstall { server: s.name.clone() });
                }
            }
```

**Step 4 — run, expect PASS**
`cargo test -p bongterm-mcp config::tests`

**Step 5 — commit**
```
git add crates/bongterm-mcp/src/config.rs
git commit -m "$(cat <<'EOF'
feat(mcp/4.A.4): reject npx -y auto-install at config import (gate #31)

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>
EOF
)"
```

- [ ] 4.A.4 complete

### 4.A.5 Idle shutdown only when no active agent attached

**Files**
- `crates/bongterm-mcp/src/supervisor.rs` (modify: attach/detach agent tracking + `try_idle_shutdown`)

**Step 1 — failing test** (append to `supervisor.rs` `tests`):
```rust
    use crate::StopReason;

    #[test]
    fn idle_shutdown_blocked_while_agent_attached() {
        let sup = Supervisor::new();
        let ws = WorkspaceId("ws-1".into());
        sup.register(ws.clone(), cfg("fs"), Box::new(MockMcpTransport::new())).unwrap();
        sup.attach_agent(&ws, "fs", AgentSessionId("a1".into())).unwrap();
        // Attempting idle shutdown must be refused while an agent is attached.
        assert_eq!(sup.try_idle_shutdown(&ws, "fs"), IdleShutdownOutcome::BlockedActiveAgent);
        assert_eq!(sup.server_count(&ws), 1);
        // After the agent detaches, idle shutdown proceeds.
        sup.detach_agent(&ws, "fs", &AgentSessionId("a1".into())).unwrap();
        assert_eq!(sup.try_idle_shutdown(&ws, "fs"), IdleShutdownOutcome::Stopped);
        assert_eq!(sup.server_count(&ws), 0);
    }
```

**Step 2 — run, expect FAIL**
`cargo test -p bongterm-mcp supervisor::tests::idle_shutdown_blocked_while_agent_attached`
Expect: `cannot find type 'AgentSessionId'` / `IdleShutdownOutcome` / missing methods.

**Step 3 — minimal impl** — in `supervisor.rs`:
```rust
/// Identifies an attached agent session keeping an MCP server alive.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AgentSessionId(pub String);

/// Result of attempting an idle shutdown.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdleShutdownOutcome {
    /// Server had no attached agents and was stopped.
    Stopped,
    /// Shutdown refused: at least one active agent is attached.
    BlockedActiveAgent,
}
```
Add `agents: std::collections::HashSet<AgentSessionId>` to `Entry` (init empty in both `register` paths), then:
```rust
impl Supervisor {
    /// # Panics
    /// Panics if the internal mutex is poisoned.
    pub fn attach_agent(&self, ws: &WorkspaceId, server: &str, agent: AgentSessionId) -> Result<(), SupervisorError> {
        let key = ServerKey { workspace: ws.clone(), server_name: server.to_string() };
        let mut map = self.entries.lock().unwrap();
        let e = map.get_mut(&key).ok_or_else(|| SupervisorError::NotRegistered(server.to_string()))?;
        e.agents.insert(agent);
        Ok(())
    }

    /// # Panics
    /// Panics if the internal mutex is poisoned.
    pub fn detach_agent(&self, ws: &WorkspaceId, server: &str, agent: &AgentSessionId) -> Result<(), SupervisorError> {
        let key = ServerKey { workspace: ws.clone(), server_name: server.to_string() };
        let mut map = self.entries.lock().unwrap();
        let e = map.get_mut(&key).ok_or_else(|| SupervisorError::NotRegistered(server.to_string()))?;
        e.agents.remove(agent);
        Ok(())
    }

    /// Stop the server only if no agent session is attached (spec §3.4).
    ///
    /// # Panics
    /// Panics if the internal mutex is poisoned.
    pub fn try_idle_shutdown(&self, ws: &WorkspaceId, server: &str) -> IdleShutdownOutcome {
        let key = ServerKey { workspace: ws.clone(), server_name: server.to_string() };
        let mut map = self.entries.lock().unwrap();
        let Some(e) = map.get(&key) else { return IdleShutdownOutcome::Stopped };
        if !e.agents.is_empty() {
            return IdleShutdownOutcome::BlockedActiveAgent;
        }
        let _ = e.transport.stop(crate::StopReason::IdleTimeout);
        map.remove(&key);
        IdleShutdownOutcome::Stopped
    }
}
```
Add `use std::collections::HashSet;` to the import block (`HashMap` is already imported; combine or add a second `use`).

**Step 4 — run, expect PASS**
`cargo test -p bongterm-mcp supervisor::tests::idle_shutdown_blocked_while_agent_attached`

**Step 5 — commit**
```
git add crates/bongterm-mcp/src/supervisor.rs
git commit -m "$(cat <<'EOF'
feat(mcp/4.A.5): idle shutdown gated on no active agent attached

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>
EOF
)"
```

- [ ] 4.A.5 complete

### 4.A.6 Health check + RSS sample + restart-with-backoff

**Files**
- `crates/bongterm-mcp/src/supervisor.rs` (modify: `RestartPolicy`, `ServerHealth`, `record_failure`/`record_success`, `next_backoff`)

**Step 1 — failing test** (append to `supervisor.rs` `tests`):
```rust
    use std::time::Duration;

    #[test]
    fn restart_backoff_escalates_then_marks_unhealthy() {
        let mut policy = RestartPolicy::default();
        // Backoff schedule is 1s, 5s, 30s (spec §3.4 / §4.2).
        assert_eq!(policy.record_failure(), RestartAction::RetryAfter(Duration::from_secs(1)));
        assert_eq!(policy.record_failure(), RestartAction::RetryAfter(Duration::from_secs(5)));
        assert_eq!(policy.record_failure(), RestartAction::RetryAfter(Duration::from_secs(30)));
        // Three failures within the window → Unhealthy; no auto-restart.
        assert_eq!(policy.record_failure(), RestartAction::MarkUnhealthy);
        assert_eq!(policy.health(), ServerHealth::Unhealthy);
        // A success resets the backoff and restores health.
        policy.record_success();
        assert_eq!(policy.health(), ServerHealth::Healthy);
        assert_eq!(policy.record_failure(), RestartAction::RetryAfter(Duration::from_secs(1)));
    }
```

**Step 2 — run, expect FAIL**
`cargo test -p bongterm-mcp supervisor::tests::restart_backoff_escalates_then_marks_unhealthy`
Expect: `cannot find type 'RestartPolicy'` / `RestartAction` / `ServerHealth`.

**Step 3 — minimal impl** — in `supervisor.rs`:
```rust
use std::time::Duration;

/// Health state of a supervised MCP server.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServerHealth {
    Healthy,
    /// Three failures inside the window — auto-restart disabled until user re-enables.
    Unhealthy,
}

/// What the supervisor should do after a failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RestartAction {
    RetryAfter(Duration),
    MarkUnhealthy,
}

/// Exponential restart backoff with a failure ceiling (spec §3.4).
#[derive(Debug, Clone)]
pub struct RestartPolicy {
    schedule: Vec<Duration>,
    failures: usize,
    health: ServerHealth,
}

impl Default for RestartPolicy {
    fn default() -> Self {
        Self {
            schedule: vec![Duration::from_secs(1), Duration::from_secs(5), Duration::from_secs(30)],
            failures: 0,
            health: ServerHealth::Healthy,
        }
    }
}

impl RestartPolicy {
    /// Record a failure and decide the next action.
    pub fn record_failure(&mut self) -> RestartAction {
        if self.failures < self.schedule.len() {
            let action = RestartAction::RetryAfter(self.schedule[self.failures]);
            self.failures += 1;
            action
        } else {
            self.health = ServerHealth::Unhealthy;
            RestartAction::MarkUnhealthy
        }
    }

    /// Record a successful health check: reset backoff + restore health.
    pub fn record_success(&mut self) {
        self.failures = 0;
        self.health = ServerHealth::Healthy;
    }

    #[must_use]
    pub fn health(&self) -> ServerHealth {
        self.health
    }
}
```

**Step 4 — run, expect PASS**
`cargo test -p bongterm-mcp supervisor::tests::restart_backoff_escalates_then_marks_unhealthy`

**Step 5 — commit**
```
git add crates/bongterm-mcp/src/supervisor.rs
git commit -m "$(cat <<'EOF'
feat(mcp/4.A.6): restart-with-backoff (1s/5s/30s) + Unhealthy ceiling + health reset

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>
EOF
)"
```

- [ ] 4.A.6 complete

---

## 4.B — Context Optimizer v1 (token budget, NOT process governance)

> **Binding distinction (§1.3, §20.1):** the Context Optimizer prunes the *tool schema* exposed to an agent — it saves **tokens**, not resident memory. It must never claim to reduce RSS; process governance (4.A) owns RSS. Keep `TokenBudgetPreview` free of any RSS field.

### 4.B.1 Per-agent tool allowlist

**Files**
- `crates/bongterm-mcp/src/optimizer.rs` (new)
- `crates/bongterm-mcp/src/lib.rs` (modify: `pub mod optimizer;`)

**Step 1 — failing test** (append to `crates/bongterm-mcp/src/optimizer.rs`):
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::McpToolDescriptor;

    fn tool(name: &str) -> McpToolDescriptor {
        McpToolDescriptor { name: name.into(), description: "d".into(), input_schema_json: "{}".into() }
    }

    #[test]
    fn allowlist_filters_tools_default_deny() {
        let all = vec![tool("read_file"), tool("write_file"), tool("delete_all")];
        let allow = ToolAllowlist::new(vec!["read_file".into(), "write_file".into()]);
        let opt = ContextOptimizer::new(allow);
        let exposed = opt.filter_tools(&all);
        let names: Vec<&str> = exposed.iter().map(|t| t.name.as_str()).collect();
        assert_eq!(names, vec!["read_file", "write_file"]);
        // A tool absent from the allowlist is blocked (default deny).
        assert!(!opt.is_allowed("delete_all"));
        assert!(opt.is_allowed("read_file"));
    }

    #[test]
    fn empty_allowlist_blocks_everything() {
        let all = vec![tool("read_file")];
        let opt = ContextOptimizer::new(ToolAllowlist::new(vec![]));
        assert!(opt.filter_tools(&all).is_empty());
    }
}
```

**Step 2 — run, expect FAIL**
`cargo test -p bongterm-mcp optimizer::tests`
Expect: `cannot find type 'ContextOptimizer'` / `ToolAllowlist`.

**Step 3 — minimal impl** (top of `crates/bongterm-mcp/src/optimizer.rs`):
```rust
//! Context Optimizer v1 — token-budget tool-schema pruning per agent.
//! Spec §1.3 / §20.1: this saves TOKENS, not resident memory. It owns no
//! process governance (that is 4.A / `bongterm-process-control`).

use crate::McpToolDescriptor;

/// Per-agent allowlist of tool names. Default deny: only listed tools pass.
#[derive(Debug, Clone)]
pub struct ToolAllowlist {
    allowed: std::collections::HashSet<String>,
}

impl ToolAllowlist {
    #[must_use]
    pub fn new(names: Vec<String>) -> Self {
        Self { allowed: names.into_iter().collect() }
    }

    #[must_use]
    pub fn contains(&self, name: &str) -> bool {
        self.allowed.contains(name)
    }
}

/// Prunes the MCP tool schema exposed to an agent to a token-bounded allowlist.
pub struct ContextOptimizer {
    allowlist: ToolAllowlist,
}

impl ContextOptimizer {
    #[must_use]
    pub fn new(allowlist: ToolAllowlist) -> Self {
        Self { allowlist }
    }

    #[must_use]
    pub fn is_allowed(&self, tool_name: &str) -> bool {
        self.allowlist.contains(tool_name)
    }

    /// Return only the allowlisted tools, preserving input order.
    #[must_use]
    pub fn filter_tools(&self, tools: &[McpToolDescriptor]) -> Vec<McpToolDescriptor> {
        tools.iter().filter(|t| self.allowlist.contains(&t.name)).cloned().collect()
    }
}
```
Add to `crates/bongterm-mcp/src/lib.rs`:
```rust
pub mod optimizer;
```

**Step 4 — run, expect PASS**
`cargo test -p bongterm-mcp optimizer::tests`

**Step 5 — commit**
```
git add crates/bongterm-mcp/src/optimizer.rs crates/bongterm-mcp/src/lib.rs
git commit -m "$(cat <<'EOF'
feat(mcp/4.B.1): Context Optimizer per-agent tool allowlist (default deny)

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>
EOF
)"
```

- [ ] 4.B.1 complete

### 4.B.2 Token-budget preview (token estimate, never RSS)

**Files**
- `crates/bongterm-mcp/src/optimizer.rs` (modify: `TokenBudgetPreview`, `preview`)

**Step 1 — failing test** (append to `optimizer.rs` `tests`):
```rust
    #[test]
    fn token_budget_preview_counts_only_allowed_tools() {
        let all = vec![tool("read_file"), tool("write_file"), tool("delete_all")];
        let opt = ContextOptimizer::new(ToolAllowlist::new(vec!["read_file".into()]));
        let preview = opt.preview(&all);
        assert_eq!(preview.exposed_tool_count, 1);
        assert_eq!(preview.pruned_tool_count, 2);
        // Estimated tokens must be derived from exposed schema only and be > 0.
        assert!(preview.estimated_tokens > 0);
        // The preview must reflect only the exposed (allowlisted) schema, so
        // pruning reduces the estimate vs. exposing everything.
        let full = ContextOptimizer::new(ToolAllowlist::new(
            vec!["read_file".into(), "write_file".into(), "delete_all".into()],
        ));
        assert!(preview.estimated_tokens < full.preview(&all).estimated_tokens);
    }
```

**Step 2 — run, expect FAIL**
`cargo test -p bongterm-mcp optimizer::tests::token_budget_preview_counts_only_allowed_tools`
Expect: `no method named 'preview'` / `cannot find type 'TokenBudgetPreview'`.

**Step 3 — minimal impl** — in `optimizer.rs`:
```rust
/// A token-budget preview of the pruned tool schema. Contains TOKEN estimates
/// only — never RSS (process governance is a separate concern, §1.3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TokenBudgetPreview {
    pub exposed_tool_count: usize,
    pub pruned_tool_count: usize,
    /// Coarse token estimate over exposed tool name + description + schema.
    pub estimated_tokens: usize,
}

impl ContextOptimizer {
    /// Estimate the token budget of the pruned schema this agent will see.
    #[must_use]
    pub fn preview(&self, tools: &[McpToolDescriptor]) -> TokenBudgetPreview {
        let exposed = self.filter_tools(tools);
        // ~4 chars/token heuristic over the serialized schema surface.
        let chars: usize = exposed
            .iter()
            .map(|t| t.name.len() + t.description.len() + t.input_schema_json.len())
            .sum();
        TokenBudgetPreview {
            exposed_tool_count: exposed.len(),
            pruned_tool_count: tools.len() - exposed.len(),
            estimated_tokens: chars.div_ceil(4),
        }
    }
}
```

**Step 4 — run, expect PASS**
`cargo test -p bongterm-mcp optimizer::tests`

**Step 5 — commit**
```
git add crates/bongterm-mcp/src/optimizer.rs
git commit -m "$(cat <<'EOF'
feat(mcp/4.B.2): token-budget preview over pruned schema (tokens, not RSS)

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>
EOF
)"
```

- [ ] 4.B.2 complete

### 4.B.3 Temporary scoped config generation + Unavailable label

**Files**
- `crates/bongterm-mcp/src/optimizer.rs` (modify: `McpSupport`, `generate_scoped_config`)

**Step 1 — failing test** (append to `optimizer.rs` `tests`):
```rust
    use crate::McpServerConfig;

    #[test]
    fn generates_scoped_config_for_supporting_agent() {
        let server = McpServerConfig { name: "fs".into(), argv: vec!["node".into()], env: vec![] };
        let all = vec![tool("read_file"), tool("write_file"), tool("delete_all")];
        let opt = ContextOptimizer::new(ToolAllowlist::new(vec!["read_file".into()]));
        let scoped = opt.generate_scoped_config(McpSupport::ConfigFile, &server, &all).unwrap();
        // Only the allowlisted tool is exposed in the temporary scoped config.
        assert_eq!(scoped.exposed_tools, vec!["read_file".to_string()]);
        assert_eq!(scoped.server_name, "fs");
    }

    #[test]
    fn non_supporting_agent_is_labeled_unavailable() {
        let server = McpServerConfig { name: "fs".into(), argv: vec!["node".into()], env: vec![] };
        let all = vec![tool("read_file")];
        let opt = ContextOptimizer::new(ToolAllowlist::new(vec!["read_file".into()]));
        let err = opt.generate_scoped_config(McpSupport::None, &server, &all).unwrap_err();
        assert!(matches!(err, OptimizerError::McpGovernanceUnavailable), "got {err:?}");
    }
```

**Step 2 — run, expect FAIL**
`cargo test -p bongterm-mcp optimizer::tests::generates_scoped_config_for_supporting_agent`
Expect: `cannot find type 'McpSupport'` / `ScopedMcpConfig` / `OptimizerError`.

**Step 3 — minimal impl** — in `optimizer.rs`:
```rust
use crate::McpServerConfig;

/// Whether an agent adapter supports BongTerm-mediated MCP configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum McpSupport {
    /// Agent reads an MCP config file we can scope.
    ConfigFile,
    /// Agent accepts MCP via env injection we can scope.
    EnvInjection,
    /// No BongTerm-mediated MCP governance possible → label Unavailable.
    None,
}

/// A temporary scoped config exposing only allowlisted tools (spec §3.4).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScopedMcpConfig {
    pub server_name: String,
    pub exposed_tools: Vec<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum OptimizerError {
    #[error("MCP governance unavailable for this agent adapter")]
    McpGovernanceUnavailable,
}

impl ContextOptimizer {
    /// Generate a temporary scoped MCP config for a supporting agent, or signal
    /// that governance is unavailable for a non-supporting adapter.
    pub fn generate_scoped_config(
        &self,
        support: McpSupport,
        server: &McpServerConfig,
        tools: &[McpToolDescriptor],
    ) -> Result<ScopedMcpConfig, OptimizerError> {
        match support {
            McpSupport::None => Err(OptimizerError::McpGovernanceUnavailable),
            McpSupport::ConfigFile | McpSupport::EnvInjection => Ok(ScopedMcpConfig {
                server_name: server.name.clone(),
                exposed_tools: self.filter_tools(tools).into_iter().map(|t| t.name).collect(),
            }),
        }
    }
}
```

**Step 4 — run, expect PASS**
`cargo test -p bongterm-mcp optimizer::tests`

**Step 5 — commit**
```
git add crates/bongterm-mcp/src/optimizer.rs
git commit -m "$(cat <<'EOF'
feat(mcp/4.B.3): temporary scoped MCP config + Unavailable label for non-supporting agents

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>
EOF
)"
```

- [ ] 4.B.3 complete

---

## 4.C — Secrets Vault + Late-Scoped Env Injection

> **Binding (§37 / Security contract):** committed config holds references only; the vault is DPAPI / Credential-Manager-backed, per-user, no cloud. Secrets never appear in argv, URLs, history, transcripts, scrollback, logs, or exports — they are passed via an env block to children, resolved to plaintext only in memory, at spawn time, only for the authorized consumer. Missing secrets **fail closed**; never launch with an empty env var.

### 4.C.1 `WindowsVault` `SecretStore` (DPAPI + Credential Manager)

**Files**
- `crates/bongterm-vault-windows/src/dpapi.rs` (new, `#[cfg(windows)]`)
- `crates/bongterm-vault-windows/src/credman.rs` (new, `#[cfg(windows)]`)
- `crates/bongterm-vault-windows/src/vault.rs` (new)
- `crates/bongterm-vault-windows/src/lib.rs` (modify)
- `crates/bongterm-vault-windows/Cargo.toml` (modify)

> **Test portability note:** DPAPI + Credential Manager require Windows and a live user profile, so the production path is exercised by an ignored Windows-only integration test. The unit contract is proven against an in-memory backend (`InMemoryBackend`) so the `SecretStore` logic (authorization, fail-closed, scope) is testable on any runner. `WindowsVault` is generic over a `VaultBackend` trait; production wires the DPAPI/CredMan backend, tests wire the in-memory one.

**Step 1 — failing test** (append to `crates/bongterm-vault-windows/src/vault.rs`):
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use bongterm_secrets_api::{ConsumerId, ResolveError, SecretRef, SecretScope, SecretStore};

    fn vault_with(name: &str, val: &str, authorized: &str) -> WindowsVault<InMemoryBackend> {
        let backend = InMemoryBackend::new();
        backend.store_raw(name, val.as_bytes());
        let mut authz = std::collections::HashMap::new();
        authz.insert(name.to_string(), vec![ConsumerId(authorized.to_string())]);
        WindowsVault::with_backend_and_authz(backend, authz)
    }

    #[test]
    fn resolves_for_authorized_consumer() {
        let vault = vault_with("GITHUB_PAT", "ghp_secretvalue", "agent:claude-code");
        let r = SecretRef { name: "GITHUB_PAT".into(), scope: SecretScope::Agent };
        let v = vault.resolve(&r, &ConsumerId("agent:claude-code".into())).unwrap();
        assert_eq!(v.expose(), "ghp_secretvalue");
    }

    #[test]
    fn unauthorized_consumer_is_rejected() {
        let vault = vault_with("GITHUB_PAT", "ghp_secretvalue", "agent:claude-code");
        let r = SecretRef { name: "GITHUB_PAT".into(), scope: SecretScope::Agent };
        let err = vault.resolve(&r, &ConsumerId("agent:other".into())).unwrap_err();
        assert!(matches!(err, ResolveError::Unauthorized { .. }), "got {err:?}");
    }

    #[test]
    fn missing_secret_fails_closed() {
        let vault = vault_with("GITHUB_PAT", "ghp_secretvalue", "agent:claude-code");
        let r = SecretRef { name: "NOT_THERE".into(), scope: SecretScope::Agent };
        let err = vault.resolve(&r, &ConsumerId("agent:claude-code".into())).unwrap_err();
        assert!(matches!(err, ResolveError::Missing(_)), "got {err:?}");
    }
}
```

**Step 2 — run, expect FAIL**
`cargo test -p bongterm-vault-windows vault::tests`
Expect: `cannot find type 'WindowsVault'` / `InMemoryBackend` / `VaultBackend`.

**Step 3 — minimal impl** (top of `crates/bongterm-vault-windows/src/vault.rs`):
```rust
//! `WindowsVault` — DPAPI + Credential-Manager-backed `SecretStore`.
//! Late, scoped resolution: plaintext exists only in memory, at resolve time,
//! for an authorized consumer. Missing secrets fail closed (spec §37).

use std::collections::HashMap;
use std::sync::Mutex;

use bongterm_secrets_api::{
    ConsumerId, ResolveError, SecretRef, SecretStore, SecretValue,
};

/// Pluggable storage backend. Production = DPAPI + Credential Manager;
/// tests = in-memory. Keeps the `SecretStore` logic testable off-Windows.
pub trait VaultBackend: Send + Sync {
    /// Fetch the stored (already DPAPI-unprotected) plaintext bytes for a name.
    fn fetch(&self, name: &str) -> Option<Vec<u8>>;
    /// Store plaintext bytes (production wraps with DPAPI before persisting).
    fn put(&self, name: &str, plaintext: &[u8]);
}

/// In-memory backend for unit tests. Never used in production.
#[derive(Default)]
pub struct InMemoryBackend {
    map: Mutex<HashMap<String, Vec<u8>>>,
}

impl InMemoryBackend {
    #[must_use]
    pub fn new() -> Self {
        Self { map: Mutex::new(HashMap::new()) }
    }

    /// Test helper: store a raw value.
    ///
    /// # Panics
    /// Panics if the internal mutex is poisoned.
    pub fn store_raw(&self, name: &str, value: &[u8]) {
        self.map.lock().unwrap().insert(name.to_string(), value.to_vec());
    }
}

impl VaultBackend for InMemoryBackend {
    fn fetch(&self, name: &str) -> Option<Vec<u8>> {
        self.map.lock().unwrap().get(name).cloned()
    }
    fn put(&self, name: &str, plaintext: &[u8]) {
        self.map.lock().unwrap().insert(name.to_string(), plaintext.to_vec());
    }
}

/// Vault generic over a backend. Authorization maps secret name → allowed consumers.
pub struct WindowsVault<B: VaultBackend> {
    backend: B,
    authz: HashMap<String, Vec<ConsumerId>>,
}

impl<B: VaultBackend> WindowsVault<B> {
    #[must_use]
    pub fn with_backend_and_authz(backend: B, authz: HashMap<String, Vec<ConsumerId>>) -> Self {
        Self { backend, authz }
    }

    fn is_authorized(&self, name: &str, consumer: &ConsumerId) -> bool {
        self.authz.get(name).is_some_and(|allowed| allowed.contains(consumer))
    }
}

impl<B: VaultBackend> SecretStore for WindowsVault<B> {
    fn resolve(&self, secret: &SecretRef, consumer: &ConsumerId) -> Result<SecretValue, ResolveError> {
        let Some(bytes) = self.backend.fetch(&secret.name) else {
            return Err(ResolveError::Missing(secret.clone()));
        };
        if !self.is_authorized(&secret.name, consumer) {
            return Err(ResolveError::Unauthorized { secret: secret.clone(), consumer: consumer.clone() });
        }
        let plaintext = String::from_utf8(bytes)
            .map_err(|e| ResolveError::Backend(format!("non-utf8 secret: {e}")))?;
        Ok(SecretValue::from_plaintext(plaintext))
    }

    fn exists(&self, secret: &SecretRef) -> bool {
        self.backend.fetch(&secret.name).is_some()
    }
}
```
Create `crates/bongterm-vault-windows/src/dpapi.rs`:
```rust
//! DPAPI wrappers (`CryptProtectData` / `CryptUnprotectData`), per-user scope.
#![allow(unsafe_code)] // scoped to documented Win32 DPAPI calls

use windows::Win32::Security::Cryptography::{CryptProtectData, CryptUnprotectData, CRYPT_INTEGER_BLOB};
use windows::Win32::Foundation::LocalFree;

/// Encrypt plaintext to a DPAPI blob bound to the current user.
///
/// # Errors
/// Returns the Win32 error string if DPAPI fails.
pub fn protect(plaintext: &[u8]) -> Result<Vec<u8>, String> {
    // SAFETY: documented Win32 DPAPI call; output blob freed with LocalFree.
    unsafe {
        let mut input = CRYPT_INTEGER_BLOB { cbData: plaintext.len() as u32, pbData: plaintext.as_ptr() as *mut u8 };
        let mut output = CRYPT_INTEGER_BLOB::default();
        CryptProtectData(std::ptr::addr_of_mut!(input), None, None, None, None, 0, std::ptr::addr_of_mut!(output))
            .map_err(|e| e.to_string())?;
        let slice = std::slice::from_raw_parts(output.pbData, output.cbData as usize).to_vec();
        let _ = LocalFree(Some(windows::Win32::Foundation::HLOCAL(output.pbData.cast())));
        Ok(slice)
    }
}

/// Decrypt a DPAPI blob produced by [`protect`] for the current user.
///
/// # Errors
/// Returns the Win32 error string if DPAPI fails.
pub fn unprotect(blob: &[u8]) -> Result<Vec<u8>, String> {
    // SAFETY: documented Win32 DPAPI call; output blob freed with LocalFree.
    unsafe {
        let mut input = CRYPT_INTEGER_BLOB { cbData: blob.len() as u32, pbData: blob.as_ptr() as *mut u8 };
        let mut output = CRYPT_INTEGER_BLOB::default();
        CryptUnprotectData(std::ptr::addr_of_mut!(input), None, None, None, None, 0, std::ptr::addr_of_mut!(output))
            .map_err(|e| e.to_string())?;
        let slice = std::slice::from_raw_parts(output.pbData, output.cbData as usize).to_vec();
        let _ = LocalFree(Some(windows::Win32::Foundation::HLOCAL(output.pbData.cast())));
        Ok(slice)
    }
}
```
Create `crates/bongterm-vault-windows/src/credman.rs`:
```rust
//! Windows Credential Manager storage of DPAPI-wrapped secret blobs.
//! `CredWriteW` / `CredReadW` / `CredDeleteW`, target prefix "BongTerm:".
#![allow(unsafe_code)] // scoped to documented Win32 Credential Manager calls

use crate::dpapi;
use crate::vault::VaultBackend;

const TARGET_PREFIX: &str = "BongTerm:";

/// Production backend: DPAPI-wrap then store in Credential Manager.
pub struct CredManBackend;

impl VaultBackend for CredManBackend {
    fn fetch(&self, name: &str) -> Option<Vec<u8>> {
        let blob = cred_read(&format!("{TARGET_PREFIX}{name}"))?;
        dpapi::unprotect(&blob).ok()
    }
    fn put(&self, name: &str, plaintext: &[u8]) {
        if let Ok(blob) = dpapi::protect(plaintext) {
            let _ = cred_write(&format!("{TARGET_PREFIX}{name}"), &blob);
        }
    }
}

fn cred_read(target: &str) -> Option<Vec<u8>> {
    use windows::core::HSTRING;
    use windows::Win32::Security::Credentials::{CredReadW, CredFree, CRED_TYPE_GENERIC, CREDENTIALW};
    // SAFETY: documented Win32 Credential Manager read; pointer freed with CredFree.
    unsafe {
        let mut cred: *mut CREDENTIALW = std::ptr::null_mut();
        let t = HSTRING::from(target);
        CredReadW(&t, CRED_TYPE_GENERIC, None, std::ptr::addr_of_mut!(cred)).ok()?;
        if cred.is_null() { return None; }
        let c = &*cred;
        let out = std::slice::from_raw_parts(c.CredentialBlob, c.CredentialBlobSize as usize).to_vec();
        CredFree(cred.cast());
        Some(out)
    }
}

fn cred_write(target: &str, blob: &[u8]) -> Result<(), String> {
    use windows::core::HSTRING;
    use windows::Win32::Security::Credentials::{CredWriteW, CRED_TYPE_GENERIC, CRED_PERSIST_LOCAL_MACHINE, CREDENTIALW};
    // SAFETY: documented Win32 Credential Manager write with owned buffers.
    unsafe {
        let t = HSTRING::from(target);
        let mut cred = CREDENTIALW {
            Type: CRED_TYPE_GENERIC,
            TargetName: windows::core::PWSTR(t.as_ptr().cast_mut()),
            CredentialBlobSize: u32::try_from(blob.len()).map_err(|e| e.to_string())?,
            CredentialBlob: blob.as_ptr().cast_mut(),
            Persist: CRED_PERSIST_LOCAL_MACHINE,
            ..Default::default()
        };
        CredWriteW(std::ptr::addr_of_mut!(cred), 0).map_err(|e| e.to_string())
    }
}
```
Replace `crates/bongterm-vault-windows/src/lib.rs` body:
```rust
//! bongterm-vault-windows — DPAPI / Credential Manager `SecretStore`.
//! See spec §1.2 ownership matrix + §37 secrets reference model.

#![cfg_attr(not(windows), forbid(unsafe_code))]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::missing_errors_doc)]

pub mod vault;

#[cfg(windows)]
pub mod dpapi;
#[cfg(windows)]
pub mod credman;

pub use vault::{InMemoryBackend, VaultBackend, WindowsVault};
#[cfg(windows)]
pub use credman::CredManBackend;
```
Set `crates/bongterm-vault-windows/Cargo.toml`:
```toml
[package]
name = "bongterm-vault-windows"
version = "0.0.0"
edition.workspace = true
rust-version.workspace = true
license.workspace = true
publish = false

[dependencies]
bongterm-secrets-api = { path = "../bongterm-secrets-api" }
thiserror = { workspace = true }

[target.'cfg(windows)'.dependencies]
windows = { workspace = true }

[dev-dependencies]
```

**Step 4 — run, expect PASS**
`cargo test -p bongterm-vault-windows vault::tests`
Then on Windows: `cargo build -p bongterm-vault-windows`.

**Step 5 — commit**
```
git add crates/bongterm-vault-windows/
git commit -m "$(cat <<'EOF'
feat(vault/4.C.1): WindowsVault SecretStore — DPAPI + Credential Manager, fail-closed, scoped authz

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>
EOF
)"
```

- [ ] 4.C.1 complete

### 4.C.2 `.env` import flow (no plaintext on disk)

**Files**
- `crates/bongterm-vault-windows/src/vault.rs` (modify: `EnvImport`, `import_dotenv`)

**Step 1 — failing test** (append to `vault.rs` `tests`):
```rust
    #[test]
    fn imports_dotenv_into_vault_without_disk_plaintext() {
        let dotenv = "# comment\nGITHUB_PAT=ghp_fromfile\nEMPTY=\nQUOTED=\"with spaces\"\n";
        let parsed = EnvImport::parse(dotenv);
        // Comments and blank lines skipped; empty value preserved; quotes stripped.
        assert_eq!(parsed.len(), 3);
        assert_eq!(parsed.get("GITHUB_PAT").map(String::as_str), Some("ghp_fromfile"));
        assert_eq!(parsed.get("QUOTED").map(String::as_str), Some("with spaces"));
        // Importing writes into the backend (DPAPI-wrapped in prod); the importer
        // never returns a path and never persists plaintext to disk itself.
        let backend = InMemoryBackend::new();
        let vault = WindowsVault::with_backend_and_authz(backend, std::collections::HashMap::new());
        let count = vault.import_dotenv(dotenv);
        assert_eq!(count, 3);
        assert!(vault.exists(&SecretRef { name: "GITHUB_PAT".into(), scope: SecretScope::Workspace }));
    }
```

**Step 2 — run, expect FAIL**
`cargo test -p bongterm-vault-windows vault::tests::imports_dotenv_into_vault_without_disk_plaintext`
Expect: `cannot find type 'EnvImport'` / `no method named 'import_dotenv'`.

**Step 3 — minimal impl** — in `vault.rs`:
```rust
/// Parser for `.env` files. Produces in-memory name→value pairs only; it never
/// re-serializes plaintext to disk (callers write into the DPAPI-backed vault).
pub struct EnvImport;

impl EnvImport {
    /// Parse `.env` content into name→value pairs. Skips comments/blank lines;
    /// strips surrounding single or double quotes; preserves empty values.
    #[must_use]
    pub fn parse(content: &str) -> HashMap<String, String> {
        let mut out = HashMap::new();
        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let Some((k, v)) = line.split_once('=') else { continue };
            let key = k.trim().to_string();
            if key.is_empty() {
                continue;
            }
            let mut val = v.trim();
            if (val.starts_with('"') && val.ends_with('"') && val.len() >= 2)
                || (val.starts_with('\'') && val.ends_with('\'') && val.len() >= 2)
            {
                val = &val[1..val.len() - 1];
            }
            out.insert(key, val.to_string());
        }
        out
    }
}

impl<B: VaultBackend> WindowsVault<B> {
    /// Import `.env` content into the vault backend. Returns the number of
    /// secrets written. Plaintext is wrapped by the backend (DPAPI in prod);
    /// no plaintext file is created by this method.
    pub fn import_dotenv(&self, content: &str) -> usize {
        let parsed = EnvImport::parse(content);
        for (name, value) in &parsed {
            self.backend.put(name, value.as_bytes());
        }
        parsed.len()
    }
}
```

**Step 4 — run, expect PASS**
`cargo test -p bongterm-vault-windows vault::tests`

**Step 5 — commit**
```
git add crates/bongterm-vault-windows/src/vault.rs
git commit -m "$(cat <<'EOF'
feat(vault/4.C.2): .env import into vault backend; no plaintext persisted to disk

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>
EOF
)"
```

- [ ] 4.C.2 complete

### 4.C.3 Vault-backed env block at spawn (late, scoped resolution)

**Files**
- `crates/bongterm-vault-windows/src/vault.rs` (modify: `build_env_block`)

**Step 1 — failing test** (append to `vault.rs` `tests`):
```rust
    #[test]
    fn build_env_block_resolves_references_late_and_fails_closed_on_missing() {
        let vault = vault_with("GITHUB_PAT", "ghp_secretvalue", "agent:claude-code");
        let consumer = ConsumerId("agent:claude-code".into());
        // Env spec: literal passthrough + ${secret:NAME} reference resolution.
        let spec = vec![
            ("LOG".to_string(), "info".to_string()),
            ("TOKEN".to_string(), "${secret:GITHUB_PAT}".to_string()),
        ];
        let block = vault.build_env_block(&spec, &consumer).unwrap();
        // Literal preserved; reference resolved to plaintext only in this block.
        assert_eq!(block.iter().find(|(k, _)| k == "LOG").map(|(_, v)| v.as_str()), Some("info"));
        assert_eq!(block.iter().find(|(k, _)| k == "TOKEN").map(|(_, v)| v.as_str()), Some("ghp_secretvalue"));

        // A missing reference fails closed — never an empty TOKEN.
        let bad = vec![("TOKEN".to_string(), "${secret:NOPE}".to_string())];
        let err = vault.build_env_block(&bad, &consumer).unwrap_err();
        assert!(matches!(err, ResolveError::Missing(_)), "got {err:?}");
    }
```

**Step 2 — run, expect FAIL**
`cargo test -p bongterm-vault-windows vault::tests::build_env_block_resolves_references_late_and_fails_closed_on_missing`
Expect: `no method named 'build_env_block'`.

**Step 3 — minimal impl** — in `vault.rs`, add to `use` (`SecretScope`) and:
```rust
use bongterm_secrets_api::SecretScope;

impl<B: VaultBackend> WindowsVault<B> {
    /// Build an env block for a child process at spawn time. `${secret:NAME}`
    /// references are resolved late, in memory, for the given consumer only;
    /// literals pass through. A missing reference fails closed (spec §37).
    pub fn build_env_block(
        &self,
        spec: &[(String, String)],
        consumer: &ConsumerId,
    ) -> Result<Vec<(String, String)>, ResolveError> {
        let mut out = Vec::with_capacity(spec.len());
        for (name, raw) in spec {
            if let Some(secret_name) = raw.strip_prefix("${secret:").and_then(|s| s.strip_suffix('}')) {
                let r = SecretRef { name: secret_name.to_string(), scope: SecretScope::Agent };
                let value = self.resolve(&r, consumer)?; // fails closed on Missing/Unauthorized
                out.push((name.clone(), value.expose().to_string()));
            } else {
                out.push((name.clone(), raw.clone()));
            }
        }
        Ok(out)
    }
}
```

**Step 4 — run, expect PASS**
`cargo test -p bongterm-vault-windows vault::tests`

**Step 5 — commit**
```
git add crates/bongterm-vault-windows/src/vault.rs
git commit -m "$(cat <<'EOF'
feat(vault/4.C.3): late-scoped env block at spawn; fail closed on missing reference

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>
EOF
)"
```

- [ ] 4.C.3 complete

### 4.C.4 Launch-time disclosure model

**Files**
- `crates/bongterm-vault-windows/src/vault.rs` (modify: `LaunchDisclosure`, `disclose`)

> The disclosure *model* (data) lives here so the UI can render it without touching the vault; per the ownership matrix the UI renders, the vault discloses references. The disclosure lists **secret references and exposure class — never values**.

**Step 1 — failing test** (append to `vault.rs` `tests`):
```rust
    #[test]
    fn disclosure_lists_references_never_values() {
        let vault = vault_with("GITHUB_PAT", "ghp_secretvalue", "agent:claude-code");
        let consumer = ConsumerId("agent:claude-code".into());
        let spec = vec![
            ("LOG".to_string(), "info".to_string()),
            ("TOKEN".to_string(), "${secret:GITHUB_PAT}".to_string()),
        ];
        let disclosure = vault.disclose(&spec, &consumer);
        // Exactly one secret reference disclosed; the literal is not a secret.
        assert_eq!(disclosure.secret_refs, vec!["GITHUB_PAT".to_string()]);
        assert_eq!(disclosure.consumer, consumer);
        // The disclosure struct must NOT carry the plaintext anywhere.
        let dbg = format!("{disclosure:?}");
        assert!(!dbg.contains("ghp_secretvalue"), "disclosure leaked a value: {dbg}");
    }
```

**Step 2 — run, expect FAIL**
`cargo test -p bongterm-vault-windows vault::tests::disclosure_lists_references_never_values`
Expect: `cannot find type 'LaunchDisclosure'` / `no method named 'disclose'`.

**Step 3 — minimal impl** — in `vault.rs`:
```rust
/// What an agent/tool will receive at launch — references only, never values.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LaunchDisclosure {
    pub consumer: ConsumerId,
    /// Names of secret references this launch will resolve (no values).
    pub secret_refs: Vec<String>,
}

impl<B: VaultBackend> WindowsVault<B> {
    /// Produce the pre-launch disclosure: which secret references this env spec
    /// will resolve for the consumer. Carries no plaintext.
    #[must_use]
    pub fn disclose(&self, spec: &[(String, String)], consumer: &ConsumerId) -> LaunchDisclosure {
        let secret_refs = spec
            .iter()
            .filter_map(|(_, raw)| {
                raw.strip_prefix("${secret:")
                    .and_then(|s| s.strip_suffix('}'))
                    .map(str::to_string)
            })
            .collect();
        LaunchDisclosure { consumer: consumer.clone(), secret_refs }
    }
}
```

**Step 4 — run, expect PASS**
`cargo test -p bongterm-vault-windows vault::tests`

**Step 5 — commit**
```
git add crates/bongterm-vault-windows/src/vault.rs
git commit -m "$(cat <<'EOF'
feat(vault/4.C.4): launch-time disclosure model — references only, never values

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>
EOF
)"
```

- [ ] 4.C.4 complete

---

## 4.D — Redaction + Secret-Leak Gate

> **Binding:** redaction applies to persisted transcripts, indexes, exports, AI-context bundles, and diagnostics — **not** to the raw terminal display (silent mutation of visible output destroys trust). Detection is documented best-effort; idempotent (applying twice equals once).

### 4.D.1 `Redactor` corpus

**Files**
- `crates/bongterm-security/src/redactor.rs` (new)
- `crates/bongterm-security/src/lib.rs` (modify: `pub mod redactor;`)

**Step 1 — failing test** (append to `crates/bongterm-security/src/redactor.rs`):
```rust
#[cfg(test)]
mod tests {
    use super::*;

    const PLACEHOLDER: &str = "[REDACTED]";

    #[test]
    fn redacts_known_token_formats() {
        let r = Redactor::new();
        let cases = [
            // AWS access key id
            "AKIAIOSFODNN7EXAMPLE",
            // GitHub PAT (classic)
            "ghp_1234567890abcdefghijklmnopqrstuvwx",
            // OpenAI key
            "sk-abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRST",
            // Anthropic key
            "sk-ant-api03-abcDEF123456_ghIJKL7890-mnopqrstuvwxYZ",
            // JWT
            "eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxMjM0In0.dozjgNryP4J3jVmNHl0w5N",
        ];
        for c in cases {
            let input = format!("prefix {c} suffix");
            let out = r.redact(&input);
            assert!(!out.contains(c), "token survived redaction: {c} -> {out}");
            assert!(out.contains(PLACEHOLDER), "no placeholder for {c}: {out}");
        }
    }

    #[test]
    fn redacts_ssh_private_key_header() {
        let r = Redactor::new();
        let input = "-----BEGIN OPENSSH PRIVATE KEY-----\nb3BlbnNzaC1rZXk\n-----END OPENSSH PRIVATE KEY-----";
        let out = r.redact(input);
        assert!(!out.contains("b3BlbnNzaC1rZXk"), "key body survived: {out}");
    }

    #[test]
    fn redacts_high_entropy_strings() {
        let r = Redactor::new();
        let secret = "Zx9Kq2Lm8Vn4Pw7Rt6Yb3Hd1Gf5Js0Ca"; // 33 mixed alnum chars
        let out = r.redact(&format!("token={secret}"));
        assert!(!out.contains(secret), "high-entropy string survived: {out}");
    }

    #[test]
    fn redaction_is_idempotent() {
        let r = Redactor::new();
        let input = "ghp_1234567890abcdefghijklmnopqrstuvwx and AKIAIOSFODNN7EXAMPLE";
        let once = r.redact(input);
        let twice = r.redact(&once);
        assert_eq!(once, twice, "redaction must be idempotent");
    }

    #[test]
    fn leaves_benign_text_unchanged() {
        let r = Redactor::new();
        let input = "the quick brown fox jumps over 12 lazy dogs";
        assert_eq!(r.redact(input), input);
    }
}
```

**Step 2 — run, expect FAIL**
`cargo test -p bongterm-security redactor::tests`
Expect: `cannot find type 'Redactor'`.

**Step 3 — minimal impl** (top of `crates/bongterm-security/src/redactor.rs`):
```rust
//! Best-effort secret redactor (spec §3.7). Applies to persisted/exported/
//! indexed/AI-context/diagnostic text — NEVER to the raw terminal display.
//! Idempotent. Detection is documented best-effort, not complete.

/// The token kinds the corpus recognizes. Closed set — bounded by design.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenKind {
    AwsAccessKeyId,
    GitHubPat,
    OpenAiKey,
    AnthropicKey,
    Jwt,
    SshPrivateKeyBody,
    HighEntropy,
}

const PLACEHOLDER: &str = "[REDACTED]";

/// Redacts known secret formats from text.
pub struct Redactor;

impl Default for Redactor {
    fn default() -> Self {
        Self::new()
    }
}

impl Redactor {
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Replace recognized secret tokens with `[REDACTED]`. Idempotent.
    #[must_use]
    pub fn redact(&self, input: &str) -> String {
        // SSH private key bodies: redact lines between BEGIN/END markers.
        let mut out = self.redact_ssh_blocks(input);
        // Tokenize on whitespace-ish boundaries; replace any token that matches.
        out = out
            .split_inclusive(|c: char| c.is_whitespace())
            .map(|chunk| {
                let (tok, trailing) = split_trailing_ws(chunk);
                if tok != PLACEHOLDER && Self::classify(tok).is_some() {
                    format!("{PLACEHOLDER}{trailing}")
                } else {
                    chunk.to_string()
                }
            })
            .collect();
        out
    }

    fn redact_ssh_blocks(&self, input: &str) -> String {
        let mut redacting = false;
        let mut lines: Vec<String> = Vec::new();
        for line in input.lines() {
            if line.contains("BEGIN") && line.contains("PRIVATE KEY") {
                redacting = true;
                lines.push(line.to_string());
                continue;
            }
            if line.contains("END") && line.contains("PRIVATE KEY") {
                redacting = false;
                lines.push(line.to_string());
                continue;
            }
            if redacting && !line.trim().is_empty() {
                lines.push(PLACEHOLDER.to_string());
            } else {
                lines.push(line.to_string());
            }
        }
        let joined = lines.join("\n");
        if input.ends_with('\n') { format!("{joined}\n") } else { joined }
    }

    /// Classify a single token, if it matches a known format.
    #[must_use]
    pub fn classify(token: &str) -> Option<TokenKind> {
        if token.starts_with("AKIA") && token.len() == 20 && token[4..].chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit()) {
            return Some(TokenKind::AwsAccessKeyId);
        }
        if token.starts_with("ghp_") && token.len() >= 36 {
            return Some(TokenKind::GitHubPat);
        }
        if token.starts_with("sk-ant-") {
            return Some(TokenKind::AnthropicKey);
        }
        if token.starts_with("sk-") && token.len() >= 20 {
            return Some(TokenKind::OpenAiKey);
        }
        if is_jwt(token) {
            return Some(TokenKind::Jwt);
        }
        if is_high_entropy(token) {
            return Some(TokenKind::HighEntropy);
        }
        None
    }
}

fn split_trailing_ws(chunk: &str) -> (&str, &str) {
    let end = chunk.trim_end_matches(char::is_whitespace).len();
    (&chunk[..end], &chunk[end..])
}

fn is_jwt(token: &str) -> bool {
    let parts: Vec<&str> = token.split('.').collect();
    parts.len() == 3
        && parts.iter().all(|p| p.len() >= 8 && p.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-'))
        && token.starts_with("eyJ")
}

fn is_high_entropy(token: &str) -> bool {
    if token.len() < 32 {
        return false;
    }
    let alnum = token.chars().all(|c| c.is_ascii_alphanumeric());
    let has_upper = token.chars().any(|c| c.is_ascii_uppercase());
    let has_lower = token.chars().any(|c| c.is_ascii_lowercase());
    let has_digit = token.chars().any(|c| c.is_ascii_digit());
    alnum && has_upper && has_lower && has_digit
}
```
Add to `crates/bongterm-security/src/lib.rs` (after attribute lines):
```rust
pub mod redactor;
```

**Step 4 — run, expect PASS**
`cargo test -p bongterm-security redactor::tests`

**Step 5 — commit**
```
git add crates/bongterm-security/src/redactor.rs crates/bongterm-security/src/lib.rs
git commit -m "$(cat <<'EOF'
feat(security/4.D.1): Redactor corpus (AWS/GitHub/OpenAI/Anthropic/JWT/SSH/high-entropy), idempotent

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>
EOF
)"
```

- [ ] 4.D.1 complete

### 4.D.2 `xtask secret-leak-corpus` real impl (gate #23)

**Files**
- `tests/fixtures/secrets/corpus.jsonl` (new)
- `tools/xtask/src/secret_leak_corpus.rs` (modify)
- `tools/xtask/Cargo.toml` (modify: add `bongterm-security`)

**Step 1 — failing test** — first create the corpus `tests/fixtures/secrets/corpus.jsonl`:
```jsonl
{"kind":"aws","sample":"AKIAIOSFODNN7EXAMPLE","must_be_redacted":true}
{"kind":"github_pat","sample":"ghp_1234567890abcdefghijklmnopqrstuvwx","must_be_redacted":true}
{"kind":"openai","sample":"sk-abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRST","must_be_redacted":true}
{"kind":"anthropic","sample":"sk-ant-api03-abcDEF123456_ghIJKL7890-mnopqrstuvwxYZ","must_be_redacted":true}
{"kind":"jwt","sample":"eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxMjM0In0.dozjgNryP4J3jVmNHl0w5N","must_be_redacted":true}
{"kind":"high_entropy","sample":"Zx9Kq2Lm8Vn4Pw7Rt6Yb3Hd1Gf5Js0Ca","must_be_redacted":true}
{"kind":"benign","sample":"the quick brown fox","must_be_redacted":false}
```
Then add the regression test (append to `tools/xtask/src/secret_leak_corpus.rs`):
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn corpus_run_has_zero_leaks() {
        // The committed corpus must run clean through the redactor (gate #23).
        let report = run_corpus().expect("corpus run must succeed");
        assert_eq!(report.leaks, 0, "secret-leak corpus regressions: {:?}", report.leaked_kinds);
        assert!(report.checked >= 7, "expected the full corpus to be checked");
    }
}
```

**Step 2 — run, expect FAIL**
`cargo test -p xtask secret_leak_corpus::tests::corpus_run_has_zero_leaks`
Expect: `cannot find function 'run_corpus'` / `cannot find type ... CorpusReport`.

**Step 3 — minimal impl** — replace `tools/xtask/src/secret_leak_corpus.rs` with:
```rust
//! Phase 4.D.2: run the synthetic secret corpus through the production redactor.
//! Exit non-zero on any surviving token (spec §6.1 #23).

use anyhow::{Context, Result};
use bongterm_security::redactor::Redactor;
use std::path::PathBuf;

#[derive(Debug, serde::Deserialize)]
struct CorpusCase {
    kind: String,
    sample: String,
    must_be_redacted: bool,
}

#[derive(Debug, Default)]
pub struct CorpusReport {
    pub checked: usize,
    pub leaks: usize,
    pub leaked_kinds: Vec<String>,
}

fn corpus_path() -> PathBuf {
    // CARGO_MANIFEST_DIR = tools/xtask; corpus lives at <root>/tests/fixtures/secrets.
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest.join("../../tests/fixtures/secrets/corpus.jsonl")
}

pub fn run_corpus() -> Result<CorpusReport> {
    let path = corpus_path();
    let text = std::fs::read_to_string(&path)
        .with_context(|| format!("reading corpus at {}", path.display()))?;
    let redactor = Redactor::new();
    let mut report = CorpusReport::default();
    for line in text.lines().filter(|l| !l.trim().is_empty()) {
        let case: CorpusCase = serde_json::from_str(line)
            .with_context(|| format!("parsing corpus line: {line}"))?;
        report.checked += 1;
        let redacted = redactor.redact(&format!("value: {} end", case.sample));
        let survived = redacted.contains(&case.sample);
        if case.must_be_redacted && survived {
            report.leaks += 1;
            report.leaked_kinds.push(case.kind);
        }
    }
    Ok(report)
}

pub fn run() -> Result<()> {
    let report = run_corpus()?;
    if report.leaks > 0 {
        anyhow::bail!(
            "secret-leak corpus FAILED: {} leak(s) in kinds {:?} ({} checked)",
            report.leaks,
            report.leaked_kinds,
            report.checked
        );
    }
    println!("secret-leak corpus PASSED: {} cases, 0 leaks", report.checked);
    Ok(())
}
```
Add to `tools/xtask/Cargo.toml` `[dependencies]`:
```toml
bongterm-security = { path = "../../crates/bongterm-security" }
```

**Step 4 — run, expect PASS**
`cargo test -p xtask secret_leak_corpus::tests::corpus_run_has_zero_leaks`
Then end-to-end: `cargo xtask secret-leak-corpus` (prints PASSED, exits 0).

**Step 5 — commit**
```
git add tests/fixtures/secrets/corpus.jsonl tools/xtask/src/secret_leak_corpus.rs tools/xtask/Cargo.toml
git commit -m "$(cat <<'EOF'
feat(xtask/4.D.2): real secret-leak-corpus gate over Redactor (gate #23)

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>
EOF
)"
```

- [ ] 4.D.2 complete

### 4.D.3 Telemetry redaction preview (before any opt-in export)

**Files**
- `crates/bongterm-security/src/redactor.rs` (modify: `RedactionPreview`, `preview`)

**Step 1 — failing test** (append to `redactor.rs` `tests`):
```rust
    #[test]
    fn preview_shows_redacted_text_and_match_count_before_send() {
        let r = Redactor::new();
        let bundle = "log line\ntoken ghp_1234567890abcdefghijklmnopqrstuvwx done\nAKIAIOSFODNN7EXAMPLE";
        let preview = r.preview(bundle);
        // The preview is what the user sees before consenting to export.
        assert!(!preview.redacted.contains("ghp_1234567890abcdefghijklmnopqrstuvwx"));
        assert!(!preview.redacted.contains("AKIAIOSFODNN7EXAMPLE"));
        assert_eq!(preview.match_count, 2, "two tokens should be flagged");
        // Original is retained for side-by-side diff in the UI but never exported.
        assert_eq!(preview.original, bundle);
    }
```

**Step 2 — run, expect FAIL**
`cargo test -p bongterm-security redactor::tests::preview_shows_redacted_text_and_match_count_before_send`
Expect: `cannot find type 'RedactionPreview'` / `no method named 'preview'`.

**Step 3 — minimal impl** — in `redactor.rs`:
```rust
/// A preview shown before any opt-in telemetry/diagnostic export (spec §6.1 #19).
/// Holds original (for UI diff, never exported) + redacted (what would be sent).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RedactionPreview {
    pub original: String,
    pub redacted: String,
    pub match_count: usize,
}

impl Redactor {
    /// Produce a redaction preview for user review before export.
    #[must_use]
    pub fn preview(&self, bundle: &str) -> RedactionPreview {
        let redacted = self.redact(bundle);
        let match_count = redacted.matches(PLACEHOLDER).count();
        RedactionPreview { original: bundle.to_string(), redacted, match_count }
    }
}
```

**Step 4 — run, expect PASS**
`cargo test -p bongterm-security redactor::tests`

**Step 5 — commit**
```
git add crates/bongterm-security/src/redactor.rs
git commit -m "$(cat <<'EOF'
feat(security/4.D.3): redaction preview before opt-in export (gate #19)

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>
EOF
)"
```

- [ ] 4.D.3 complete

---

## 4.E — Dangerous-Command Policy, Workspace Trust, Production Safety

### 4.E.1 Dangerous-command matcher

**Files**
- `crates/bongterm-security/src/dangerous.rs` (new)
- `crates/bongterm-security/src/lib.rs` (modify: `pub mod dangerous;`)

**Step 1 — failing test** (append to `crates/bongterm-security/src/dangerous.rs`):
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flags_known_dangerous_commands() {
        let m = DangerousCommandMatcher::new();
        assert_eq!(m.classify("git push --force origin main"), Some(DangerKind::GitForcePush));
        assert_eq!(m.classify("git push --force-with-lease"), Some(DangerKind::GitForcePush));
        assert_eq!(m.classify("rm -rf /"), Some(DangerKind::RecursiveDelete));
        assert_eq!(m.classify("sudo rm -rf /var"), Some(DangerKind::RecursiveDelete));
        assert_eq!(m.classify("kubectl delete pod foo"), Some(DangerKind::KubectlDelete));
        assert_eq!(m.classify("terraform destroy -auto-approve"), Some(DangerKind::TerraformDestroy));
    }

    #[test]
    fn benign_commands_are_not_flagged() {
        let m = DangerousCommandMatcher::new();
        assert_eq!(m.classify("git push origin main"), None);
        assert_eq!(m.classify("ls -la"), None);
        assert_eq!(m.classify("rm file.txt"), None);
        assert_eq!(m.classify("kubectl get pods"), None);
        assert_eq!(m.classify("terraform plan"), None);
    }

    #[test]
    fn dangerous_command_requires_approval_not_advisory() {
        // A detected dangerous command must map to RequireApproval enforcement,
        // never silently Advisory (no auto-run of destructive actions).
        let m = DangerousCommandMatcher::new();
        let kind = m.classify("rm -rf /").unwrap();
        assert_eq!(kind.enforcement(), EnforcementLevel::RequireApproval);
    }
}
```

**Step 2 — run, expect FAIL**
`cargo test -p bongterm-security dangerous::tests`
Expect: `cannot find type 'DangerousCommandMatcher'` / `DangerKind`.

**Step 3 — minimal impl** (top of `crates/bongterm-security/src/dangerous.rs`):
```rust
//! Dangerous-command pattern matcher (spec §3.2 no-auto-run, PRD §17.5).
//! Closed `DangerKind` enum — bounded set, exhaustive match.

use crate::EnforcementLevel;

/// Classified dangerous-command kinds. Closed set.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DangerKind {
    GitForcePush,
    RecursiveDelete,
    KubectlDelete,
    TerraformDestroy,
}

impl DangerKind {
    /// Detected dangerous commands always require explicit approval —
    /// never auto-run (CLAUDE.md hard non-goal).
    #[must_use]
    pub fn enforcement(self) -> EnforcementLevel {
        match self {
            Self::GitForcePush
            | Self::RecursiveDelete
            | Self::KubectlDelete
            | Self::TerraformDestroy => EnforcementLevel::RequireApproval,
        }
    }
}

/// Matches command lines against known destructive patterns.
pub struct DangerousCommandMatcher;

impl Default for DangerousCommandMatcher {
    fn default() -> Self {
        Self::new()
    }
}

impl DangerousCommandMatcher {
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Classify a command line, returning the danger kind if it matches.
    #[must_use]
    pub fn classify(&self, command: &str) -> Option<DangerKind> {
        let c = command.to_lowercase();
        let tokens: Vec<&str> = c.split_whitespace().collect();
        let has = |w: &str| tokens.iter().any(|t| *t == w);

        if has("git") && has("push") && tokens.iter().any(|t| t.starts_with("--force")) {
            return Some(DangerKind::GitForcePush);
        }
        if has("rm") && tokens.iter().any(|t| t == "-rf" || t == "-fr" || t == "-r" && has("-f")) {
            return Some(DangerKind::RecursiveDelete);
        }
        if has("kubectl") && has("delete") {
            return Some(DangerKind::KubectlDelete);
        }
        if has("terraform") && has("destroy") {
            return Some(DangerKind::TerraformDestroy);
        }
        None
    }
}
```
Add to `crates/bongterm-security/src/lib.rs`:
```rust
pub mod dangerous;
```

**Step 4 — run, expect PASS**
`cargo test -p bongterm-security dangerous::tests`

**Step 5 — commit**
```
git add crates/bongterm-security/src/dangerous.rs crates/bongterm-security/src/lib.rs
git commit -m "$(cat <<'EOF'
feat(security/4.E.1): dangerous-command matcher (force-push/rm -rf/kubectl delete/terraform destroy)

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>
EOF
)"
```

- [ ] 4.E.1 complete

### 4.E.2 Workspace trust store (new folder defaults Untrusted)

**Files**
- `crates/bongterm-security/src/trust.rs` (new)
- `crates/bongterm-security/src/lib.rs` (modify: `pub mod trust;`)

**Step 1 — failing test** (append to `crates/bongterm-security/src/trust.rs`):
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn newly_opened_workspace_defaults_untrusted() {
        let store = WorkspaceTrustStore::new();
        // A folder never seen before is Untrusted by default (threat model §35).
        assert_eq!(store.state("C:/repos/unknown"), TrustState::Untrusted);
        assert!(store.requires_prompt("C:/repos/unknown"));
    }

    #[test]
    fn explicitly_trusting_persists_decision() {
        let store = WorkspaceTrustStore::new();
        store.trust("C:/repos/myproj");
        assert_eq!(store.state("C:/repos/myproj"), TrustState::Trusted);
        assert!(!store.requires_prompt("C:/repos/myproj"));
    }

    #[test]
    fn revoking_returns_to_untrusted() {
        let store = WorkspaceTrustStore::new();
        store.trust("C:/repos/myproj");
        store.revoke("C:/repos/myproj");
        assert_eq!(store.state("C:/repos/myproj"), TrustState::Untrusted);
    }
}
```

**Step 2 — run, expect FAIL**
`cargo test -p bongterm-security trust::tests`
Expect: `cannot find type 'WorkspaceTrustStore'` / `TrustState`.

**Step 3 — minimal impl** (top of `crates/bongterm-security/src/trust.rs`):
```rust
//! Workspace trust (spec §35 malicious workspace config). New folders default
//! Untrusted; the user must explicitly trust before risky config is honored.

use std::collections::HashSet;
use std::sync::Mutex;

/// Trust state for a workspace path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrustState {
    Untrusted,
    Trusted,
}

/// Stores per-workspace trust decisions. Default deny: unknown = Untrusted.
#[derive(Default)]
pub struct WorkspaceTrustStore {
    trusted: Mutex<HashSet<String>>,
}

impl WorkspaceTrustStore {
    #[must_use]
    pub fn new() -> Self {
        Self { trusted: Mutex::new(HashSet::new()) }
    }

    /// Current trust state for a workspace path.
    ///
    /// # Panics
    /// Panics if the internal mutex is poisoned.
    #[must_use]
    pub fn state(&self, path: &str) -> TrustState {
        if self.trusted.lock().unwrap().contains(path) {
            TrustState::Trusted
        } else {
            TrustState::Untrusted
        }
    }

    /// Whether opening this workspace must prompt the user.
    #[must_use]
    pub fn requires_prompt(&self, path: &str) -> bool {
        self.state(path) == TrustState::Untrusted
    }

    /// Record an explicit trust decision.
    ///
    /// # Panics
    /// Panics if the internal mutex is poisoned.
    pub fn trust(&self, path: &str) {
        self.trusted.lock().unwrap().insert(path.to_string());
    }

    /// Revoke trust, returning the workspace to Untrusted.
    ///
    /// # Panics
    /// Panics if the internal mutex is poisoned.
    pub fn revoke(&self, path: &str) {
        self.trusted.lock().unwrap().remove(path);
    }
}
```
Add to `crates/bongterm-security/src/lib.rs`:
```rust
pub mod trust;
```

**Step 4 — run, expect PASS**
`cargo test -p bongterm-security trust::tests`

**Step 5 — commit**
```
git add crates/bongterm-security/src/trust.rs crates/bongterm-security/src/lib.rs
git commit -m "$(cat <<'EOF'
feat(security/4.E.2): workspace trust store — new folders default Untrusted

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>
EOF
)"
```

- [ ] 4.E.2 complete

### 4.E.3 Production safety mode

**Files**
- `crates/bongterm-security/src/prod_mode.rs` (new)
- `crates/bongterm-security/src/lib.rs` (modify: `pub mod prod_mode;`)

**Step 1 — failing test** (append to `crates/bongterm-security/src/prod_mode.rs`):
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::dangerous::DangerKind;

    #[test]
    fn production_mode_escalates_dangerous_to_deny() {
        // When production safety mode is ON, dangerous commands escalate from
        // RequireApproval to Deny (hardest enforcement).
        let mode = ProductionSafetyMode::on();
        assert_eq!(mode.escalate(DangerKind::GitForcePush), EnforcementLevel::Deny);
    }

    #[test]
    fn off_mode_preserves_require_approval() {
        let mode = ProductionSafetyMode::off();
        assert_eq!(mode.escalate(DangerKind::RecursiveDelete), EnforcementLevel::RequireApproval);
        assert!(!mode.is_on());
    }
}
```

**Step 2 — run, expect FAIL**
`cargo test -p bongterm-security prod_mode::tests`
Expect: `cannot find type 'ProductionSafetyMode'`.

**Step 3 — minimal impl** (top of `crates/bongterm-security/src/prod_mode.rs`):
```rust
//! Production safety mode (PRD §17.5 / spec §9.10 danger tokens). When ON,
//! dangerous-command enforcement is raised to the hardest level.

use crate::dangerous::DangerKind;
use crate::EnforcementLevel;

/// A per-workspace toggle that hardens dangerous-command enforcement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProductionSafetyMode {
    on: bool,
}

impl ProductionSafetyMode {
    #[must_use]
    pub fn on() -> Self {
        Self { on: true }
    }

    #[must_use]
    pub fn off() -> Self {
        Self { on: false }
    }

    #[must_use]
    pub fn is_on(self) -> bool {
        self.on
    }

    /// Resolve the enforcement level for a dangerous command under this mode.
    /// ON escalates to `Deny`; OFF keeps the kind's baseline enforcement.
    #[must_use]
    pub fn escalate(self, kind: DangerKind) -> EnforcementLevel {
        if self.on {
            EnforcementLevel::Deny
        } else {
            kind.enforcement()
        }
    }
}
```
Add to `crates/bongterm-security/src/lib.rs`:
```rust
pub mod prod_mode;
```

**Step 4 — run, expect PASS**
`cargo test -p bongterm-security prod_mode::tests`

**Step 5 — commit**
```
git add crates/bongterm-security/src/prod_mode.rs crates/bongterm-security/src/lib.rs
git commit -m "$(cat <<'EOF'
feat(security/4.E.3): production safety mode escalates dangerous commands to Deny

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>
EOF
)"
```

- [ ] 4.E.3 complete

---

## 4.F — Conformance + Negative Coverage (test-kit)

### 4.F.1 `Supervisor` conformance suite

**Files**
- `crates/bongterm-test-kit/src/conformance/mcp_supervisor_conformance.rs` (new)
- `crates/bongterm-test-kit/src/conformance/mod.rs` (modify)
- `crates/bongterm-test-kit/Cargo.toml` (modify: ensure `bongterm-mcp` dep present — it already is)

**Step 1 — failing test** (create `crates/bongterm-test-kit/src/conformance/mcp_supervisor_conformance.rs`):
```rust
//! Conformance suite for `bongterm_mcp::supervisor::Supervisor`.

use bongterm_mcp::supervisor::{IdleShutdownOutcome, Supervisor, WorkspaceId};
use bongterm_mcp::{McpServerConfig, MockMcpTransport};

/// Exercise one-process-per-server + idle-shutdown invariants.
///
/// # Panics
/// Panics if any conformance assertion fails.
pub fn run() {
    let sup = Supervisor::new();
    let ws = WorkspaceId("conformance-ws".to_string());
    let cfg = McpServerConfig { name: "srv".to_string(), argv: vec!["node".to_string()], env: vec![] };

    sup.register(ws.clone(), cfg.clone(), Box::new(MockMcpTransport::new()))
        .expect("first register must succeed");
    assert!(
        sup.register(ws.clone(), cfg, Box::new(MockMcpTransport::new())).is_err(),
        "duplicate (workspace, server) must be rejected — one process per server"
    );
    assert_eq!(sup.server_count(&ws), 1);
    assert_eq!(
        sup.try_idle_shutdown(&ws, "srv"),
        IdleShutdownOutcome::Stopped,
        "idle shutdown with no attached agent must stop the server"
    );
    assert_eq!(sup.server_count(&ws), 0);
}

#[cfg(test)]
mod tests {
    #[test]
    fn supervisor_conformance_passes_for_real_supervisor() {
        super::run();
    }
}
```

**Step 2 — run, expect FAIL**
`cargo test -p bongterm-test-kit mcp_supervisor_conformance`
Expect: module not declared → `cannot find` / unresolved; build error until `mod.rs` updated.

**Step 3 — minimal impl** — add to `crates/bongterm-test-kit/src/conformance/mod.rs`:
```rust
pub mod mcp_supervisor_conformance;
```
Ensure `crates/bongterm-test-kit/Cargo.toml` lists `bongterm-mcp = { path = "../bongterm-mcp" }` (Phase 0 already wired it; if absent, add it).

**Step 4 — run, expect PASS**
`cargo test -p bongterm-test-kit mcp_supervisor_conformance`

**Step 5 — commit**
```
git add crates/bongterm-test-kit/src/conformance/mcp_supervisor_conformance.rs crates/bongterm-test-kit/src/conformance/mod.rs crates/bongterm-test-kit/Cargo.toml
git commit -m "$(cat <<'EOF'
test(test-kit/4.F.1): Supervisor conformance suite — one-proc-per-server + idle shutdown

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>
EOF
)"
```

- [ ] 4.F.1 complete

### 4.F.2 Negative coverage: env-block no-leak + dangerous-command non-bypass

**Files**
- `crates/bongterm-test-kit/src/conformance/negative.rs` (modify)
- `crates/bongterm-test-kit/Cargo.toml` (modify: add `bongterm-vault-windows`, `bongterm-mcp`)
- `tools/xtask/allowed-deps.toml` (modify: add `bongterm-vault-windows` to `bongterm-test-kit` allowed list)

**Step 1 — failing test** (append inside the `mod tests` in `crates/bongterm-test-kit/src/conformance/negative.rs`):
```rust
    // -------------------------------------------------------------------------
    // Test 6 — env block never carries a missing secret (fails closed)
    // -------------------------------------------------------------------------
    #[test]
    fn env_block_fails_closed_on_missing_secret() {
        use bongterm_secrets_api::ConsumerId;
        use bongterm_vault_windows::{InMemoryBackend, WindowsVault};

        let vault = WindowsVault::with_backend_and_authz(
            InMemoryBackend::new(),
            std::collections::HashMap::new(),
        );
        let spec = vec![("TOKEN".to_string(), "${secret:NOPE}".to_string())];
        let result = vault.build_env_block(&spec, &ConsumerId("agent:x".to_string()));
        assert!(result.is_err(), "missing secret must fail closed, never empty env var");
    }

    // -------------------------------------------------------------------------
    // Test 7 — dangerous command is never silently Advisory
    // -------------------------------------------------------------------------
    #[test]
    fn dangerous_command_never_advisory() {
        use bongterm_security::dangerous::DangerousCommandMatcher;
        use bongterm_security::EnforcementLevel;

        let m = DangerousCommandMatcher::new();
        let kind = m.classify("git push --force").expect("must be flagged");
        assert_ne!(
            kind.enforcement(),
            EnforcementLevel::Advisory,
            "dangerous commands must require approval or deny, never Advisory"
        );
    }
```

**Step 2 — run, expect FAIL**
`cargo test -p bongterm-test-kit negative`
Expect: `unresolved import 'bongterm_vault_windows'` (dep + allowed-deps edge not yet added).

**Step 3 — minimal impl** — add to `crates/bongterm-test-kit/Cargo.toml` `[dependencies]`:
```toml
bongterm-vault-windows = { path = "../bongterm-vault-windows" }
```
Add `bongterm-vault-windows` to the `bongterm-test-kit` `allowed` array in `tools/xtask/allowed-deps.toml`. (`bongterm-mcp`, `bongterm-security`, `bongterm-secrets-api` are already listed.)

**Step 4 — run, expect PASS**
`cargo test -p bongterm-test-kit negative`
Then: `cargo xtask check-deps` (must still pass with the new edge).

**Step 5 — commit**
```
git add crates/bongterm-test-kit/src/conformance/negative.rs crates/bongterm-test-kit/Cargo.toml tools/xtask/allowed-deps.toml
git commit -m "$(cat <<'EOF'
test(test-kit/4.F.2): negative coverage — env-block fail-closed + dangerous-cmd non-bypass

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>
EOF
)"
```

- [ ] 4.F.2 complete

---

## 4.exit — Exit Gate + Threat-Model Review

### 4.exit.1 ADR + threat-model coverage doc

**Files**
- `docs/adr/0010-mcp-process-governance.md` (new)
- `docs/security/threat-model-phase4.md` (new)

Write `docs/adr/0010-mcp-process-governance.md` recording: one process per server per workspace; JobObject RSS (60 MB default) + child-count caps; restart backoff 1s/5s/30s then Unhealthy; idle shutdown gated on no active agent; Context Optimizer is token-budget only (not RSS). Status: Accepted.

Write `docs/security/threat-model-phase4.md` mapping each §35.4 priority to its Phase 4 control:
- **Indirect prompt injection (highest):** all MCP/tool results untrusted; dangerous-command matcher → RequireApproval/Deny; no auto-run; per-agent tool allowlist (default deny).
- **Supply-chain compromise:** no `npx -y` (rejected at import 4.A.4 + transport 4.A); version-pinned argv; config schema validation.
- **Secret exfiltration:** DPAPI/CredMan vault; late-scoped resolution; env block never on argv/disk; Redactor on all persisted/exported text; secret-leak corpus gate = 0.
- **Malicious VT/OSC escapes:** out of Phase 4 scope (parser owns; carried by Phase 0 fuzz + Phase 5).
- **Malicious workspace config:** workspace trust defaults Untrusted; risky config honored only after explicit trust.
- **DoS / resource exhaustion:** JobObject RSS + child-count caps; restart-backoff ceiling → Unhealthy; idle shutdown.

**Commit**
```
git add docs/adr/0010-mcp-process-governance.md docs/security/threat-model-phase4.md
git commit -m "$(cat <<'EOF'
docs(4.exit): ADR-0010 MCP governance + Phase 4 threat-model coverage table

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>
EOF
)"
```

- [ ] 4.exit.1 complete

### 4.exit.2 Full-workspace gate run

Run, all must be green:
```
cargo fmt --all -- --check
cargo clippy --all-targets --all-features --workspace -- -D warnings
cargo test --workspace
cargo xtask check-deps
cargo xtask secret-leak-corpus
```
Map to §6.1 gates:
- **#16** MCP manual JSON import + permissions visible + JobObject caps enforced + logs visible + no `npx -y` → 4.A.1–4.A.6, 4.B.* green.
- **#19** Telemetry off by default; diagnostic export shows redaction preview before send → 4.D.3 green.
- **#23** Secret-leak corpus = 0 leaks → `cargo xtask secret-leak-corpus` exit 0 (4.D.2).
- **#31** Never auto-installs; never `npx -y` → 4.A.4 forbidden-install + existing transport rejection.

Exit only when all four gates stay green for 7 consecutive nightly runs and the threat-model review (4.exit.1) is accepted. Then advance `orca.md` (`4.replan` → invoke `superpowers:writing-plans` for Phase 5).

- [ ] 4.exit.2 complete

---

## Self-Review

### Outline-task coverage (every `orca.md` 4.* maps to a task)
| orca outline | Plan task |
|---|---|
| 4.A.1 `Supervisor` real impl (1 proc/server/ws) | 4.A.1 |
| 4.A.2 JobObject caps via `bongterm-process-control` | 4.A.2 (+ `WindowsJobGovernor`) |
| 4.A.3 MCP JSON import + schema validation | 4.A.3 |
| 4.A.4 No `npx -y` + forbidden-install test | 4.A.4 (+ existing transport test) |
| 4.A.5 Idle shutdown only when no active agent | 4.A.5 |
| 4.A.6 Health check + RSS sample + restart-backoff | 4.A.6 |
| 4.B.1 Context Optimizer allowlist + token preview | 4.B.1 + 4.B.2 |
| 4.B.2 Temporary scoped MCP config | 4.B.3 |
| 4.B.3 Unavailable label for non-supporting agents | 4.B.3 |
| 4.C.1 `bongterm-vault-windows` DPAPI/CredMan `SecretStore` | 4.C.1 |
| 4.C.2 `.env` import flow | 4.C.2 |
| 4.C.3 Vault-backed env at spawn (no disk plaintext) | 4.C.3 |
| 4.C.4 Launch-time disclosure modal | 4.C.4 (model; UI render is Phase 2/5 surface) |
| 4.D.1 `Redactor` corpus | 4.D.1 |
| 4.D.2 `xtask secret-leak-corpus` real impl | 4.D.2 |
| 4.D.3 Telemetry redaction preview | 4.D.3 |
| 4.E.1 Dangerous-command matcher | 4.E.1 |
| 4.E.2 Workspace trust prompt | 4.E.2 |
| 4.E.3 Production safety mode UI | 4.E.3 (logic; UI render is a `bongterm-ui` surface) |
| 4.exit | 4.exit.1 + 4.exit.2 |

Added (not in outline but required): 4.F.1 Supervisor conformance, 4.F.2 negative coverage — required by §5.1 (every concrete impl runs the conformance suite) + §5.6 security negative tests.

### §6.1 gate coverage
- **#16** → 4.A.1–4.A.6, 4.B.1–4.B.3 (import, caps, permissions/allowlist visible, logs via metrics, no `npx -y`).
- **#19** → 4.D.3 (redaction preview before send; telemetry-off-by-default is a `bongterm-diagnostics` consent flag wired in Phase 5 §5.D.2, referenced here).
- **#23** → 4.D.1 + 4.D.2 (corpus = 0 leaks gate).
- **#31** → 4.A.4 (forbidden-install at import) + existing `MockMcpTransport`/transport `npx -y` rejection.

### Placeholder scan
No `TODO`/`unimplemented!()`/`todo!()` in any code step. Every step ships compiling Rust with a concrete body. The only deliberately-deferred items are the UI *renderings* of the disclosure modal (4.C.4) and production-safety toggle (4.E.3) and the telemetry-consent flag (#19), which the ownership matrix assigns to `bongterm-ui`/`bongterm-diagnostics` surfaces landing in their own phases — the Phase 4 logic/data they consume is fully implemented here.

### Type consistency
- `JobObjectCaps`, `ProcessGovernor`, `ProcessHandle`, `GovernorError`, `TerminationReason`, `AdmissionVerdict` reused verbatim from `bongterm-process-control` (no redefinition).
- `McpServerConfig`, `McpToolDescriptor`, `McpError`, `StopReason`, `McpTransport`, `MockMcpTransport` reused from `bongterm-mcp` lib.rs.
- `SecretRef`, `SecretScope`, `ConsumerId`, `SecretValue`, `SecretStore`, `ResolveError`, `ExposureClass` reused from `bongterm-secrets-api`.
- `Decision`, `EnforcementLevel`, `RiskClass`, `PolicyEvaluator`, `PolicyRequest` reused from `bongterm-security`; new `DangerKind`/`Redactor`/`WorkspaceTrustStore`/`ProductionSafetyMode` are additive modules.
- Backoff schedule `1s/5s/30s` and 60 MB RSS default match spec §3.4 / §4.2 exactly.
- `Supervisor` signatures (`register`, `register_with_caps`, `caps_for`, `attach_agent`, `detach_agent`, `try_idle_shutdown`) are consistent across 4.A.1–4.A.5 and the 4.F.1 conformance suite.

### Threat-model coverage (§35 scenarios considered)
Indirect prompt injection (untrusted MCP results + default-deny allowlist + dangerous-command approval gate), supply-chain (no `npx -y`, schema validation, version-pinned argv), secret exfiltration (DPAPI vault + late-scoped env + redactor + corpus gate + no-plaintext-on-disk `.env` import), malicious workspace config (trust defaults Untrusted), DoS/resource exhaustion (JobObject caps + backoff ceiling + idle shutdown). Malicious VT/OSC escapes are explicitly out of Phase 4 scope (parser-owned; Phase 0 fuzz + Phase 5). Documented in 4.exit.1.

### Architecture / ownership compliance
- `mcp` gains registry/lifecycle/governance/optimizer only — no agent UI, renderer, or Git (matrix-clean).
- `security` gains redactor/dangerous/trust/prod-mode — no renderer/parser/vault impl.
- `vault-windows` implements `SecretStore` + env-block + disclosure data — no policy decisions, no UI.
- `process-control` gains real JobObject enforcement — measures via `sample_rss`, enforces caps; does not decide policy.
- New dependency edges (`bongterm-mcp → bongterm-process-control`, `bongterm-test-kit → bongterm-vault-windows`) are added to `allowed-deps.toml` and verified by `cargo xtask check-deps`.
- All Windows `unsafe` is scoped to documented Win32 user-mode APIs (DPAPI, Credential Manager, JobObject) — no DLL injection, hooks, undocumented syscalls, or OS-bypass (CLAUDE.md hard non-goals upheld).

*End of Phase 4 plan.*
