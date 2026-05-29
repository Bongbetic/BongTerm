# BongTerm Phase 2 Status

Source of truth:

- Plan: `docs/superpowers/plans/2026-05-29-bongt-phase2.md`
- Execution rules: `AGENTS.md`

Current focus: `2.B.4`

| Task ID | Status | Last test run | Notes/blockers | Next task |
| --- | --- | --- | --- | --- |
| 2.A.0 | Complete | `cargo test -p bongterm-agents summarize_exit` (pass, 1 test) | `cargo fmt --all -- --check` fails from unrelated pre-existing workspace formatting diffs | 2.A.2 |
| 2.A.2 | Complete | `cargo test -p bongterm-agents discover::` (pass, 3 tests) | RED observed first (`BinaryDiscovery`/`parse_version_line` missing), then GREEN after implementing `discover.rs` | 2.A.3 |
| 2.A.3 | Complete | `cargo test -p bongterm-agents classify::` (pass, 3 tests) | RED observed first (`LineBuffer`/`is_suspected_injection`/`classify_claude_line` missing), then GREEN after implementing shared `classify.rs` line-buffer + injection heuristics + Claude JSON classification | 2.A.4 |
| 2.A.4 | Complete | `cargo test -p bongterm-agents claude_code::` (pass, 4 tests) | RED observed first (`ClaudeCodeAdapter` missing), then GREEN after implementing `claude_code.rs` with shared `classify.rs` core, injectable discovery path, and stateful stream-json classifier | 2.A.5 |
| 2.A.5 | Complete | `cargo test -p bongterm-agents codex_cli::` (pass, 3 tests) | RED step initially showed 0 tests because `codex_cli.rs` was still placeholder; then GREEN after implementing `codex_cli.rs` (`CodexCliAdapter` + `CodexCliClassifier`) with shared `classify.rs` core | 2.A.6 |
| 2.A.6 | Complete | `cargo test -p bongterm-agents --test conformance` (pass, 2 tests) | RED observed first (`--test conformance` missing target), then GREEN after adding `conformance.rs` + extending `agent_adapter_conformance::run_offline`; required `cargo xtask check-deps` failed on unrelated pre-existing violation `bongterm-storage-sqlite -> bongterm-test-kit` | 2.B.1 |
| 2.B.1 | Complete | `cargo test -p bongterm-agents transcript::` (pass, 2 tests) | RED observed first (`TranscriptSink` missing; tests failed to compile), then GREEN after implementing `transcript.rs` `TranscriptSink` over `TranscriptRepo` with monotonic chunk indexing and paused-on-error backpressure behavior | 2.B.2 |
| 2.B.2 | Complete | `cargo test -p bongterm-agents file_change::` (pass, 4 tests) | RED observed first (`parse_porcelain_v1`/`ChangeStatus`/`GitPorcelainTracker` missing), then GREEN after implementing `file_change.rs` porcelain-v1 parser + snapshot diff attribution + injectable git runner | 2.B.3 |
| 2.B.3 | Complete | `cargo test -p bongterm-agents approval::` (pass, 4 tests) | RED observed first (`ApprovalQueue`/`ApprovalDecision`/`ApprovalState` missing), then GREEN after implementing `approval.rs` policy-routed `ApprovalQueue` with explicit `EnforcementLevel` labels and deny-never-approvable resolution rule | 2.B.4 |
| 2.B.4 | Complete | `cargo test -p bongterm-agents replay_` (pass, 4 tests) | RED observed first (`ReplayBuilder` missing), then GREEN after implementing `replay.rs` `ReplayBuilder` + `ReplaySpec` prefilled summary context replay | 2.C.2a |
| 2.C.2a | Not started | Not run | - | 2.C.1 |
| 2.C.1 | Not started | Not run | - | 2.C.3a |
| 2.C.3a | Not started | Not run | - | 2.C.3b |
| 2.C.3b | Not started | Not run | - | 2.C.3c |
| 2.C.3c | Not started | Not run | - | 2.D.1 |
| 2.D.1 | Not started | Not run | - | 2.EXIT |
| 2.EXIT | Not started | Not run | - | - |
