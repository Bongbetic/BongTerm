# BongTerm Phase 5 Status

Source of truth:

- Plan: `docs/superpowers/plans/2026-05-29-bongt-phase5.md`
- Product intent: `docs/PRD/bongterm_prd_v7.md`
- Execution control plane: `orca.md`

Current focus: **Phase 5 local implementation complete and committed; Phase 6 prep-only task 6.A.0 completed** on 2026-06-03.

Commit: `d221e06 feat(phase5): close hardening release prep`

Branch: `codex/phase5-hardening-closeout`

Phase 1 exit closure: local gates #1,#4-8,#17,#28,#29 are green. The remaining Phase 1 exit proof is the required 7 consecutive remote nightlies.

Phase 2 exit closure: local gates #15 and #24 are green and wired into nightly. The remaining Phase 2 exit proof is the required 7 consecutive remote nightlies.

Phase 5 exit closure: local code/doc/tooling gates are green and committed. The remaining Phase 5 exit proof is a signed MSIX install/upgrade/uninstall smoke on a clean Windows VM with the real signing toolchain/cert.

Phase 6 prep: `docs/dogfood/README.md`, `docs/dogfood/_template.md`, and `docs/dogfood/stage-a-summary.md` now exist for 6.A.0. Stage A dogfood has **not** started.

Additional Phase 6 local prep completed: Stage B plan/summary skeletons, public-flip/community docs, install/privacy docs, static landing page, and xtask `checksums`, `release-verify`, and `site-check` subcommands. Local xtask tests are green.

Push/remote proof blocker: `git push -u origin codex/phase5-hardening-closeout` was rejected because the GitHub OAuth token lacks `workflow` scope for changed `.github/workflows/*.yml` files.
Resolved for testing: branch pushed over SSH and PR #1 opened (`https://github.com/soubarnak/BongTerm/pull/1`), which starts PR CI. SECURITY placeholder is removed in favor of GitHub private vulnerability reporting. A dev-signed MSIX smoke artifact exists under `target/msix/` with public cert `target/msix/BongTerm-Dev.cer`.

| Area | Status | Last test run | Notes/blockers |
| --- | --- | --- | --- |
| Phase 1 exit gates | Local green | `cargo test -p bongterm-app --test phase1_exit_gates -- --nocapture` (pass, 5 tests) | Remote 7-nightly proof still external/time-bound. |
| UIA/accessibility | Local green | `cargo test -p bongterm-ui` (pass); `cargo test -p bongterm-test-kit` (pass) | Manual Narrator QA documented in `tests/accessibility/narrator_smoke.md`. |
| IME + DPI | Local green | `cargo test -p bongterm-ui` (pass) | Live CJK IME QA remains manual. |
| Renderer device loss | Local green | `cargo test -p bongterm-render device_loss` (pass) | Recovery policy falls back to software after repeated loss. |
| Diagnostics/export/minidump/recovery | Local green | `cargo test -p bongterm-diagnostics` (pass) | Telemetry remains off by default; export bundle uses redaction preview. |
| Forbidden abstraction/EDR | Local green | `cargo test -p bongterm-security forbidden` (pass); `cargo run -p xtask -- forbidden-abstraction` (pass) | Runtime auditor and static scan are present; external Defender/EDR smoke is documented. |
| Release packaging | Local green | `cargo run -p xtask -- package-msix` (pass) | `makeappx.exe`/real signing cert/clean VM not available in this local proof. |
| SBOM + attestation | Local green | `cargo run -p xtask -- sbom` (pass); `cargo run -p xtask -- attestation` (pass) | Outputs: `sbom.cdx.json`, `attestation.intoto.jsonl`. |
| Workspace gates | Green | `cargo fmt --all -- --check` (pass); `cargo clippy --all-targets --all-features --workspace -- -D warnings` (pass); `cargo test --workspace` (pass); `cargo xtask check-deps` (pass) | Stable rustfmt warns that nightly-only rustfmt options are ignored. |

Next task: begin local testing from PR #1 and dev-signed MSIX smoke. Phase 6 public-release exit remains blocked until external Phase 5 clean-VM smoke, 7 remote nightlies, legal/trademark ADRs, signed release `dist/`, Stage A/B dogfood, public flip, and GitHub release complete.
