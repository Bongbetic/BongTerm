# BongTerm Handoff — CI made green (ci.yml red → green) — 2026-05-31

## TL;DR

- **All 7 `ci.yml` gates pass _locally_ on the CI-pinned stable 1.95 toolchain.**
  At session start three were red (`cargo fmt --all -- --check`, `cargo clippy
  --workspace --all-targets --all-features -- -D warnings`, `cargo deny check`).
  **Caveat: CI itself has not run.** `ci.yml` triggers on `push:[main]` + PRs and
  the working branch is `master`, so nothing has executed on GitHub's
  `windows-latest`. A `.gitattributes` (`eol=lf`) was added to close the most
  likely local↔CI divergence — Windows checkout (`core.autocrlf=true`) would
  otherwise produce CRLF working trees and fail rustfmt's `newline_style="Unix"`
  check. **To truly confirm green, open a PR or push to `main`.** Six commits
  landed on `master` (`ccef9ca`, `9c98f06`, `03b5678`, `41047c4`, `1cda90c`,
  `12db136`).
- **The uncommitted storage-sqlite coverage regression is resolved properly**
  (not by deleting tests). The 3 repo-conformance tests are restored; the
  `check-deps` dep-direction violation is fixed with a one-line allow-list entry.
- **The headless vertical-slice proof still passes** (`terminal_session.rs` —
  real shell spawn → parse → snapshot). The GUI *visual* check (glyphs render,
  typing visible) is the one thing this session could not verify (no display) —
  **it is the human-only next step.**
- "Ship" remains multi-session and calendar/human-bound (Phases 3–6 + 30-day
  dogfood + beta + trademark). This session removed CI blockers and cleaned the
  tree; it did not and could not finish the product.

## This session's commits (on `master`, oldest→newest)

| Commit | Summary |
|--------|---------|
| `ccef9ca` | fix(deps): allow storage-sqlite dev-dep on test-kit; **restore** the 3 repo-conformance tests + re-lock `Cargo.lock`. Fixes a `check-deps` failure that an uncommitted change had "fixed" by deleting coverage. |
| `9c98f06` | chore: make CI fmt + clippy gates green on stable 1.95. Stable rustfmt across all crates + ~38 clippy `-D warnings` lints resolved (behavior-preserving). |
| `03b5678` | chore(deny): ignore `adler` unmaintained advisory (RUSTSEC-2025-0056) + allow `WTFPL` license — both transitive via vendored wezterm, no upgrade path. |
| `41047c4` | docs: fix stale `Documents\Projects\BongT` paths (CLAUDE.md handoff rule, orca.md memory dir) after the move to `D:\Programming\Bongbetic\BongT`; align AGENTS.md with the local-`main` workflow. |
| `1cda90c` | docs: record this CI-green session (this handoff + SHIP-READINESS Update 3 + orca status). |
| `12db136` | build: add `.gitattributes` (`* text=auto eol=lf`) so the fmt gate survives Windows CI (`core.autocrlf=true` would otherwise check out CRLF and fail `newline_style="Unix"`). No content churn — all 201 tracked files already LF. |

## Verification (all run this session, stable 1.95, on `master` HEAD `41047c4`)

| Gate (ci.yml) | Command | Result |
|---|---|---|
| fmt | `cargo fmt --all -- --check` | exit 0 |
| clippy | `cargo clippy --workspace --all-targets --all-features -- -D warnings` | exit 0 |
| test | `cargo test --workspace` | 343 passed / 0 failed / 1 ignored |
| deny | `cargo deny check` | advisories ok, bans ok, licenses ok, sources ok |
| check-deps | `cargo xtask check-deps` | check-deps: ok |
| release smoke | `cargo build --release --workspace` | exit 0 (4m19s) |
| submodule | `git submodule status vendor/wezterm` | clean SHA (stray untracked `.gitkeep` removed) |
| slice proof | `cargo test -p bongterm-app` | `terminal_session` 1 passed |

## Important details / rationale

- **Why the clippy sweep touched ~25 files.** Under `-D warnings`, a denied lint
  in a dependency aborts that crate's metadata and *masks* lints in dependents.
  The first run only showed `bongterm-pty` (3 casts); fixing it revealed
  `bongterm-agents` (5), then `storage-sqlite`/`ledger`/`settings`/`xtask`, then
  `ui`/`render`/`app`/`blocks`/`term`. They were all enumerated in one pass
  (clippy without `-D warnings`) and fixed together. **All fixes are
  behavior-preserving** — widening casts → `From`; narrowing/wrapping casts kept
  verbatim under narrowly-scoped `#[allow]` + justification; Win32 FFI out-params
  use `&raw mut`; deliberate state machines keep explicit arms via
  `#[allow(clippy::match_same_arms)]`.
- **Security-sensitive fix reviewed by hand:** `tools/xtask/src/prompt_injection_corpus.rs`
  `GateEnforcement::Default` is deliberately hand-written to return
  `RequireApproval` (the conservative posture). Clippy's `derivable_impls` would
  have you `#[derive(Default)]`, which defaults to the *first* variant `Allow` and
  would **silently weaken gate #24**. Kept hand-written under `#[allow]` — do not
  "simplify" this.
- **rustfmt drift latent issue (not fixed, by design):** `rustfmt.toml` declares
  nightly-only options (`imports_granularity`, `group_imports`) but `ci.yml` runs
  fmt on **stable**, which ignores them. The code is now stable-formatted (CI
  passes). If anyone runs *nightly* `cargo fmt` it may re-introduce diffs. To end
  the drift permanently, either add a pinned-nightly fmt CI job or drop the two
  nightly-only opts from `rustfmt.toml`. Left as a deliberate decision for the
  team — out of scope for "make CI green."
- **WTFPL allow / adler ignore (legal/security note):** both are transitive
  through the vendored `wezterm-term` submodule with no upgrade path. WTFPL is
  FSF Free/Libre and imposes no obligations; the adler advisory is
  *unmaintained*, not a vulnerability. Surfaced here for visibility; revisit if
  the vendored wezterm is ever updated.

## What is green vs what remains

- **`ci.yml`: green locally on stable 1.95, NOT yet run on CI.** All 7 steps
  pass in a local reproduction, but `ci.yml` triggers on `push:[main]` + PRs and
  the working branch is `master`, so it has never executed on GitHub. Open a PR
  or push to `main` to get a real CI result (the only thing that actually
  confirms green). The line-ending divergence that most threatened the fmt gate
  is now closed by `.gitattributes` (`12db136`); a residual risk is anything else
  environmental (toolchain image, `cargo-deny` advisory-db drift over time).
  (The `master`-vs-`main` trigger mismatch is a latent config gap — noted, not
  fixed; decide whether to retarget the trigger or rename the branch.)
- **`nightly.yml`: gates #15 + #24 green**, but the **Phase 1 exit gates
  (#1,#4-8,#17,#28,#29) are still not wired** — that is task **`1.exit`** and it
  is the real remaining Phase-1 work. Those perf gates (keystroke-to-glyph p99,
  throughput, RSS/VRAM, live dashboard) need measurement harnesses built on the
  now-wired terminal pipeline; they are **unstarted**.

## Next actionable (pick one)

1. **Human-only:** run `cargo run -p bongterm-app` and confirm a live shell —
   glyphs render, typing is visible, `dir`/`ls` works. This is the one check no
   headless session can do; it gates calling the slice "done".
2. **`1.exit`** (orca `[next]`): build the Phase-1 perf-gate harnesses
   (#1,#4-8,#17,#28,#29) against the live `TerminalSession` pipeline and wire
   them into `nightly.yml`. Largest remaining Phase-1 item.
3. **Vertical-slice polish** (not yet in orca): resize (re-create PTY+adapter on
   window resize; currently fixed 80×24), colour/attrs in `current_snapshot`,
   fold the terminal surface into the `bongterm-ui` shell (needs a port so `ui`
   stays presentation-only), then swap the pragmatic iced-`text` grid for
   `bongterm-render::TerminalPipeline` (perf gates #1/#4/#6).
4. **`2.replan`** (orca): invoke `superpowers:writing-plans` for Phase 3 (consult
   AnythingLLM `engineer`; do not implement from outline alone).

## Key artifacts

| Artifact | Path |
|----------|------|
| Task authority | `orca.md` |
| Ship-readiness audit | `SHIP-READINESS.md` (see Update 3) |
| CI gates | `.github/workflows/ci.yml`, `.github/workflows/nightly.yml` |
| Dep-direction policy | `tools/xtask/allowed-deps.toml` + `tools/xtask/src/check_deps.rs` |
| cargo-deny policy | `deny.toml` |
| Phase 2 status table | `docs/codex/phase-status.md` |

*Generated 2026-05-31. All changes on `master` (HEAD `41047c4`), not pushed. No sensitive data.*
