# Release Runbook

**Status:** Phase 5 release-prep runbook

## Release artifact checklist (spec §6.4)

GitHub release `v0.1.0-mvp0` must include:

- [ ] Signed MSIX (x64)
- [ ] Signing certificate (`.cer`, public key only)
- [ ] `sha256sums.txt`
- [ ] `sha256sums.txt.sig` (detached signature)
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

## Steps

Phase 6 owns dogfood and public-release execution. This runbook owns rollback and artifact completeness.
