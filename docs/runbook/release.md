# Release Runbook

**Status:** Phase 6 prep; release blocked by signed `dist/`, clean-VM proof, dogfood, legal/name, public flip, and GitHub release gates

## Release artifact checklist (spec §6.4)

GitHub release `v0.1.0-mvp0` must include:

- [ ] Signed MSIX (x64)
- [ ] Signing certificate (`.cer`, public key only)
- [ ] `checksums.txt`
- [ ] `checksums.txt.sig` (signature)
- [ ] `attestation.intoto.jsonl` (provenance)
- [ ] `THIRD_PARTY_NOTICES.md`
- [ ] `sbom.cdx.json` (CycloneDX)
- [ ] `benchmark-report.md`
- [ ] `CHANGELOG.md`
- [ ] `known-issues.md`
- [ ] `SECURITY.md`
- [ ] `INSTALL.md`

## Pre-release gate

All 25 P0 acceptance gates green for 7 consecutive scheduled nightly CI runs.
This gate is complete: scheduled `nightly.yml` runs `27411817353`
(2026-06-12), `27463710495` (2026-06-13), `27496013141`
(2026-06-14), `27549311099` (2026-06-15), `27616935145`
(2026-06-16), `27687120185` (2026-06-17), and `27755555379`
(2026-06-18) passed on `master`. Manual dispatch run `27343029777`
is excluded from the scheduled-only count.
P1 gates (6 total): exceptions documented in `known-issues.md`.

## Rollback plan

If a critical bug is found within 48 h of public release:
1. Yank the GitHub release (set to draft).
2. Publish a `known-issues.md` update in the release notes.
3. Cut a `v0.1.1-mvp0-hotfix` if the fix is under 2 h of work.
4. For larger issues: unpublish and revert to "coming soon" page until fixed.

## Dry-Run Commands

```powershell
cargo xtask package-msix
cargo xtask sbom
cargo xtask check-licenses
cargo xtask checksums dist
cargo xtask attestation --subject dist/BongTerm-0.1.0-mvp0-x64.msix --out dist/attestation.intoto.jsonl
cargo xtask release-verify dist
cargo xtask site-check site
```

`release-verify` checks artifact presence, checksum matching, non-empty checksum signature, attestation references the MSIX digest, SBOM references vendored WezTerm, and release artifacts contain no known secret pattern.

## Latest Local Preflight

2026-06-19:

- `cargo xtask doctor` — pass; found VS Build Tools `cl.exe`, Windows SDK, `signtool.exe`, and `makeappx.exe`.
- `cargo xtask package-msix` — pass; built `target/msix/BongTerm.msix` as a real unsigned MSIX via Windows SDK `makeappx`. Set `BONGT_SIGN_THUMBPRINT` to sign a release package.
- `cargo xtask sbom` — pass.
- `cargo xtask check-licenses` — pass; regenerated `THIRD_PARTY_NOTICES.md`.
- `cargo xtask site-check site` — pass.
- `cargo xtask release-verify dist` — fail as expected until real signed release artifacts are assembled under `dist/`.
- `cargo xtask bench-report --gate` — timed out after 184s; no benchmark gate pass claimed from this machine.

## Ship-When Checklist

| Gate | Status | Evidence |
| --- | --- | --- |
| Stage A complete | blocked | `docs/dogfood/stage-a-summary.md` |
| Stage B complete or experimental disclaimer accepted | blocked | `docs/dogfood/stage-b-summary.md` |
| P0 gates green for 7 consecutive nightlies | pass | Scheduled `nightly.yml` runs `27411817353`, `27463710495`, `27496013141`, `27549311099`, `27616935145`, `27687120185`, and `27755555379` passed. |
| Phase 5 clean-VM signed install smoke complete | blocked | External VM/signing proof required. |
| Trademark search accepted | pending | `docs/adr/0009-trademark-search.md` |
| Product-name ADR accepted | pending | `docs/adr/0002-product-name.md` |
| SECURITY inbox real and monitored | pass | GitHub private vulnerability reporting URL in `SECURITY.md`; owner/cadence recorded. |
| Release artifact set verified | pending | `cargo xtask release-verify dist` needs real signed `dist/`. |
| Landing page checked | pass | `cargo xtask site-check site` passed locally on 2026-06-19. |

## Steps

Phase 6 owns dogfood and public-release execution. This runbook owns rollback, artifact completeness, and final ship-when evidence.
