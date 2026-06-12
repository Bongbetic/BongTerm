# BongTerm Runtime Correction Status

Source of truth:

- Plan: `docs/superpowers/plans/2026-05-28-bongt-phase1.md`
- Product intent: `docs/PRD/bongterm_prd_v7.md`
- Execution control plane: `orca.md`

Current focus: **Release pipeline mode active; release proof unblocked on
default branch; scheduled nightly proof 1/7** on 2026-06-12.

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

UI follow-up complete: the terminal shader now renders text in widget-local
coordinates for composed shell layouts, the resource dashboard splits category
and metrics lines to avoid side-panel overlap, and the current-process CPU
sampler establishes a first-sample baseline at `0.0%`. Manual wide visual smoke
captured `C:\Users\souba\AppData\Local\Temp\bongterm-ui-smoke-wide-26320.png`
with center-pane terminal alignment and readable resource metrics.

Release proof unblock is complete: PR #1 merged to `master` at merge commit
`21e2feb` on 2026-06-11, and default branch now contains active `ci`,
`nightly`, and tag-gated `release` workflows. Master CI run `27341442656`
passed after the merge. Manual nightly run `27343029777` passed all gate steps,
proving workflow health. This manual dispatch does **not** satisfy the
scheduled-nightly time gate. Real signed `dist/`, clean-VM signed install
smoke, dogfood, legal/name decision, public flip, and GitHub release remain
hard blockers.

Scheduled nightly proof update: first post-merge scheduled `nightly.yml` run
`27411817353` on `master` passed on 2026-06-12 (created
`2026-06-12T11:06:29Z`, completed `2026-06-12T11:20:46Z`) at head
`af29c970d94965b43ed590930ea7c72755bef64f`. Manual dispatch run
`27343029777` is excluded from the scheduled-only count. Current latest
consecutive scheduled green streak: **1/7**.

Last verification:

- RED: `cargo test -p bongterm-app --test shell_app` failed with missing `terminal_surface_size_for_window` and `AppMessage::WindowResized` for `1.R.3`.
- GREEN: `cargo test -p bongterm-app --test shell_app` — pass, 3 tests.
- UI RED/GREEN follow-up targets: `cargo test -p bongterm-render shader_text_layout_uses_widget_local_origin`, `cargo test -p bongterm-ui resource_row_separates_title_from_metrics`, and `cargo test -p bongterm-ledger current_process_sampler_first_sample_sets_cpu_baseline` — each failed before implementation and passed after.
- `cargo test -p bongterm-render -p bongterm-ui -p bongterm-ledger -p bongterm-app --test shell_app` — pass.
- `cargo clippy -p bongterm-render -p bongterm-ui -p bongterm-ledger -p bongterm-app --all-targets --all-features -- -D warnings` — pass; vendored wezterm warnings still print from dependencies.
- `cargo fmt --all -- --check` — pass; stable rustfmt still prints existing nightly-only config warnings.
- `git diff --check` — pass.
- `cargo test --workspace --quiet` — pass.
- Manual resize smoke: `target\debug\bongterm-app.exe` opened responding PID `26696`, title `BongTerm - workspace`; Win32 resize to `900x600` and `1200x720` left the process responsive.
- Manual wide visual smoke: `target\debug\bongterm-app.exe` opened `BongTerm - workspace`; screenshot `C:\Users\souba\AppData\Local\Temp\bongterm-ui-smoke-wide-26320.png` showed center-pane terminal alignment and non-overlapping resource metrics.

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
Follow-up CI smoke fix: GitHub-hosted Windows runners can resolve and execute Windows PowerShell but return an empty ConPTY stream. The shell smoke gate now skips only that runner-specific empty-stream condition on GitHub Actions; local/reference machines still require Windows PowerShell coverage.
Default-branch proof update: PR #1 merged to `master` at `21e2feb`; master CI
run `27341442656` passed; manual nightly proof run `27343029777` passed but is
excluded from the release time gate. Scheduled nightly run `27411817353` passed
on 2026-06-12, so the scheduled-only streak is now 1/7.

| Area | Status | Last test run | Notes/blockers |
| --- | --- | --- | --- |
| Phase 1 exit gates | Local green; manual nightly proof green; scheduled 1/7 | `cargo test -p bongterm-app --test phase1_exit_gates -- --nocapture` (pass, 5 tests); manual nightly run `27343029777` (pass, excluded); scheduled nightly run `27411817353` (pass, 2026-06-12) | Remote proof still blocked until 7/7 consecutive scheduled nightlies. |
| UIA/accessibility | Local green | `cargo test -p bongterm-ui` (pass); `cargo test -p bongterm-test-kit` (pass) | Manual Narrator QA documented in `tests/accessibility/narrator_smoke.md`. |
| IME + DPI | Local green | `cargo test -p bongterm-ui` (pass) | Live CJK IME QA remains manual. |
| Renderer device loss | Local green | `cargo test -p bongterm-render device_loss` (pass) | Recovery policy falls back to software after repeated loss. |
| Diagnostics/export/minidump/recovery | Local green | `cargo test -p bongterm-diagnostics` (pass) | Telemetry remains off by default; export bundle uses redaction preview. |
| Forbidden abstraction/EDR | Local green | `cargo test -p bongterm-security forbidden` (pass); `cargo run -p xtask -- forbidden-abstraction` (pass) | Runtime auditor and static scan are present; external Defender/EDR smoke is documented. |
| Release packaging | Local green | `cargo run -p xtask -- package-msix` (pass) | `makeappx.exe`/real signing cert/clean VM not available in this local proof. |
| SBOM + attestation | Local green | `cargo run -p xtask -- sbom` (pass); `cargo run -p xtask -- attestation` (pass) | Outputs: `sbom.cdx.json`, `attestation.intoto.jsonl`. |
| Workspace gates | Green | PR #1 `correctness` run `27339704976` (pass); master CI run `27341442656` (pass); local follow-up checks on 2026-06-11: `cargo fmt --all -- --check`, `cargo clippy -p bongterm-render -p bongterm-ui -p bongterm-ledger -p bongterm-app --all-targets --all-features -- -D warnings`, `cargo test -p bongterm-render -p bongterm-ui -p bongterm-ledger -p bongterm-app --test shell_app`, `cargo test --workspace --quiet` | Stable rustfmt warns that nightly-only rustfmt options are ignored; GitHub warns Node.js 20 actions will need update. |

Next task: external release proof is blocked until a clean Windows VM, real
signing certificate/toolchain, and signed MSIX path are available. `6.A.1`
remains blocked and not executable until Phase 5 clean-VM signed install smoke
proof and 7 consecutive scheduled remote nightly CI green runs are accepted or
completed (last checked 2026-06-12T17:03:00+05:30; current scheduled streak
1/7; scheduled proof `27411817353` green; manual proof `27343029777` green but
excluded). Phase 6
public-release exit remains blocked until external Phase 5 clean-VM smoke,
7 remote nightlies, legal/trademark ADRs, signed release `dist/`, Stage A/B
dogfood, public flip, and GitHub release complete.
