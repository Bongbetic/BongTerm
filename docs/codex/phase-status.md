# BongTerm Runtime Correction Status

Source of truth:

- Plan: `docs/superpowers/plans/2026-05-28-bongt-phase1.md`
- Product intent: `docs/PRD/bongterm_prd_v7.md`
- Execution control plane: `orca.md`

Current focus: **Release pipeline mode active; Phase 1 runtime correction
locally complete; release proof unblock next** on 2026-06-11.

Workflow reset: user approved the public `v0.1.0-mvp0` ship plan and explicitly
chose to change workflow cadence. `AGENTS.md` and `orca.md` now allow controlled
release pipeline mode: continue sequential planned tasks in one session only
after each task has its RED/GREEN checks, required verification, status updates,
and blocker assessment. This does not relax external/manual release gates.

Completed tasks: `1.R.1`, `1.R.2`, and `1.R.3` runtime correction. The running
binary composes `bongterm-ui` shell chrome with the existing live terminal
runtime, renders `AgentSidebarVm::view()` in the agent panel, renders a UI-local
resource dashboard DTO translated from `bongterm-ledger::DashboardViewModel`,
and routes composed-app resize through shell-owned center-pane surface sizing
before resizing terminal PTY/parser/grid state.

Next task: release proof unblock — inspect default/local workflow state, add or
repair tag-gated release workflow support if missing, and prepare the push/merge
path required to start remote nightly/release proof.

Last verification:

- RED: `cargo test -p bongterm-app --test shell_app` failed with missing `terminal_surface_size_for_window` and `AppMessage::WindowResized` for `1.R.3`.
- GREEN: `cargo test -p bongterm-app --test shell_app` — pass, 3 tests.
- `cargo test -p bongterm-ui` — pass, 46 tests.
- `cargo clippy -p bongterm-app -p bongterm-ui --all-targets --all-features -- -D warnings` — pass; vendored wezterm warnings still print from dependencies.
- `cargo build -p bongterm-app` — pass.
- `cargo fmt --all -- --check` — pass; stable rustfmt still prints existing nightly-only config warnings.
- `git diff --check` — pass.
- Manual resize smoke: `target\debug\bongterm-app.exe` opened responding PID `26696`, title `BongTerm - workspace`; Win32 resize to `900x600` and `1200x720` left the process responsive.

Commit: `d221e06 feat(phase5): close hardening release prep`

Branch: `codex/phase5-hardening-closeout`

Phase 1 exit closure: local gates #1,#4-8,#17,#28,#29 are green and runtime
correction is locally complete through `1.R.3`. The remote exit proof is still
the required 7 consecutive remote nightlies.

Phase 2 exit closure: local gates #15 and #24 are green and wired into nightly. The remaining Phase 2 exit proof is the required 7 consecutive remote nightlies.

Phase 5 exit closure: local code/doc/tooling gates are green and committed. The remaining Phase 5 exit proof is a signed MSIX install/upgrade/uninstall smoke on a clean Windows VM with the real signing toolchain/cert.

Phase 6 prep: `docs/dogfood/README.md`, `docs/dogfood/_template.md`, and `docs/dogfood/stage-a-summary.md` now exist for 6.A.0. Stage A dogfood has **not** started.

Additional Phase 6 local prep completed: Stage B plan/summary skeletons, public-flip/community docs, install/privacy docs, static landing page, and xtask `checksums`, `release-verify`, and `site-check` subcommands. Local xtask tests are green.

Push/remote proof blocker: `git push -u origin codex/phase5-hardening-closeout` was rejected because the GitHub OAuth token lacks `workflow` scope for changed `.github/workflows/*.yml` files.
Resolved for testing: branch pushed over SSH and PR #1 opened (`https://github.com/soubarnak/BongTerm/pull/1`). PR #1 `correctness` run `27318475490` passed on 2026-06-11 after commit `2cc345a fix(app): keep startup off font probing`; the prior Gate #4 CI failure was fixed by removing synchronous font probing from app boot. SECURITY placeholder is removed in favor of GitHub private vulnerability reporting. A dev-signed MSIX smoke artifact exists under `target/msix/` with public cert `target/msix/BongTerm-Dev.cer`.

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
| Workspace gates | Green | PR #1 `correctness` run `27318475490` (pass); local targeted checks on 2026-06-11: `cargo fmt --all -- --check`, `cargo clippy -p bongterm-render -p bongterm-app --all-targets --all-features -- -D warnings`, `cargo test -p bongterm-render`, `cargo test -p bongterm-app -j 1` | Stable rustfmt warns that nightly-only rustfmt options are ignored. |

Next task: release proof unblock is actionable locally. `6.A.1` remains blocked
and not executable until Phase 5 clean-VM signed install smoke proof and 7
consecutive remote nightly CI green runs are accepted or completed (last checked
2026-06-11T07:56:30+05:30). Phase 6 public-release exit remains blocked until
external Phase 5 clean-VM smoke, 7 remote nightlies, legal/trademark ADRs,
signed release `dist/`, Stage A/B dogfood, public flip, and GitHub release
complete.
