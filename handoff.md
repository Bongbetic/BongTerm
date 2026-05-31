# BongTerm Handoff — `1.exit` measurable subset landed — 2026-05-31

## TL;DR

- Picked up orca `[next] = 1.exit` (Phase-1 exit gates). First found and fixed a
  **gate-numbering error**: orca's inline labels were off-by-one vs the canonical
  spec §6.1. The Phase-1 set is §6.1 **{1,4,5,6,7,8,17,28,29}** — #1 is *shell-
  profile launch* (not keystroke-to-glyph); #2/#3 (renderer perf) are deliberately
  **excluded** from Phase 1. New `docs/phase1-exit-gates.md` triages all nine.
- **Built + verified + wired + committed the 5 measurable gates** (no display/GPU
  needed): **#1** shell-smoke, **#5-RSS**, **#8** blocks, **#28** settings recovery,
  **#29** storage recovery. Commits `b81eaf0`→`2e0947e` on `master`.
- **5 gates remain BLOCKED** on the same root cause — the real subsystems
  (renderer / mux / ledger) are not wired into `bongterm-app`. These need a
  GPU/display **and a human visual check**: #4 cold-start-to-first-frame,
  #5-VRAM, #6 idle CPU, #7 split panes, #17 dashboard attribution. They were
  **not faked green** (per the documented anti-gate-gaming discipline).
- Post-session truth: `cargo test --workspace` = **350 pass / 0 fail / 1 ignored**;
  `cargo fmt --all -- --check` clean; `cargo clippy --workspace --all-targets
  --all-features -- -D warnings` clean. Verified by direct runs, not trusted.

## Commits this session (on `master`, oldest→newest)

| Commit | Summary |
|--------|---------|
| `b81eaf0` | docs(orca): triage 1.exit gates; fix off-by-one gate labels (+ `docs/phase1-exit-gates.md`). |
| `33add16` | feat(1.exit): gates **#1** shell-smoke + **#5-RSS** wired into `nightly.yml`. |
| `e87beb5` | feat(1.exit): gates **#8** blocks (corpus + latency p99) + **#29** storage recovery. |
| `2e0947e` | feat(1.exit): gate **#28** settings recovery — backup + Safe Mode + schema migration (real logic built in `bongterm-settings`). |
| *(pending)* | docs: mark 1.exit measurable subset done in orca + triage + this handoff. |

## Gate evidence (verified locally this session)

| Gate | Observable | Result |
|---|---|---|
| #1 | shell profiles launch (real ConPTY→`bongterm-term`→snapshot) | PASS 4/6: CMD, Windows PowerShell, PowerShell 7, SSH. SKIP-logged: Git Bash, WSL (no distro; `bash.exe` here is the WSL shim). CMD + WinPS are required so it can't pass vacuously. |
| #5 (RSS) | core process RSS ≤ 120 MB / 1 pane | **9.8 MB** via real `bongterm-ledger::CurrentProcessSampler` (`GetProcessMemoryInfo`). VRAM part of #5 BLOCKED. |
| #8 | block-detection corpus + latency p99 ≤ 5 ms | fixture corpus green + **p99 500 ns** over 10k iters (real `parse_osc 133;D → BlockBuilder::push → confidence()`). |
| #28 | settings load + validation-fail + backup + Safe Mode + migration | new logic in `bongterm-settings`; `settings_migration_and_last_known_good` drives all paths (backup read off disk + byte-compared; v1→v2 migration preserves user fields). |
| #29 | SQLite + sidecar recovery: torn / checksum / corrupt-DB | `storage_recovery_suite` green; corrupt DB → `SqliteStore::open()` `Err "file is not a database"` (WAL pragma forces page-1 read), no panic, no fabrication. |

CI steps added to `.github/workflows/nightly.yml` (one per gate, `--nocapture` so the
measured numbers surface in CI logs).

## Two real defects/notes surfaced (not yet actioned)

1. **`bongterm-ledger` doc-vs-code lie:** `CurrentProcessSampler`'s doc comment claims
   a `register_pid` method for child-PID attribution — **it does not exist**; the
   sampler only measures the current process. This is exactly the missing capability
   gate **#17** (dashboard attribution) needs. Building `register_pid` + process-tree
   attribution is **headless-testable** and is the recommended bounded next step.
2. **rustfmt nightly drift** (pre-existing): `rustfmt.toml` declares nightly-only opts
   ignored by the stable fmt gate — harmless now, latent. (Unchanged from prior handoff.)

## Next session — the integration wall (its own session + a human)

The remaining Phase-1 gates all need the real subsystems wired into `bongterm-app`,
which currently runs a single iced-`text` `TerminalSession` and consumes none of
`render`/`mux`/`ledger`. Honest path:

1. **`bongterm-ledger::register_pid` + child attribution** (autonomous, headless-
   testable) — closes the logic half of #17 and fixes defect #1.
2. **Wire `bongterm-render::TerminalPipeline` into the app** (autonomous code; human
   visual) — unblocks #4 (cold-start-to-first-frame), #5-VRAM, #6 (idle CPU). Also
   unblocks §6.1 **#2/#3** (the renderer-perf gates that are in *no* phase's exit set
   — see `docs/phase1-exit-gates.md`).
3. **Wire `bongterm-mux` panes into the app** (autonomous code; human visual) — #7.
4. **Wire the ledger dashboard view into the app** — #17 UI half.
5. **GUI visual verify** (human-only): `cargo run -p bongterm-app` — glyphs render,
   typing visible, panes split, dashboard shows attribution.
6. **Confirm CI for real**: fix the `master`-vs-`main` trigger mismatch on
   `ci.yml`/`nightly.yml`, then push/PR so they run on `windows-latest`.

Do **not** mark #4/#5-VRAM/#6/#7/#17 green from a headless harness — they are only
real with a display/GPU and a human looking at the window.

## Key artifacts

| Artifact | Path |
|----------|------|
| Task authority | `orca.md` |
| Gate triage (this work) | `docs/phase1-exit-gates.md` |
| Ground-truth audit | `SHIP-READINESS.md` |
| CI gates | `.github/workflows/ci.yml`, `.github/workflows/nightly.yml` |
| Canonical gate criteria | `docs/superpowers/specs/2026-05-27-bongt-mvp0-design.md` §6.1 |

*Generated 2026-05-31. All changes on `master`, not pushed. No sensitive data.*
