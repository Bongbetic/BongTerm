# BongTerm Handoff — Phase 2 code complete — 2026-05-31

## TL;DR

- **Repo moved** `C:\Users\souba\Documents\Projects\BongT` → `D:\Programming\Bongbetic\BongT`. Many docs still cite the old path. A stale incremental build cached the old `CARGO_MANIFEST_DIR`, causing 5 phantom `bongterm-blocks` failures — fixed by rebuild. **Run `cargo clean` once** on a fresh clone if you see `os error 3` / path-not-found in fixture tests.
- **Phase 2 (agent observability) is code-complete.** Tasks 2.C.3a → 2.EXIT landed this session. Both P0 gates (#15, #24) green locally + wired into `nightly.yml`.
- **The vertical slice landed — the app now runs a real shell.** `cargo run -p bongterm-app` opens a window running pwsh/cmd. New `bongterm-app::session::TerminalSession` (ConPTY + parser) + a thin iced shell (`terminal_app.rs`); `WezTermAdapter::current_snapshot` was a stub (empty runs) and is now implemented. Proven headlessly (`tests/terminal_session.rs`) + a clean 6s launch. **Not visually verified** (no display in-session) — run it to confirm glyphs/typing. Renderer is pragmatic iced-`text` (not wgpu yet); 80×24 fixed; `bongterm-ui` shell bypassed. See `SHIP-READINESS.md` Update 2.

## This session's commits (on `master`)

| Commit | Task | Summary |
|--------|------|---------|
| `31a9c0e` | 2.C.3a | `corpus.rs`: `InjectionScenario` model + `load_dir` loader |
| `8c9924a` | 2.C.3b | 32 injection fixtures (30 poisoned + 2 benign) + detection-alignment test |
| `2dd8048` | 2.C.3c | `xtask prompt-injection-corpus` gate #24 runner |
| `79fecc2` | 2.D.1 | `tests/gate15.rs` — gate #15 offline launch + transcript-capture |
| `662e31b` | 2.EXIT | `nightly.yml` — wire gates #15 + #24 into nightly |

(`+` a docs commit updating `orca.md`, `phase-status.md`, `SHIP-READINESS.md`, this handoff.)

## Verification (all run this session)

- `cargo test -p bongterm-agents corpus::` → 3 pass
- `cargo test -p xtask prompt_injection_corpus::tests` → 6 pass
- `cargo run -p xtask -- prompt-injection-corpus` → `32 scenarios passed gate #24`, exit 0
- `cargo test -p bongterm-agents --test gate15` → 3 pass, 1 ignored
- `cargo test --workspace` → green; `cargo run -p xtask -- check-deps` → ok

## Plan inconsistencies reconciled (read before trusting the Phase 2 plan verbatim)

The Phase 2 plan (`docs/superpowers/plans/2026-05-29-bongt-phase2.md`) drifted from the committed code in three places; all reconciled, documented in commit messages:

1. **2.C.3c schema** — plan's xtask `Scenario` used `payload` + `expected_enforcement`; the 2.C.3a/b fixtures use `poisoned_content` + `provoked_action` (no enforcement). Fixed via `#[serde(alias = "poisoned_content")]` + `#[serde(default)]` on the xtask struct. No fixture churn.
2. **2.C.3c markers** — plan's pasted `MARKERS` had drifted from the real `classify::INJECTION_MARKERS`; 9 fixtures would have missed. Set `MARKERS` byte-identical to the committed `classify` list (the plan's own stated invariant).
3. **2.D.1 gate15 APIs** — plan used fictional signatures (`TranscriptSink::append`/`captured_text`, `LifecycleCommand::ObserveExit`, `status_label()`, names "Claude Code"/"Codex CLI"). Real APIs: `capabilities().name` = `claude-code`/`codex-cli`; `ProcessExited` + `state() -> LifecycleState`; transcript captured from `AgentEvent::Output`.

## Known gaps / pending items

- **`1.exit` still pending** (Phase 1 CI gate wiring #1,#4-8,#17,#28,#29). Phase 2 was built ahead of it. A fully green nightly needs both.
- **Marker drift guard missing.** The plan attributes a `markers_match_xtask_corpus_runner` drift test to 2.A.3; it does not exist. Neither crate imports the other, so it needs a third mechanism (e.g. an `xtask` check that parses both source lists). The two lists are currently equal by hand.
- **Workspace clippy/fmt debt.** `cargo clippy --workspace --all-targets -- -D warnings` and `cargo fmt --all --check` fail on pre-existing issues in other crates (`bongterm-settings` missing `# Panics`, `map_or`/match-arm/derivable-impl, etc.) and the nightly-only rustfmt config on the stable toolchain. Phase 2 code itself is clippy-clean. This blocks ci.yml's existing clippy/fmt gates — needs a hygiene pass (1.exit territory).
- **Uncommitted pre-existing changes** still in the tree (not from this session): `crates/bongterm-storage-sqlite/{Cargo.toml,src/lib.rs}` removes the `bongterm-test-kit` dev-dep + 3 repo-conformance tests to satisfy `check-deps` — a **coverage regression**. Resolve properly (host the storage conformance harness in `bongterm-test-kit`, which already depends on the trait crates, and run it against `SqliteStore` there) or revert. Also `AGENTS.md`, `Cargo.lock` modified.

## Vertical-slice commits (this session, after Phase 2)

| Commit | What |
|--------|------|
| `54d17a0` | `TerminalSession` core + real `WezTermAdapter::current_snapshot`; headless proof |
| `6a5cd26` | iced terminal shell (`terminal_app.rs`); `main.rs` repointed; app runs a real shell |

## Next actionable

Highest-value follow-ups on the slice (none are in orca.md yet):
1. **Visually verify** `cargo run -p bongterm-app` — confirm glyphs render and typing works; fix render/input issues a headless test can't catch.
2. **Resize** (re-create PTY + adapter on window resize; currently fixed 80×24).
3. **Colour/attributes** — `current_snapshot` emits one run/row with default colours; extract per-cell fg/bg/attrs and have the renderer honour them.
4. **Fold into the `bongterm-ui` shell** — host the terminal surface inside `BongTermShell` (tabs/palette/sidebar) instead of bypassing it. Needs a port so ui stays presentation-only (ui can't depend on pty/term).
5. **wgpu renderer** — swap the pragmatic iced-`text` grid for `bongterm-render::TerminalPipeline` behind the `SurfaceSnapshot` boundary (perf gates #1/#4/#6).

Or resume orca.md: `2.replan` — invoke `superpowers:writing-plans` for Phase 3 (consult AnythingLLM `engineer`; **do not implement from outline alone**).

## Key artifacts

| Artifact | Path |
|----------|------|
| Ship-readiness audit | `SHIP-READINESS.md` (repo root) |
| Task authority | `orca.md` |
| Phase 2 status table | `docs/codex/phase-status.md` |
| Phase 2 plan | `docs/superpowers/plans/2026-05-29-bongt-phase2.md` |
| Execution rules | `AGENTS.md` |

*Generated 2026-05-31. All changes on `master`. No sensitive data.*
