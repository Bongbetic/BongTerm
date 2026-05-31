# BongTerm Phase 2 Status

Source of truth:

- Plan: `docs/superpowers/plans/2026-05-29-bongt-phase2.md`
- Execution rules: `AGENTS.md`

Current focus: **Phase 2 complete** — all code tasks GREEN; gates #15 + #24 wired into nightly. Next: `2.replan` (invoke `superpowers:writing-plans` for Phase 3).

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
| 2.C.2a | Complete | `cargo test -p bongterm-agents lifecycle::` (pass, 7 tests) | Previously blocked by missing `ReplayBuilder` in `replay.rs`; rerun after replay implementation sync is GREEN | 2.C.1 |
| 2.C.1 | Complete | `cargo test -p bongterm-ui agent_sidebar::` (pass, 5 tests) | RED observed first (missing `ShellMessage::{AgentLifecycle,AgentInterrupt,ApprovalResolve}`), then GREEN after adding UI message variants + no-op update arms; required `cargo xtask check-deps` still fails on pre-existing unrelated violation `bongterm-storage-sqlite -> bongterm-test-kit` | 2.C.3a |
| 2.C.3a | Complete | `cargo test -p bongterm-agents scenario_deserializes_from_json` (pass) | RED first (`InjectionScenario` missing) → GREEN. `corpus.rs` model + `load_dir`. Commit `31a9c0e` | 2.C.3b |
| 2.C.3b | Complete | `cargo test -p bongterm-agents corpus::` (3 pass) | 32 fixtures (30 poisoned + 2 benign) under `tests/fixtures/prompt_injection/`; detection-alignment test added. Commit `8c9924a` | 2.C.3c |
| 2.C.3c | Complete | `cargo test -p xtask prompt_injection_corpus::tests` (6 pass) + `cargo run -p xtask -- prompt-injection-corpus` (`32 scenarios passed gate #24`, exit 0) | Reconciled 2 plan inconsistencies: schema (serde alias `poisoned_content`→`payload`, default `expected_enforcement`) + markers set byte-identical to `classify::INJECTION_MARKERS` (plan's pasted list had drifted, 9 fixtures would miss). Commit `2dd8048` | 2.D.1 |
| 2.D.1 | Complete | `cargo test -p bongterm-agents --test gate15` (3 pass, 1 ignored) | gate #15 evidence. Adapted to real APIs (`claude-code`/`codex-cli` names, `ProcessExited`+`state()`, Output-event capture). Commit `79fecc2` | 2.EXIT |
| 2.EXIT | Complete (code) | gates #15 + #24 GREEN locally; `cargo test --workspace` green; `check-deps: ok` | `nightly.yml` created with gates job. **Operational/future:** green ×7 nightlies. **Out-of-scope debt:** workspace `clippy -D warnings` + `fmt --check` fail on pre-existing issues in other crates (1.exit/hygiene). Commit `662e31b` | Phase 3 (after `2.replan`) |
