# BongTerm Phase 3 Status

Source of truth:

- Plan: `docs/superpowers/plans/2026-05-29-bongt-phase3.md`
- Execution rules: `AGENTS.md`

Current focus: **Phase 3 task 3.A.3 complete** — failed-command `Explainer` for non-zero exit blocks.

Phase 2 handoff: all code tasks are complete; gates #15 + #24 are GREEN locally and wired into nightly. Operational/future requirement remains green x7 nightlies.

| Task ID | Status | Last test run | Notes/blockers | Next task |
| --- | --- | --- | --- | --- |
| 3.A.0 | Complete | `cargo test -p bongterm-devassist wiring_tests` (pass, 2 tests); `cargo xtask check-deps` (pass) | RED observed first (`DevassistError` and submodules missing). Added the required `bongterm-devassist -> bongterm-test-kit` allowed-deps edge after `check-deps` exposed the dev-dep matrix gap. | 3.A.1 |
| 3.A.1 | Complete | RED: `cargo test -p bongterm-devassist ai::runner` failed with unresolved `AiRequest`, `AiContext`, `UnavailableBackend`, `AiIntent`, `AiAvailability`. GREEN: `cargo test -p bongterm-devassist ai::runner` (pass, 2 tests); `cargo build -p bongterm-test-kit` (pass); `cargo xtask check-deps` (pass) | Added preview-only `AiBackend` port types, `UnavailableBackend`, test-kit `mocks::ai_backend::MockAiBackend`, placeholder notifier module, and dependency matrix edge. Made `MODULE_NAME` consts public to avoid normal-build dead-code warnings under future `-D warnings` checks. | 3.A.2 |
| 3.A.2 | Complete | RED: `cargo test -p bongterm-devassist ai::cmdk` first exposed a unit-test/test-kit type split, then failed correctly with unresolved `CmdKError`, `CmdKSession`, `CmdKState`. GREEN: `cargo test -p bongterm-devassist ai::cmdk` (pass, 3 tests); `cargo test -p bongterm-devassist` (pass, 7 tests) | Added `CmdKSession`, `CmdKState`, `CmdKError`, preview-only request flow, and explicit `confirm_run`. Kept mock-backed Cmd-K assertions in integration tests so `bongterm-test-kit` and devassist share the same external crate types. | 3.A.3 |
| 3.A.3 | Complete | RED: `cargo test -p bongterm-devassist ai::explainer` failed with unresolved `Explainer`. GREEN: `cargo test -p bongterm-devassist ai::explainer` (pass, 3 tests); `cargo test -p bongterm-devassist` (pass, 10 tests) | Added non-zero-exit `Explainer`, bounded transcript-tail context, `ExplainFailure` requests, and zero-exit refusal. Mock-backed tests live in integration tests to avoid duplicate crate types. | 3.A.4 |
