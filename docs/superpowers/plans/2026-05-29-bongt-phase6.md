# BongTerm Phase 6 Execution Plan (Dogfood + Public Release)

Date: 2026-05-29
Source: `docs/superpowers/specs/2026-05-27-bongt-mvp0-design.md` (§6.1 gate #22, §6.2 dogfood stages, §6.4 artifact set, §6.6 ship-when checklist, §19.3 landing); PRD v7 §18.2/§18.4/§19.3/§20.2.
Status: Active
For agentic workers: this plan is written for an agent worker executing tasks one at a time. Each task has exact files/artifacts, explicit steps, and a measurable exit criterion or a TDD failing-check-first ordering. Process gates record completion in a named artifact. Do not skip a task's exit criterion to advance the `[next]` marker.

## Goal

Take MVP-0 from "all gates green" to a shipped public experimental `v0.1.0-mvp0` GitHub release by completing solo + trusted-circle dogfood, brand/legal readiness, repo public flip, and a fully signed/verifiable release artifact set.

## Architecture / Approach

Phase 6 is a release phase, not a feature phase: most work is process gates (dogfood logs, trademark search, brand review, user recruitment) plus release-engineering tooling that produces and verifies a tamper-evident artifact set. Verifiable tooling (checksums, signature verification, attestation, SBOM, artifact-completeness check, landing-page build) is built TDD-first behind BongTerm-owned `cargo xtask` subcommands and a release verifier so a single command can prove the release is shippable; inherently manual gates (30-day dogfood, trademark/registry search, recruiting) are driven by crisp acceptance checks recorded in tracking artifacts under `docs/dogfood/` and `docs/adr/`.

## Tech Stack / Tooling

- Release tooling: `cargo xtask` subcommands (`release-verify`, `checksums`, `attestation`, extends existing `sbom`, `package-msix`, `bench-report`, `check-licenses`) — Rust, reusing the existing `tools/xtask/` runner (`clap`-based, modules per subcommand).
- Signing: `signtool.exe` (OV cert) per `docs/runbook/code-signing.md`; detached signing for `checksums.txt` via the signing cert (PowerShell `Set-AuthenticodeSignature` over a catalog, or `signtool sign /fd sha256` over the checksums file — concrete command pinned in the task).
- Provenance: SLSA-style in-toto statement emitted as `attestation.intoto.jsonl` by `cargo xtask attestation`; verified by `cargo xtask release-verify`.
- SBOM: CycloneDX `sbom.cdx.json` via existing `cargo xtask sbom` (CycloneDX 1.5 JSON).
- CI: GitHub Actions, extending `.github/workflows/ci.yml` (nightly job) + new `.github/workflows/release.yml` (tag-triggered).
- Landing page: static HTML/CSS under `site/` built and link-checked by `cargo xtask` (no JS framework, no build server — plain static files).
- Tracking artifacts: Markdown under `docs/dogfood/`, `docs/adr/`, `docs/runbook/`.

---

## File / Artifact Structure

Every file or artifact Phase 6 creates or finalizes, and its responsibility:

| Path | Kind | Responsibility |
|---|---|---|
| `docs/dogfood/README.md` | doc | Stage A protocol, daily-log template, weekly-workload tracker rules, fallback-logging discipline. |
| `docs/dogfood/_template.md` | template | Per-day log skeleton (date, terminal-as-default confirmation, workloads exercised, fallbacks, defects filed, secret-leak check). |
| `docs/dogfood/<date>.md` (×30 working days) | log | One per working day of Stage A; canonical evidence for §6.1 #22 / §6.2 Stage A. |
| `docs/dogfood/stage-a-summary.md` | report | Stage A roll-up: day count, workload-minimum coverage matrix, open P0/P1 defects (must be 0), confirmed secret leaks (must be 0), exit verdict. |
| `docs/dogfood/stage-b-recruiting.md` | tracker | Stage B candidate list, channels posted, commitments, NDA/expectation notes, 14-day window dates. |
| `docs/dogfood/stage-b-summary.md` | report | Aggregated Stage B findings, defect triage, per-user agent-workflow completion, "no public-facing defect" verdict. |
| `docs/dogfood/secret-leak-audit.md` | report | Result of scanning all dogfood transcripts/exports for secrets; must conclude zero confirmed leaks. |
| `docs/adr/0002-product-name.md` | ADR | Finalized brand decision (status flips Pending→Accepted) with trademark + perception evidence. |
| `docs/adr/0002-brand-perception-notes.md` | doc | "bong" connotation review across target geographies; feeds ADR-0002 decision. |
| `docs/adr/0009-trademark-search.md` | ADR | Records USPTO/EUIPO/Indian-TM/GitHub/npm/crates/domain search results + conflict verdict. |
| `docs/runbook/release.md` | runbook | Finalized end-to-end release procedure + rollback/rollforward + dry-run (replaces placeholder). |
| `docs/runbook/smartscreen.md` | runbook | Finalized SmartScreen warm-up plan with executed-step log section. |
| `docs/runbook/public-flip-checklist.md` | runbook | Pre-flip repo-readiness gate (README/CONTRIBUTING/LICENSE/issue templates/secret scan). |
| `README.md` | doc | Public-facing project README (value prop, install, status disclaimer). Finalized for public flip. |
| `CONTRIBUTING.md` | doc | Contribution guidelines, profile-PR policy (PRD §18.5), security-disclosure pointer. |
| `LICENSE` (+ `LICENSE-APACHE`) | license | Apache-2.0 license text at repo root. |
| `CODE_OF_CONDUCT.md` | doc | Community conduct policy. |
| `.github/ISSUE_TEMPLATE/bug_report.md`, `feature_request.md`, `config.yml` | templates | Issue intake; routes security reports to `SECURITY.md`. |
| `.github/PULL_REQUEST_TEMPLATE.md` | template | PR checklist incl. tests/security review. |
| `SECURITY.md` | doc | Finalized real disclosure inbox, SLA, supported versions (replaces PLACEHOLDER). |
| `PRIVACY.md` | doc | Privacy notice (telemetry off by default). |
| `CHANGELOG.md` | doc | `v0.1.0-mvp0` entry finalized. |
| `known-issues.md` | doc | Published P1 exceptions + dogfood-surfaced non-blocking issues, with rationale + timeline. |
| `INSTALL.md` | doc | Install + signature/checksum verification + SmartScreen guidance. |
| `THIRD_PARTY_NOTICES.md` | artifact | Regenerated, validated against vendored WezTerm + Cargo.lock. |
| `sbom.cdx.json` | artifact | Release SBOM (generated into `dist/`). |
| `benchmark-report.md` | artifact | Reference-hardware benchmark results for release notes. |
| `attestation.intoto.jsonl` | artifact | SLSA provenance statement for the MSIX. |
| `tools/xtask/src/checksums.rs` | code | `cargo xtask checksums` — emit `*.sha256` + `checksums.txt` over `dist/`. |
| `tools/xtask/src/attestation.rs` | code | `cargo xtask attestation` — emit in-toto provenance for the MSIX. |
| `tools/xtask/src/release_verify.rs` | code | `cargo xtask release-verify` — assert full artifact set present, checksums match, signatures valid, attestation present, SBOM references WezTerm, no secrets in artifacts. |
| `tools/xtask/tests/release_verify_tests.rs` | test | Conformance tests for checksums/attestation/release-verify against fixture `dist/`. |
| `tools/xtask/tests/fixtures/dist-good/`, `dist-missing-artifact/`, `dist-bad-checksum/`, `dist-secret-leak/` | fixtures | Verifier test fixtures. |
| `site/index.html`, `site/style.css`, `site/assets/` | site | Landing page (PRD §19.3 / spec §19.3 copy). |
| `tools/xtask/src/site_check.rs` | code | `cargo xtask site-check` — HTML validity + dead-link + required-claim presence check. |
| `.github/workflows/release.yml` | CI | Tag-triggered job: build → package → sbom → checksums → attestation → release-verify → draft GitHub release. |
| `.github/workflows/ci.yml` | CI | Extended with the 7-consecutive-nightly green-gate tracker job. |
| `dist/` | output dir | Assembled release artifact staging directory (git-ignored). |

`dist/` is added to `.gitignore`; generated artifacts are attached to the GitHub release, not committed (except the review-committed `THIRD_PARTY_NOTICES.md`, `sbom.cdx.json` snapshot if policy requires, `CHANGELOG.md`, `known-issues.md`).

---

## Scope Locks (binding — do not pull forward)

1. **No Phase 6-Post-MVP items** (CLAUDE.md §Execution phasing): no Markdown review, no Command Lens, no database branching, no durable session daemon, no plugin marketplace, no cross-platform ports. If a dogfood request asks for any, log it as a `0.2.0+` item in `known-issues.md`, do not build it.
2. **Security contract holds through release** (CLAUDE.md §Security contract): secrets never appear in artifacts, exports, logs, transcripts, or the SBOM; `SECURITY.md` inbox is monitored before the public flip.
3. **Gate rule**: all 25 P0 §6.1 gates green for **7 consecutive nightly CI runs** before public release; P1 exceptions only via documented `known-issues.md` entries (experimental release).
4. **Definition of Done (PRD §34)** applies to every tooling task: tests, error/degraded states, security review, docs updated.
5. **Release type is `0.1.0-mvp0` public experimental**: Stage A mandatory; Stage B required for this plan's exit but the landing page carries the experimental disclaimer (spec §6.2 table).

---

## Ordered Work Plan

Tasks are grouped to match the `orca.md` Phase 6 outline (`6.A` Stage A, `6.B` Stage B, `6.C` brand/legal, `6.D` release engineering + flip, `6.exit`). Verifiable tooling tasks (`6.D.4.*`, `6.D.5.*`) use TDD ordering. Process gates state precondition → action → exit criterion → recording artifact.

### 6.A — Stage A: solo dogfood (maps orca 6.A.1–6.A.3)

- [ ] **6.A.0 Author Stage A protocol + log template.**
  - Files: `docs/dogfood/README.md`, `docs/dogfood/_template.md`, `docs/dogfood/stage-a-summary.md` (skeleton with empty matrix).
  - Steps: Write the Stage A rules from spec §6.2 verbatim into `README.md` (30 consecutive working days; BongTerm as default terminal; fallback-logging discipline; the 7 weekly/daily workload minimums). Create `_template.md` with fields: `date`, `bongterm_default: yes/no`, `workloads_today: [...]`, `fallbacks: [{tool, reason, duration, is_blocker}]`, `defects_filed: [...]`, `secret_leak_check: pass/fail`. Create `stage-a-summary.md` with a 7-row workload-coverage matrix (one row per minimum) and counters for working-days, open P0/P1, confirmed secret leaks.
  - Exit criterion: all three files exist; `_template.md` has all seven workload fields; `stage-a-summary.md` matrix lists all seven §6.2 minimums by name.
  - Records completion: the files themselves; commit `docs(phase6/6.A.0): Stage A dogfood protocol + log template`.

- [ ] **6.A.1 Set BongTerm as default terminal; begin daily logging.**
  - Precondition: Phase 5 exit done — signed/installable build exists and §6.1 gates green; `docs/dogfood/_template.md` exists.
  - Action: Set the Phase-5 build as the Windows default terminal (Settings → Privacy & security → For developers → Terminal, or `wt`-equivalent default-terminal registration as documented in `INSTALL.md`). Use it for all real terminal work. Each working day, copy `_template.md` to `docs/dogfood/<YYYY-MM-DD>.md` and fill it in.
  - Exit criterion (per-day): a `docs/dogfood/<date>.md` exists for each working day with `bongterm_default: yes` (or a logged fallback with reason/duration/is_blocker), `secret_leak_check: pass`, and `defects_filed` listing any tracker IDs.
  - Records completion: dated logs; this task stays `in_progress` for the full window — advance only when 6.A.3 exit passes.

- [ ] **6.A.2 Exercise the weekly/daily workload minimums (spec §6.2).**
  - Precondition: 6.A.1 active.
  - Action: Each week ensure all seven minimums are exercised and logged in that week's daily files: (1) ≥1 long-running command, (2) ≥1 failed-command explainer use, (3) ≥1 Cmd-K use, (4) ≥1 shell switch, (5) ≥1 agent run **per working day**, (6) ≥1 MCP server session **if MCP shipped in MVP-0**, (7) ≥1 simulated crash/recovery drill. Update `stage-a-summary.md` matrix weekly with the date each minimum was hit.
  - Exit criterion: by end of the 30-working-day window, `stage-a-summary.md` shows every weekly minimum satisfied for every week, and the agent-run-per-working-day row is satisfied for every working day. If MCP did not ship, the MCP row is explicitly marked `N/A — MCP not in MVP-0` (not silently skipped).
  - Records completion: `docs/dogfood/stage-a-summary.md` matrix fully populated.

- [ ] **6.A.3 Stage A exit gate.**
  - Precondition: 6.A.1–6.A.2 logs span 30 consecutive working days.
  - Action: Reconcile defect tracker and dogfood transcripts. Confirm the three hard exit conditions.
  - Exit criterion (all must hold, recorded in `stage-a-summary.md` "Exit verdict" section):
    1. **30 working days** of daily logs present (gaps explained as non-working days, not skipped working days).
    2. **Zero P0/P1** terminal-correctness defects open (cross-checked against §6.1 #27).
    3. **Zero confirmed secret leaks** across all dogfood transcripts/exports — backed by `docs/dogfood/secret-leak-audit.md`.
  - Records completion: `stage-a-summary.md` exit verdict = PASS; `secret-leak-audit.md` published; commit `docs(phase6/6.A.3): Stage A exit PASS`.

- [ ] **6.A.4 Secret-leak audit of dogfood corpus.**
  - Precondition: 6.A.1 logs exist; `cargo xtask secret-leak-corpus` available (Phase 4).
  - Action: Run the redaction/secret-leak detector over all Stage A transcripts, exports, and diagnostic bundles produced during dogfood (point the existing detector at the dogfood transcript store). Record any hits and their disposition.
  - Exit criterion: `docs/dogfood/secret-leak-audit.md` lists the scanned corpus scope, the tool + version, and concludes **zero confirmed leaks**; any candidate is shown to be a false positive or fixed and re-scanned to zero.
  - Records completion: `docs/dogfood/secret-leak-audit.md`.

### 6.B — Stage B: trusted-circle dogfood (maps orca 6.B.1–6.B.3)

- [ ] **6.B.0 Author Stage B recruiting + feedback plan.**
  - Files: `docs/dogfood/stage-b-recruiting.md`, `docs/dogfood/stage-b-summary.md` (skeleton).
  - Steps: Define target 3–5 recruits, channels (r/rust, r/PowerShell, r/commandline, ex-colleagues), the recruiting message draft, the 14-day window dates, the private feedback channel choice (Discord or Matrix) with invite mechanics, and the per-user expectation (each completes ≥1 agent workflow). Define the aggregation/triage format in `stage-b-summary.md`.
  - Exit criterion: both files exist; recruiting message draft present; feedback-channel choice decided and written; 14-day window has concrete start/end placeholders to be filled at kickoff.
  - Records completion: the files; commit `docs(phase6/6.B.0): Stage B plan`.

- [ ] **6.B.1 Recruit 3–5 Stage B users.**
  - Precondition: 6.A.3 PASS (do not expose external users before solo exit); 6.B.0 plan written.
  - Action: Post recruiting message to the named channels; collect committed participants in `stage-b-recruiting.md`.
  - Exit criterion: **≥3 committed users** recorded (name/handle, channel, commitment date). Risk-fallback (spec R18): if **<2 committed by day 21** of the recruiting window, Stage B is downgraded to optional with a documented note and the experimental disclaimer is mandatory on the landing page — record this decision in `stage-b-recruiting.md`.
  - Records completion: `docs/dogfood/stage-b-recruiting.md` participant table.

- [ ] **6.B.2 Issue signed dev-channel MSIX + open private feedback channel.**
  - Precondition: 6.B.1 has ≥3 users; signed dev-channel MSIX produced (uses 6.D.2 packaging); SmartScreen warm-up (6.D.6) staged for these users.
  - Action: Distribute `BongTerm-0.1.0-mvp0-rc.X-x64.msix` (signed) + `INSTALL.md` to recruits via the private channel; open the Discord/Matrix channel; collect feedback for the 14-day window.
  - Exit criterion: every recruit has the signed MSIX + install instructions; feedback channel live with all recruits joined; window start date recorded.
  - Records completion: `stage-b-recruiting.md` distribution + channel-live confirmation.

- [ ] **6.B.3 Stage B exit: aggregate findings.**
  - Precondition: 14-day window elapsed.
  - Action: Triage all feedback; classify each finding as P0/P1/P2/enhancement; confirm each user completed ≥1 agent workflow; route Post-MVP requests to `known-issues.md` as `0.2.0+`.
  - Exit criterion (recorded in `stage-b-summary.md`): **no public-facing defect** open (no P0/P1 that a public user would hit on a supported path); each participating user's ≥1-agent-workflow completion confirmed; remaining issues are P2/enhancement or documented in `known-issues.md`.
  - Records completion: `docs/dogfood/stage-b-summary.md` verdict = PASS (or downgraded per 6.B.1 fallback with disclaimer).

### 6.C — Brand + legal readiness (maps orca 6.C.1–6.C.2)

- [ ] **6.C.1 Trademark + namespace search.**
  - Files: `docs/adr/0009-trademark-search.md`.
  - Steps: Search and record results for "BongTerm" and "BongT" across: **USPTO** (TESS/trademark search), **EUIPO** (eSearch), **Indian TM database** (ipindiaonline public search), and namespace availability on **GitHub** (org/repo), **npm**, **crates.io**, and **domain** (at minimum `.com`, `.dev`, `.app`). For each registry/namespace record: query date, exact query, result (clear / conflict / similar-mark note), and a link/reference.
  - Exit criterion: ADR-0009 has a row for each of the seven targets (USPTO, EUIPO, Indian TM, GitHub, npm, crates, domain) with a result and a verdict line: **no direct conflict across all registries/namespaces** OR an explicit conflict + mitigation (rename option preserved per spec R17).
  - Records completion: `docs/adr/0009-trademark-search.md` status Accepted; commit `docs(phase6/6.C.1): trademark + namespace search`.

- [ ] **6.C.2 Brand-perception review + finalize product-name ADR.**
  - Files: `docs/adr/0002-brand-perception-notes.md`, `docs/adr/0002-product-name.md`.
  - Steps: Write the "bong" connotation review across target geographies (US, EU, India) in `0002-brand-perception-notes.md`: connotation risk per market, friction assessment, mitigation (disclaimer, tagline framing) or rename trigger. Then finalize `0002-product-name.md`: flip status Pending→Accepted, fill Date/Deciders, write the Decision (keep "BongTerm" with rationale, or rename) referencing 6.C.1 (ADR-0009) and the perception notes, and write Consequences.
  - Exit criterion: ADR-0002 status = Accepted with a concrete name decision and rationale citing both the trademark search (6.C.1) and the perception notes; `0002-brand-perception-notes.md` covers all three markets. Per spec §6.6 this is "not a blocker by itself; documented decision required" — the documented decision must exist.
  - Records completion: both ADR files; commit `docs(phase6/6.C.2): finalize product-name decision`.

### 6.D — Release engineering, public flip, GitHub release (maps orca 6.D.1–6.D.5 + 6.exit)

#### 6.D.1 — Repo public-readiness (precondes the flip)

- [ ] **6.D.1.a Author public-flip readiness checklist + community docs.**
  - Files: `docs/runbook/public-flip-checklist.md`, `README.md` (public version), `CONTRIBUTING.md`, `CODE_OF_CONDUCT.md`, `LICENSE` + `LICENSE-APACHE` (Apache-2.0 text), `.github/ISSUE_TEMPLATE/bug_report.md`, `.github/ISSUE_TEMPLATE/feature_request.md`, `.github/ISSUE_TEMPLATE/config.yml`, `.github/PULL_REQUEST_TEMPLATE.md`, `PRIVACY.md`.
  - Steps: Write the public README (value prop per §19.3, status = "experimental MVP-0", install pointer to `INSTALL.md`, security pointer to `SECURITY.md`). Write `CONTRIBUTING.md` (build commands, test gates, agent-profile-PR policy per PRD §18.5: test fixtures required, security review for profiles launching external tools, supported vs community split). Add Apache-2.0 `LICENSE`/`LICENSE-APACHE`. Add issue templates routing security reports to `SECURITY.md` (no public issues for vulns) via `config.yml`. Add PR template with tests + security-review checkboxes. Write `PRIVACY.md` (telemetry off by default; what diagnostic export contains; redaction-preview promise). Write `public-flip-checklist.md` enumerating every pre-flip gate (below).
  - Exit criterion: all listed files exist; `public-flip-checklist.md` enumerates: README/CONTRIBUTING/LICENSE/CODE_OF_CONDUCT/issue+PR templates present, no committed secrets (history scan), `SECURITY.md` inbox real, all P0 gates green ×7 nightlies, ADR-0002 + ADR-0009 Accepted.
  - Records completion: files; commit `docs(phase6/6.D.1a): public-flip readiness docs`.

- [ ] **6.D.1.b Finalize SECURITY.md + PRIVACY.md inbox and SLA.**
  - Files: `SECURITY.md`, `PRIVACY.md`.
  - Steps: Replace `security@PLACEHOLDER-DOMAIN` with the real monitored disclosure address; state supported versions (`0.1.0-mvp0` support window), acknowledgment SLA (≤5 working days), and the no-public-issues rule. Confirm the inbox is created and a monitoring owner + check cadence is named.
  - Exit criterion: `SECURITY.md` contains a real address (no PLACEHOLDER token anywhere in the repo — verified by repo-wide search), a defined response SLA, and supported-versions table; monitoring cadence documented.
  - Records completion: `SECURITY.md`; the public-flip checklist's "SECURITY inbox real" item checked.

- [ ] **6.D.1.c Pre-flip secret/history scan.**
  - Precondition: 6.D.1.a/b done.
  - Action: Run a full-history secret scan (the existing `cargo xtask secret-leak-corpus` detector adapted to scan tracked files + `git log -p` content, or `gitleaks` if available) over the entire repo and its history.
  - Exit criterion: **zero** confirmed secrets in tracked files or history; result recorded as a checked item in `public-flip-checklist.md` with the tool + date.
  - Records completion: `public-flip-checklist.md` secret-scan item checked.

#### 6.D.2 — Release tooling (TDD) and packaging

- [ ] **6.D.2.a (TDD) `cargo xtask checksums`.**
  - Files: `tools/xtask/src/checksums.rs`, `tools/xtask/src/main.rs` (register subcommand), `tools/xtask/tests/release_verify_tests.rs` (new test), fixtures `tools/xtask/tests/fixtures/dist-good/`.
  - Steps (TDD): (1) Write a failing test `checksums_emit_and_match` that runs the subcommand over `dist-good/` and asserts a `<file>.sha256` per artifact + a combined `checksums.txt` whose lines match recomputed SHA-256. (2) Run `cargo test -p xtask checksums_emit_and_match` → fails. (3) Implement `checksums::run(dir)` (SHA-256 via `sha2`, deterministic ordering, `<hash>  <relpath>` format). (4) Register `Checksums { dir }` in `main.rs`. (5) Re-run test → passes. (6) `cargo fmt && cargo clippy -- -D warnings`.
  - Exit criterion: test green; `cargo clippy` clean.
  - Records completion: commit `feat(phase6/6.D.2a): xtask checksums (sha256 + checksums.txt)`.

- [ ] **6.D.2.b (TDD) `cargo xtask attestation`.**
  - Files: `tools/xtask/src/attestation.rs`, `main.rs`, test `attestation_emits_intoto_for_msix`.
  - Steps (TDD): (1) Failing test asserts `attestation.intoto.jsonl` is valid JSON-lines containing an in-toto statement with `_type`, `subject[].name`/`digest.sha256` referencing the MSIX, and a `predicateType` (SLSA provenance). (2) Run → fails. (3) Implement: read MSIX path + its SHA-256, emit the in-toto statement to `dist/attestation.intoto.jsonl`. (4) Register subcommand. (5) Re-run → passes; fmt+clippy.
  - Exit criterion: test green; emitted file parses as JSON-lines and references the MSIX digest.
  - Records completion: commit `feat(phase6/6.D.2b): xtask attestation (in-toto provenance)`.

- [ ] **6.D.2.c (TDD) `cargo xtask release-verify`.**
  - Files: `tools/xtask/src/release_verify.rs`, `main.rs`, tests (`release_verify_passes_on_good`, `_fails_missing_artifact`, `_fails_bad_checksum`, `_fails_secret_leak`), fixtures `dist-good/`, `dist-missing-artifact/`, `dist-bad-checksum/`, `dist-secret-leak/`.
  - Steps (TDD): (1) Write four failing tests over the fixtures asserting: pass on complete/valid `dist`; fail when any §6.4 artifact is missing; fail when a checksum mismatches; fail when an artifact contains a planted synthetic secret. (2) Run → fail. (3) Implement `release_verify::run(dir)` asserting: **all §6.4 artifacts present** (the 12-item set below), **`checksums.txt` matches** recomputed hashes, **`checksums.txt.sig` present and verifies** against the cert, **`attestation.intoto.jsonl` present** and references the MSIX digest, **`sbom.cdx.json` references the vendored WezTerm version**, and **no artifact contains a secret** (reuse the redaction detector). (4) Register subcommand. (5) Re-run → all pass; fmt+clippy.
  - §6.4 artifact set asserted: `BongTerm-0.1.0-mvp0-x64.msix`, `.msix.cer`, `.sha256`, `checksums.txt`, `checksums.txt.sig`, `attestation.intoto.jsonl`, `THIRD_PARTY_NOTICES.md`, `sbom.cdx.json`, `benchmark-report.md`, `CHANGELOG.md`, `known-issues.md`, `SECURITY.md`, `INSTALL.md`.
  - Exit criterion: all four tests green; clippy clean.
  - Records completion: commit `feat(phase6/6.D.2c): xtask release-verify (artifact/checksum/sig/sbom/secret gate)`.

- [ ] **6.D.2.d Produce signed MSIX + cert + benchmark report + SBOM + notices into `dist/`.**
  - Precondition: `cargo xtask doctor` `signtool.exe` + `cl.exe` checks now PASS (Phase 5 prerequisite); OV cert provisioned per `docs/runbook/code-signing.md`.
  - Action: Build release; run `cargo xtask package-msix` (signed); export the public cert to `BongTerm-0.1.0-mvp0-x64.msix.cer`; run `cargo xtask bench-report --gate` on the reference machine → `dist/benchmark-report.md`; run `cargo xtask sbom` → `dist/sbom.cdx.json`; regenerate + validate `THIRD_PARTY_NOTICES.md` via `cargo xtask check-licenses`; copy `CHANGELOG.md`, `known-issues.md`, `SECURITY.md`, `INSTALL.md` into `dist/`.
  - Exit criterion: `dist/` contains a **signed** MSIX (signature verifies via `signtool verify /pa`) and all twelve §6.4 artifacts; `cargo xtask check-licenses` green; bench report shows reference-hardware numbers meeting §6.1 budget gates.
  - Records completion: `dist/` assembled; checked in `docs/runbook/release.md` dry-run log.

- [ ] **6.D.2.e Sign checksums + run release-verify on real `dist/`.**
  - Precondition: 6.D.2.a–d done, `dist/` assembled.
  - Action: `cargo xtask checksums dist/` → `*.sha256` + `checksums.txt`; sign `checksums.txt` with the OV cert (`signtool sign /fd sha256 /a` or detached `.sig`) → `checksums.txt.sig`; `cargo xtask attestation` → `dist/attestation.intoto.jsonl`; then `cargo xtask release-verify dist/`.
  - Exit criterion: `cargo xtask release-verify dist/` exits 0 (all artifacts present, checksums match, signature valid, attestation references MSIX, SBOM references WezTerm, no secrets).
  - Records completion: release-verify pass logged in `docs/runbook/release.md`.

#### 6.D.3 — Finalize release docs

- [ ] **6.D.3.a Finalize CHANGELOG, known-issues, INSTALL, release runbook.**
  - Files: `CHANGELOG.md`, `known-issues.md`, `INSTALL.md`, `docs/runbook/release.md`.
  - Steps: Write the `v0.1.0-mvp0` `CHANGELOG.md` entry (MVP-0 feature summary, explicit out-of-scope per spec §6.5). Publish `known-issues.md` with each P1 §6.1 gate exception (rationale + timeline) and Stage-A/B-surfaced non-blocking issues + Post-MVP deferrals. Write `INSTALL.md` (MSIX install, `signtool verify`/`Get-FileHash` checksum verification, SmartScreen "Unknown publisher" click-through guidance). Replace the `release.md` placeholder with the executed end-to-end procedure + rollback/rollforward + the dry-run verification record.
  - Exit criterion: `known-issues.md` lists every P1 exception with rationale+timeline; `INSTALL.md` includes exact verification commands; `release.md` no longer says "Placeholder" and has a completed dry-run section.
  - Records completion: commit `docs(phase6/6.D.3a): finalize release docs`.

- [ ] **6.D.3.b SmartScreen warm-up execution log.**
  - Files: `docs/runbook/smartscreen.md`.
  - Precondition: signed MSIX distributed to Stage B (6.B.2).
  - Action: Execute the warm-up plan — record each Stage B install + SmartScreen prompt outcome; track block-rate; submit to Microsoft reputation service when the documented install threshold is reached.
  - Exit criterion: `smartscreen.md` has an "Executed" log section showing install/prompt outcomes; **majority of test installs show no warning post-warm-up** OR a documented residual-warning note in `INSTALL.md` (click-through guidance) if reputation not yet built.
  - Records completion: `docs/runbook/smartscreen.md` executed section.

#### 6.D.4 — Landing page (TDD where verifiable)

- [ ] **6.D.4.a (TDD) `cargo xtask site-check` + landing page.**
  - Files: `site/index.html`, `site/style.css`, `site/assets/`, `tools/xtask/src/site_check.rs`, `main.rs`, test `site_check_requires_value_prop_claims`.
  - Steps (TDD): (1) Write a failing test that runs `site-check site/` and asserts: HTML parses, no dead internal links, and **all §19.3 required claims present** — resource-governed agent terminal; no Electron in terminal hot path; no cloud account required; child-process resource dashboard; Cmd-K command generation; failed-command explanation; Claude/Codex support without bundling; privacy/local-first defaults; plus the **experimental disclaimer** (per spec §6.2 table). (2) Run → fails (no `site/` yet / no checker). (3) Implement `site/index.html` with the §19.3 copy + disclaimer and the checker (`scraper` for parse + link/claim assertions). (4) Register subcommand. (5) Re-run → passes; fmt+clippy.
  - Exit criterion: `cargo xtask site-check site/` exits 0; every §19.3 claim + experimental disclaimer present and brand-consistent with ADR-0002.
  - Records completion: commit `feat(phase6/6.D.4a): landing page + site-check`.

#### 6.D.5 — CI gate + nightly green-streak

- [ ] **6.D.5.a Wire 7-consecutive-nightly P0 green-streak tracker.**
  - Files: `.github/workflows/ci.yml` (nightly job), `.github/workflows/release.yml` (new).
  - Steps: Add/confirm a scheduled nightly job that runs all P0 §6.1 gate checks (shell-smoke, render-latency, startup, resource, idle, pane, blocks, agent, MCP, dashboard, narrator, signed-installer smoke, forbidden-abstraction, secret-leak corpus, prompt-injection corpus, crash-recovery, settings, storage-recovery, cargo-deny/notices/SBOM, forbidden-install). Add a green-streak tracker (job that fails the release gate unless the last 7 nightly runs are all green). Create `release.yml`: tag `v0.1.0-mvp0` → build → package-msix → sbom → checksums → sign-checksums → attestation → `release-verify` → create **draft** GitHub release with all §6.4 artifacts; release stays draft until manual publish.
  - Exit criterion: nightly job runs all 25 P0 gates; `release.yml` is gated on the 7-green-streak check and on `release-verify` passing; release is created as draft (not auto-published).
  - Records completion: commit `ci(phase6/6.D.5a): nightly P0 streak + tag-triggered release workflow`.

- [ ] **6.D.5.b Ship-when gate verification (spec §6.6).**
  - Precondition: 6.A.3 PASS, 6.B.3 PASS-or-disclaimer, 6.C.1+6.C.2 Accepted, 6.D.1–6.D.4 done.
  - Action: Walk the §6.6 ship-when checklist and tick each item against evidence.
  - Exit criterion (all true, recorded in `docs/runbook/release.md` ship-when section): all P0 §6.1 gates green ≥7 consecutive nightlies; any P1 exception in `known-issues.md` w/ rationale+timeline; Stage A complete + Stage B complete-or-disclaimer; trademark search complete (ADR-0009); brand-perception decision documented (ADR-0002); `docs/runbook/smartscreen.md` warm-up plan present+executed; SECURITY.md inbox monitored; release runbook executed end-to-end on clean release machine; cert tested on clean VM; install/uninstall tested on clean Windows profile; SECURITY.md supported-versions+intake; PRIVACY.md exists; known-issues.md published; rollback plan exists; release draft reviewed against checklist.
  - Records completion: `docs/runbook/release.md` ship-when checklist all ticked with evidence links.

#### 6.D.6 — Public flip + publish

- [ ] **6.D.6.a Repo public flip (maps orca 6.D.1).**
  - Precondition: `docs/runbook/public-flip-checklist.md` fully ticked (6.D.1.a–c), ship-when gate (6.D.5.b) PASS.
  - Action: Flip the GitHub repository visibility private→public (irreversible-by-default for crawlers — confirm intent first).
  - Exit criterion: repo is public with README/CONTRIBUTING/LICENSE/CODE_OF_CONDUCT/issue+PR templates visible; no secrets exposed (6.D.1.c was zero).
  - Records completion: `public-flip-checklist.md` flip item checked + date.

- [ ] **6.D.6.b Publish GitHub release `v0.1.0-mvp0` (maps orca 6.D.4) + landing page live (maps orca 6.D.5).**
  - Precondition: 6.D.6.a done; `release.yml` produced a draft release with `release-verify` green.
  - Action: Deploy `site/` to the landing host; publish the draft GitHub release `v0.1.0-mvp0` with the full §6.4 artifact set attached; verify each download's published SHA-256 matches `checksums.txt` and `checksums.txt.sig` verifies.
  - Exit criterion: public release `v0.1.0-mvp0` live with all twelve §6.4 artifacts attached, checksums match published files, signature verifies, landing page live and `site-check` green; SmartScreen guidance reachable from `INSTALL.md`.
  - Records completion: release URL recorded in `docs/runbook/release.md`.

### 6.exit — Phase 6 exit

- [ ] **6.exit `v0.1.0-mvp0` shipped.**
  - Exit criterion: public GitHub release `v0.1.0-mvp0` is published (not draft) with the verified artifact set; landing page live; ship-when checklist (6.D.5.b) fully satisfied; tag `v0.1.0-mvp0` pushed. Record final verdict in `docs/runbook/release.md`.
  - Records completion: tag `v0.1.0-mvp0`; this is the MVP-0 ship.

---

## Test and Gate Strategy

1. Tooling tasks (`6.D.2.*`, `6.D.4.a`) are TDD: failing test first, run to see red, implement, run to green, `cargo fmt` + `cargo clippy -- -D warnings`, commit.
2. The release is provable by one command: `cargo xtask release-verify dist/` must exit 0 (artifact completeness, checksum match, signature validity, attestation presence, SBOM↔WezTerm linkage, zero secrets).
3. Process gates (`6.A.*`, `6.B.*`, `6.C.*`, public-flip readiness) each have a measurable exit criterion recorded in a named artifact; no gate is "complete" without its artifact.
4. CI: all 25 P0 §6.1 gates green for 7 consecutive nightly runs is a hard precondition (6.D.5.a/b) for the release workflow and the public flip.
5. Definition of Done (PRD §34) applied to every tooling task; security review applied to `release-verify`, secret scans, SECURITY.md, and the landing page.

## Immediate Next Action

Start with **6.A.0** (Stage A protocol + log template), then begin **6.A.1** (BongTerm as default terminal + daily logging). Stage A's 30-working-day clock is the long pole — start it before the release-tooling tasks, and execute `6.D.2.*`/`6.D.4.a` tooling work in parallel during the dogfood window.

---

## Self-Review

**Every orca Phase 6 outline task maps to a task here:**

| orca task | This plan |
|---|---|
| 6.A.1 Begin Stage A; daily log `docs/dogfood/<date>.md` | 6.A.0 + 6.A.1 |
| 6.A.2 Stage A workload minimums (§6.2) | 6.A.2 |
| 6.A.3 Stage A exit (30 wd; zero P0/P1; zero secret leaks) | 6.A.3 + 6.A.4 |
| 6.B.1 Recruit 3–5 users / 14 days | 6.B.0 + 6.B.1 |
| 6.B.2 Signed dev-channel MSIX + private feedback channel | 6.B.2 |
| 6.B.3 Aggregate findings; no public-facing defect | 6.B.3 |
| 6.C.1 Trademark search (USPTO+EUIPO+Indian+GH/npm/crates/domain) | 6.C.1 |
| 6.C.2 Brand-perception review → ADR-0002 | 6.C.2 |
| 6.D.1 Repo public flip | 6.D.1.a–c + 6.D.6.a |
| 6.D.2 SmartScreen warm-up executed | 6.D.3.b |
| 6.D.3 SECURITY.md inbox monitored | 6.D.1.b |
| 6.D.4 GitHub release with full artifact set | 6.D.2.* + 6.D.3.a + 6.D.6.b |
| 6.D.5 Landing-page copy (§19.3) | 6.D.4.a + 6.D.6.b |
| 6.exit `v0.1.0-mvp0` shipped | 6.exit |

**§6.1 #22 (Dogfood gate):** covered by 6.A.* + 6.B.* with self-report log review evidence in `stage-a-summary.md` / `stage-b-summary.md`.

**§6.6 ship-when checklist → tasks (all 16 items covered, verified in 6.D.5.b):** P0 gates ×7 nightlies → 6.D.5.a/b; P1 exceptions in known-issues → 6.D.3.a; Stage A complete → 6.A.3; Stage B complete-or-disclaimer → 6.B.3/6.B.1; public repo flip → 6.D.6.a; trademark search → 6.C.1; brand-perception decision → 6.C.2; SmartScreen runbook → 6.D.3.b; SECURITY inbox monitored → 6.D.1.b; release runbook executed on clean machine → 6.D.2.d/e + 6.D.5.b; cert tested on clean VM → 6.D.5.b (carries Phase 5 clean-VM evidence); install/uninstall on clean profile → 6.D.5.b; SECURITY supported-versions+intake → 6.D.1.b; privacy notice → 6.D.1.a (`PRIVACY.md`); known-issues published → 6.D.3.a; rollback plan → 6.D.3.a (`release.md`); release draft reviewed → 6.D.5.a (draft) + 6.D.6.b.

**§6.4 release artifact set (all 12 covered):** MSIX(signed) + .cer + .sha256 + checksums.txt + checksums.txt.sig + attestation.intoto.jsonl + THIRD_PARTY_NOTICES.md + sbom.cdx.json + benchmark-report.md + CHANGELOG.md + known-issues.md + SECURITY.md + INSTALL.md — produced in 6.D.2.d/e, asserted present+valid by `cargo xtask release-verify` (6.D.2.c), attached in 6.D.6.b.

**Scope-creep scan:** no Post-MVP item (Markdown review, Command Lens, DB branching, durable session daemon, plugin marketplace, cross-platform) is built; dogfood/Stage-B Post-MVP requests are routed to `known-issues.md` as `0.2.0+` (6.B.3, scope lock 1).

**Placeholder scan:** all artifact paths are concrete; thresholds are concrete (≥3 Stage B users / <2-by-day-21 fallback; 30 working days; zero P0/P1; zero confirmed secret leaks; 7 consecutive nightlies; majority-of-installs no-warning); all twelve §6.4 artifacts and all seven §6.2 workload minimums and all seven trademark targets are enumerated; no "TBD" left as an action (the only deferred decision — EV cert post-`0.1.x` — is explicitly out of MVP-0 scope per spec §6.4).

**Security contract through release:** secret scans at 6.A.4 (dogfood corpus) and 6.D.1.c (repo history) and inside `release-verify` (artifacts); SECURITY.md inbox monitored (6.D.1.b); secrets never in artifacts/exports/logs (scope lock 2).
