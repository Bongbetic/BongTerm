# Release Runbook

**Status:** Phase 6 prep; release blocked by external proof and dogfood gates

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

All 25 P0 acceptance gates green for 7 consecutive nightly CI runs.  
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
cargo xtask attestation
cargo xtask release-verify dist
cargo xtask site-check site
```

`release-verify` checks artifact presence, checksum matching, non-empty checksum signature, attestation references the MSIX digest, SBOM references vendored WezTerm, and release artifacts contain no known secret pattern.

## Ship-When Checklist

| Gate | Status | Evidence |
| --- | --- | --- |
| Stage A complete | blocked | `docs/dogfood/stage-a-summary.md` |
| Stage B complete or experimental disclaimer accepted | blocked | `docs/dogfood/stage-b-summary.md` |
| P0 gates green for 7 consecutive nightlies | blocked | Remote CI proof required. |
| Phase 5 clean-VM signed install smoke complete | blocked | External VM/signing proof required. |
| Trademark search accepted | pending | `docs/adr/0009-trademark-search.md` |
| Product-name ADR accepted | pending | `docs/adr/0002-product-name.md` |
| SECURITY inbox real and monitored | pass | GitHub private vulnerability reporting URL in `SECURITY.md`; owner/cadence recorded. |
| Release artifact set verified | pending | `cargo xtask release-verify dist` needs real signed `dist/`. |
| Landing page checked | pass | `cargo xtask site-check site` passed locally on 2026-06-03. |

## Steps

Phase 6 owns dogfood and public-release execution. This runbook owns rollback, artifact completeness, and final ship-when evidence.
